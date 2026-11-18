//! API del tablero (ADR-0004 §3) sobre los fixtures demo `limpio`.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use etl_local_api::router;
use etl_local_core::proceso::{Entradas, procesar};
use etl_local_core::tablero::Tablero;
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

const CONFIG: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../config");
const PUERTO: u16 = 8787;

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../core/tests/fixtures/demo/limpio")
}

fn xlsx(carpeta: &Path) -> Vec<PathBuf> {
    let mut v: Vec<_> = std::fs::read_dir(carpeta)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "xlsx"))
        .collect();
    v.sort();
    v
}

/// Procesa `limpio` una sola vez por binario de test.
fn salida() -> &'static Path {
    static DIR: OnceLock<tempfile::TempDir> = OnceLock::new();
    DIR.get_or_init(|| {
        let dir = tempfile::tempdir().unwrap();
        let f = fixture();
        let entradas = Entradas {
            planillas: xlsx(&f.join("planilla")),
            licencias: Some(f.join("planilla/licencias/CONTROL DE LICENCIAS.xlsx")),
            base_personal: Some(xlsx(&f.join("base_personal")).remove(0)),
            otras: Default::default(),
        };
        procesar(Path::new(CONFIG), &entradas, dir.path(), "m1").unwrap();
        dir
    })
    .path()
}

fn app(salida: &Path) -> Router {
    router(Tablero::nuevo(Path::new(CONFIG), salida), PUERTO)
}

async fn pedir(app: Router, uri: &str, host: Option<&str>) -> (StatusCode, Value) {
    let mut req = Request::get(uri);
    if let Some(h) = host {
        req = req.header(header::HOST, h);
    }
    let resp = app.oneshot(req.body(Body::empty()).unwrap()).await.unwrap();
    let estado = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json = serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| panic!("{uri}: no es JSON: {}", String::from_utf8_lossy(&bytes)));
    (estado, json)
}

async fn get(uri: &str) -> (StatusCode, Value) {
    pedir(app(salida()), uri, Some("127.0.0.1:8787")).await
}

#[tokio::test]
async fn estado_ok() {
    let (s, j) = get("/api/v1/estado").await;
    assert_eq!(s, StatusCode::OK, "{j}");
    assert_eq!(j["fecha_corte"], "2025-11-12");
}

#[tokio::test]
async fn filtros_y_series_ok() {
    let (s, j) = get("/api/v1/filtros").await;
    assert_eq!(s, StatusCode::OK, "{j}");
    assert_eq!(j["periodos"].as_array().unwrap().len(), 3);
    let (s, j) = get("/api/v1/series").await;
    assert_eq!(s, StatusCode::OK, "{j}");
    assert_eq!(j["series"].as_array().unwrap().len(), 6);
    let (s, j) = get("/api/v1/checks?periodo=2025-11").await;
    assert_eq!(s, StatusCode::OK, "{j}");
    assert_eq!(j["checks"][0]["id"], "IND-C1");
}

#[tokio::test]
async fn indicadores_con_filtro() {
    let (s, j) = get("/api/v1/indicadores?periodo=2025-10").await;
    assert_eq!(s, StatusCode::OK, "{j}");
    assert_eq!(j["periodo"], "2025-10");
    assert_eq!(j["indicadores"].as_array().unwrap().len(), 8);
}

#[tokio::test]
async fn parametro_desconocido_400() {
    // Revisión 2: un parámetro mal escrito no se ignora en silencio.
    let (s, j) = get("/api/v1/indicadores?gerencias=X").await;
    assert_eq!(s, StatusCode::BAD_REQUEST, "{j}");
    assert_eq!(j["codigo"], "filtro_invalido");
    assert!(j["mensaje"].as_str().unwrap().contains("gerencias"), "{j}");
}

#[tokio::test]
async fn filtro_inyeccion_400() {
    let (s, j) = get("/api/v1/indicadores?gerencia=x%27%20OR%20%271%27%3D%271").await;
    assert_eq!(s, StatusCode::BAD_REQUEST, "{j}");
    assert_eq!(j["codigo"], "filtro_invalido");
    assert!(!j["opciones"].as_array().unwrap().is_empty(), "{j}");
}

#[tokio::test]
async fn detalle_sin_base() {
    let (s, j) = get("/api/v1/detalle/sin-base?periodo=2025-11").await;
    assert_eq!(s, StatusCode::OK, "{j}");
    assert_eq!(j["filas"].as_array().unwrap().len(), 8);
    let (s, j) = get("/api/v1/detalle/alertas").await;
    assert_eq!(s, StatusCode::OK, "{j}");
    let (s, j) = get("/api/v1/detalle/otro").await;
    assert_eq!(s, StatusCode::NOT_FOUND, "{j}");
    assert_eq!(j["codigo"], "ruta_inexistente");
}

#[tokio::test]
async fn host_ajeno_403() {
    // Revisión 5: otra página (DNS rebinding) u otro puerto no leen la API.
    for host in [Some("evil.example"), Some("127.0.0.1:9999"), None] {
        let (s, j) = pedir(app(salida()), "/api/v1/estado", host).await;
        assert_eq!(s, StatusCode::FORBIDDEN, "{host:?}: {j}");
        assert_eq!(j["codigo"], "host_no_permitido");
    }
    let (s, _) = pedir(app(salida()), "/api/v1/estado", Some("localhost:8787")).await;
    assert_eq!(s, StatusCode::OK);
}

#[tokio::test]
async fn sin_datos_404() {
    // Revisión 4: todavía no se procesó nada.
    let vacia = tempfile::tempdir().unwrap();
    for uri in ["/api/v1/estado", "/api/v1/indicadores"] {
        let (s, j) = pedir(app(vacia.path()), uri, Some("127.0.0.1:8787")).await;
        assert_eq!(s, StatusCode::NOT_FOUND, "{uri}: {j}");
        assert_eq!(j["codigo"], "sin_datos");
        assert!(j["mensaje"].as_str().unwrap().contains("Procese"), "{j}");
    }
}

#[tokio::test]
async fn ruta_inexistente_404() {
    let (s, j) = get("/api/v1/nada").await;
    assert_eq!(s, StatusCode::NOT_FOUND, "{j}");
    assert_eq!(j["codigo"], "ruta_inexistente");
}

#[tokio::test]
async fn indicadores_traen_por_gerencia() {
    let (estado, cuerpo) = get("/api/v1/indicadores").await;
    assert_eq!(estado, StatusCode::OK);
    let inds = cuerpo["indicadores"].as_array().unwrap();
    assert!(inds.iter().all(|i| i.get("por_gerencia").is_some()));
    let ind01 = inds.iter().find(|i| i["id"] == "IND-01").unwrap();
    assert!(!ind01["por_gerencia"].as_array().unwrap().is_empty());
}

// --- Plan 7, Tarea 3: la API sirve el tablero (archivos de app/ui/dist, sin app.html) ---

/// `raiz/ui/` con el tablero, un asset y `app.html`; `raiz/secreto.txt` queda fuera.
fn carpeta_ui() -> (tempfile::TempDir, PathBuf) {
    let raiz = tempfile::tempdir().unwrap();
    let ui = raiz.path().join("ui");
    std::fs::create_dir_all(ui.join("assets")).unwrap();
    std::fs::write(
        ui.join("tablero.html"),
        "<!doctype html><title>Tablero</title>",
    )
    .unwrap();
    std::fs::write(ui.join("assets/t.js"), "console.log(1)").unwrap();
    std::fs::write(ui.join("app.html"), "app").unwrap();
    std::fs::write(raiz.path().join("secreto.txt"), "NO-DEBE-SALIR").unwrap();
    (raiz, ui)
}

struct Respuesta {
    estado: StatusCode,
    tipo: String,
    csp: String,
    nosniff: String,
    cuerpo: String,
}

async fn estatico(ui: Option<PathBuf>, uri: &str, host: &str) -> Respuesta {
    let app = etl_local_api::aplicacion(Tablero::nuevo(Path::new(CONFIG), salida()), PUERTO, ui);
    let req = Request::get(uri)
        .header(header::HOST, host)
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let h = |n: &str| {
        resp.headers()
            .get(n)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
            .to_string()
    };
    let (tipo, csp, nosniff) = (
        h("content-type"),
        h("content-security-policy"),
        h("x-content-type-options"),
    );
    let estado = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    Respuesta {
        estado,
        tipo,
        csp,
        nosniff,
        cuerpo: String::from_utf8_lossy(&bytes).into_owned(),
    }
}

const HOST: &str = "127.0.0.1:8787";

#[tokio::test]
async fn raiz_sirve_el_tablero() {
    let (_r, ui) = carpeta_ui();
    let r = estatico(Some(ui), "/", HOST).await;
    assert_eq!(r.estado, StatusCode::OK);
    assert_eq!(r.tipo, "text/html; charset=utf-8");
    assert!(r.cuerpo.contains("<title>Tablero</title>"));
}

#[tokio::test]
async fn asset_con_su_tipo() {
    let (_r, ui) = carpeta_ui();
    let r = estatico(Some(ui), "/assets/t.js", HOST).await;
    assert_eq!(r.estado, StatusCode::OK);
    assert_eq!(r.tipo, "text/javascript");
}

#[tokio::test]
async fn no_sirve_app_html() {
    let (_r, ui) = carpeta_ui();
    assert_eq!(
        estatico(Some(ui), "/app.html", HOST).await.estado,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn no_sale_de_ui() {
    let (_r, ui) = carpeta_ui();
    for uri in [
        "/..%2fsecreto.txt",
        "/assets/..%5c..%5csecreto.txt",
        "/%2e%2e/secreto.txt",
        "/assets/../../secreto.txt",
    ] {
        let r = estatico(Some(ui.clone()), uri, HOST).await;
        assert_eq!(r.estado, StatusCode::NOT_FOUND, "{uri}");
        assert!(!r.cuerpo.contains("NO-DEBE-SALIR"), "{uri}");
    }
}

#[tokio::test]
async fn html_con_csp() {
    let (_r, ui) = carpeta_ui();
    let r = estatico(Some(ui), "/", HOST).await;
    assert_eq!(
        r.csp,
        "default-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self'; connect-src 'self'; frame-ancestors 'none'"
    );
    assert_eq!(r.nosniff, "nosniff");
}

#[tokio::test]
async fn sin_ui_la_raiz_explica() {
    let r = estatico(None, "/", HOST).await;
    assert_eq!(r.estado, StatusCode::NOT_FOUND);
    let j: Value = serde_json::from_str(&r.cuerpo).unwrap();
    assert_eq!(j["codigo"], "sin_interfaz");
    assert!(
        j["mensaje"]
            .as_str()
            .unwrap()
            .contains("npm --prefix app/ui run build")
    );
}

#[tokio::test]
async fn host_ajeno_tambien_en_estaticos() {
    let (_r, ui) = carpeta_ui();
    assert_eq!(
        estatico(Some(ui), "/", "evil.example").await.estado,
        StatusCode::FORBIDDEN
    );
}

#[tokio::test]
async fn api_sigue_igual_con_ui() {
    let (_r, ui) = carpeta_ui();
    let r = estatico(Some(ui), "/api/v1/nada", HOST).await;
    assert_eq!(r.estado, StatusCode::NOT_FOUND);
    assert!(r.cuerpo.contains("ruta_inexistente"));
}

/// Plan 7 (foco 5): mientras `procesar` reemplaza `actual/`, la API pide reintentar con un código propio.
#[tokio::test]
async fn actual_a_medio_reemplazar_pide_reintentar() {
    let dir = tempfile::tempdir().unwrap();
    let f = fixture();
    let entradas = Entradas {
        planillas: xlsx(&f.join("planilla")),
        licencias: None,
        base_personal: None,
        otras: Default::default(),
    };
    procesar(Path::new(CONFIG), &entradas, dir.path(), "m1").unwrap();
    std::fs::remove_file(dir.path().join("actual/gold/planilla.parquet")).unwrap();
    let (s, j) = pedir(app(dir.path()), "/api/v1/indicadores", Some(HOST)).await;
    assert_eq!(s, StatusCode::SERVICE_UNAVAILABLE, "{j}");
    assert_eq!(j["codigo"], "actualizando");
}
