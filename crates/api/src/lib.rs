//! API local de solo lectura para el tablero (ADR-0004).
//!
//! Capa HTTP delgada sobre `etl_local_core::tablero`: rutas `/api/v1/…`, errores JSON y verificación del
//! encabezado `Host`. Cada pedido corre el núcleo (DuckDB, síncrono) en `spawn_blocking`.

use axum::Router;
use axum::extract::rejection::{PathRejection, QueryRejection};
use axum::extract::{Path, Query, Request, State};
use std::path::PathBuf;
use std::sync::Arc;

use axum::http::{HeaderValue, Method, StatusCode, Uri, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Json, Response};
use axum::routing::get;
use etl_local_core::error::ErrorTablero;
use etl_local_core::tablero::respuesta::ErrorApi;
use etl_local_core::tablero::{Detalle, Filtros, Tablero};
use serde::Serialize;

fn error(
    estado: StatusCode,
    codigo: &'static str,
    mensaje: impl Into<String>,
    opciones: Option<Vec<String>>,
) -> Response {
    // Cuerpo de todo error: `{ "codigo", "mensaje", "opciones"? }` (tipo compartido con el esquema).
    let cuerpo = ErrorApi {
        codigo: codigo.into(),
        mensaje: mensaje.into(),
        opciones,
    };
    (estado, Json(cuerpo)).into_response()
}

fn error_tablero(e: ErrorTablero) -> Response {
    let mensaje = e.to_string();
    match e {
        ErrorTablero::SinDatos(_) => error(StatusCode::NOT_FOUND, "sin_datos", mensaje, None),
        ErrorTablero::FiltroInvalido { opciones, .. } => error(
            StatusCode::BAD_REQUEST,
            "filtro_invalido",
            mensaje,
            Some(opciones),
        ),
        ErrorTablero::Actualizando { .. } => error(
            StatusCode::SERVICE_UNAVAILABLE,
            "actualizando",
            mensaje,
            None,
        ),
        otro => error(
            StatusCode::INTERNAL_SERVER_ERROR,
            otro.codigo(),
            mensaje,
            None,
        ),
    }
}

fn ruta_inexistente() -> Response {
    error(
        StatusCode::NOT_FOUND,
        "ruta_inexistente",
        "Esa dirección no existe en la API del tablero.",
        None,
    )
}

/// Corre una consulta del núcleo fuera del runtime asíncrono y la convierte en respuesta.
async fn ejecutar<T, F>(consulta: F) -> Response
where
    T: Serialize + Send + 'static,
    F: FnOnce() -> Result<T, ErrorTablero> + Send + 'static,
{
    match tokio::task::spawn_blocking(consulta).await {
        Ok(Ok(valor)) => Json(valor).into_response(),
        Ok(Err(e)) => error_tablero(e),
        Err(e) => error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "interno",
            e.to_string(),
            None,
        ),
    }
}

/// Plan 5, decisión 4: un parámetro desconocido es un filtro inválido, no se ignora.
fn filtros(q: Result<Query<Filtros>, QueryRejection>) -> Result<Filtros, Box<Response>> {
    q.map(|Query(f)| f).map_err(|e| {
        Box::new(error(
            StatusCode::BAD_REQUEST,
            "filtro_invalido",
            format!("Filtro no válido: {}", e.body_text()),
            None,
        ))
    })
}

async fn estado(State(t): State<Tablero>) -> Response {
    ejecutar(move || t.estado()).await
}

async fn opciones(State(t): State<Tablero>) -> Response {
    ejecutar(move || t.filtros()).await
}

async fn indicadores(
    State(t): State<Tablero>,
    q: Result<Query<Filtros>, QueryRejection>,
) -> Response {
    match filtros(q) {
        Ok(f) => ejecutar(move || t.indicadores(&f)).await,
        Err(r) => *r,
    }
}

async fn series(State(t): State<Tablero>, q: Result<Query<Filtros>, QueryRejection>) -> Response {
    match filtros(q) {
        Ok(f) => ejecutar(move || t.series(&f)).await,
        Err(r) => *r,
    }
}

async fn checks(State(t): State<Tablero>, q: Result<Query<Filtros>, QueryRejection>) -> Response {
    match filtros(q) {
        Ok(f) => ejecutar(move || t.checks(&f)).await,
        Err(r) => *r,
    }
}

/// TR-D18: la única ruta con nombres y documentos.
async fn detalle(
    State(t): State<Tablero>,
    tipo: Result<Path<Detalle>, PathRejection>,
    q: Result<Query<Filtros>, QueryRejection>,
) -> Response {
    let Ok(Path(tipo)) = tipo else {
        return ruta_inexistente();
    };
    match filtros(q) {
        Ok(f) => ejecutar(move || t.detalle(tipo, &f)).await,
        Err(r) => *r,
    }
}

/// ADR-0004 §1: solo `127.0.0.1:<puerto>` o `localhost:<puerto>`. Evita que una página ajena lea los
/// datos mediante DNS rebinding.
async fn verificar_host(State(puerto): State<u16>, req: Request, next: Next) -> Response {
    let host = req
        .headers()
        .get(header::HOST)
        .and_then(|h| h.to_str().ok())
        .unwrap_or_default();
    if host == format!("127.0.0.1:{puerto}") || host == format!("localhost:{puerto}") {
        next.run(req).await
    } else {
        error(
            StatusCode::FORBIDDEN,
            "host_no_permitido",
            format!("Abra el tablero en http://127.0.0.1:{puerto}/."),
            None,
        )
    }
}

fn rutas() -> Router<Tablero> {
    Router::new()
        .route("/api/v1/estado", get(estado))
        .route("/api/v1/filtros", get(opciones))
        .route("/api/v1/indicadores", get(indicadores))
        .route("/api/v1/series", get(series))
        .route("/api/v1/checks", get(checks))
        .route("/api/v1/detalle/{tipo}", get(detalle))
}

/// Router de la API. `puerto` es el que se acepta en el encabezado `Host`.
pub fn router(tablero: Tablero, puerto: u16) -> Router {
    rutas()
        .fallback(|| async { ruta_inexistente() })
        .with_state(tablero)
        .layer(middleware::from_fn_with_state(puerto, verificar_host))
}

/// Plan 7: la API más los archivos del tablero (`app/ui/dist`, sin `app.html`) en la misma dirección.
/// Sin `ui`, la raíz explica cómo construir la interfaz.
pub fn aplicacion(tablero: Tablero, puerto: u16, ui: Option<PathBuf>) -> Router {
    let ui = ui.map(Arc::new);
    rutas()
        .fallback(move |metodo: Method, uri: Uri| estatico(ui.clone(), metodo, uri))
        .with_state(tablero)
        .layer(middleware::from_fn_with_state(puerto, verificar_host))
}

/// Encabezados de seguridad de las páginas del tablero (la CSP de la app no aplica fuera de Tauri).
const CSP: &str = "default-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self'; connect-src 'self'; frame-ancestors 'none'";

fn no_encontrado() -> Response {
    error(
        StatusCode::NOT_FOUND,
        "no_encontrado",
        "Esa página no existe.",
        None,
    )
}

/// Decodifica `%xx` (sin dependencias). `None` si la secuencia no es válida o no es UTF-8.
fn decodificar(ruta: &str) -> Option<String> {
    let b = ruta.as_bytes();
    let mut salida = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            let hex = std::str::from_utf8(b.get(i + 1..i + 3)?).ok()?;
            salida.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            salida.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8(salida).ok()
}

/// Ruta relativa segura dentro de `ui`: sin `..`, `.`, segmentos vacíos, `\`, `:` ni `app.html`.
fn ruta_segura(ruta: &str) -> Option<PathBuf> {
    let texto = decodificar(ruta)?;
    let texto = texto.strip_prefix('/').unwrap_or(&texto);
    let texto = if texto.is_empty() {
        "tablero.html"
    } else {
        texto
    };
    if texto.contains(['\\', ':', '\0']) || texto == "app.html" {
        return None;
    }
    let mut rel = PathBuf::new();
    for segmento in texto.split('/') {
        if segmento.is_empty() || segmento == "." || segmento == ".." {
            return None;
        }
        rel.push(segmento);
    }
    Some(rel)
}

fn tipo_de(ruta: &std::path::Path) -> &'static str {
    match ruta
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
    {
        "html" => "text/html; charset=utf-8",
        "js" => "text/javascript",
        "css" => "text/css",
        "woff2" => "font/woff2",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "ico" => "image/x-icon",
        "json" => "application/json",
        _ => "application/octet-stream",
    }
}

async fn estatico(ui: Option<Arc<PathBuf>>, metodo: Method, uri: Uri) -> Response {
    let ruta = uri.path();
    if ruta == "/api" || ruta.starts_with("/api/") {
        return ruta_inexistente();
    }
    let Some(ui) = ui else {
        return error(
            StatusCode::NOT_FOUND,
            "sin_interfaz",
            "Falta la interfaz del tablero. Constrúyala con: npm --prefix app/ui run build",
            None,
        );
    };
    if metodo != Method::GET && metodo != Method::HEAD {
        return no_encontrado();
    }
    let Some(rel) = ruta_segura(ruta) else {
        return no_encontrado();
    };
    let (Ok(base), Ok(archivo)) = (
        std::fs::canonicalize(ui.as_path()),
        std::fs::canonicalize(ui.join(&rel)),
    ) else {
        return no_encontrado();
    };
    if !archivo.starts_with(&base) || !archivo.is_file() {
        return no_encontrado();
    }
    let leido = tokio::task::spawn_blocking({
        let archivo = archivo.clone();
        move || std::fs::read(archivo)
    })
    .await;
    let Ok(Ok(bytes)) = leido else {
        return no_encontrado();
    };
    let tipo = tipo_de(&archivo);
    let mut resp = (StatusCode::OK, bytes).into_response();
    let h = resp.headers_mut();
    h.insert(header::CONTENT_TYPE, HeaderValue::from_static(tipo));
    h.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(CSP),
    );
    h.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    if tipo.starts_with("text/html") {
        h.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    }
    resp
}

/// Sirve la API (y el tablero, con `ui`) en `listener` hasta que termine `apagado`. El puerto aceptado en
/// `Host` es el del listener. La CLI pasa Ctrl+C; la app de escritorio, un futuro que nunca termina.
pub async fn servir(
    listener: tokio::net::TcpListener,
    tablero: Tablero,
    ui: Option<PathBuf>,
    apagado: impl Future<Output = ()> + Send + 'static,
) -> std::io::Result<()> {
    let puerto = listener.local_addr()?.port();
    axum::serve(listener, aplicacion(tablero, puerto, ui))
        .with_graceful_shutdown(apagado)
        .await
}
