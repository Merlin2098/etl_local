//! Comandos que invoca la UI. Delgados: la lógica está en `servicio.rs` y en `crates/core`.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use etl_local_core::entradas::{actualizar, guardadas};
use etl_local_core::error::ErrorProceso;
use etl_local_core::proceso::{Avance, Entradas, preparar_salida};
use etl_local_core::reportes::{Reporte, cargar_reportes, estado_reportes};
use serde::Deserialize;
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State, UserAttentionType};
use tauri_plugin_opener::OpenerExt;

use crate::ajustes::{self, Ajustes};
use crate::error::ErrorApp;
use crate::servicio::{
    self, ArchivoElegido, Inicio, Resultado, RevisionArchivo, Rutas, TarjetaReporte,
};

pub struct Estado {
    pub rutas: Rutas,
    pub documentos: PathBuf,
    pub ajustes_ruta: PathBuf,
    pub cancelar: Arc<AtomicBool>,
    pub en_curso: AtomicBool,
    /// Archivos que la UI puede pedir abrir (los que se revisaron en esta sesión).
    pub revisados: Mutex<HashSet<PathBuf>>,
    /// Plan 7: dirección base del tablero (`http://127.0.0.1:<puerto>`; en debug, Vite).
    pub base_tablero: String,
}

impl Estado {
    fn reporte(&self, id: &str) -> Result<Reporte, ErrorApp> {
        cargar_reportes(&self.rutas.config)?
            .into_iter()
            .find(|r| r.id == id)
            .ok_or_else(|| ErrorApp::nuevo("interno", format!("No existe el reporte «{id}».")))
    }
}

fn rutas(v: &[String]) -> Vec<PathBuf> {
    v.iter().map(PathBuf::from).collect()
}

async fn en_segundo_plano<T: Send + 'static>(
    f: impl FnOnce() -> Result<T, ErrorApp> + Send + 'static,
) -> Result<T, ErrorApp> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| ErrorApp::nuevo("interno", e.to_string()))?
}

#[tauri::command]
pub fn inicio(estado: State<'_, Estado>) -> Result<Inicio, ErrorApp> {
    let hoy = chrono::Local::now().date_naive();
    let ajustes = ajustes::cargar(&estado.ajustes_ruta);
    let tarjetas = estado_reportes(&estado.rutas.config, &estado.rutas.salida, hoy)?
        .into_iter()
        .map(|e| TarjetaReporte {
            duracion_prevista_ms: ajustes.duraciones_ms.get(&e.reporte.id).copied(),
            estado: e,
        })
        .collect();
    Ok(Inicio {
        tarjetas,
        carpeta_visible: servicio::carpeta_visible(&estado.rutas.salida, &estado.documentos),
    })
}

#[tauri::command]
pub fn reconocer(
    estado: State<'_, Estado>,
    reporte: String,
    rutas: Vec<String>,
) -> Result<Vec<ArchivoElegido>, ErrorApp> {
    let r = estado.reporte(&reporte)?;
    servicio::reconocer(&estado.rutas.config, &r, &self::rutas(&rutas))
}

#[tauri::command]
pub fn complemento_guardado(
    estado: State<'_, Estado>,
    fuente: String,
) -> Result<Option<ArchivoElegido>, ErrorApp> {
    let g = guardadas(&estado.rutas.config, &estado.rutas.salida)?;
    let ruta = match fuente.as_str() {
        "licencias" => g.licencias,
        "base_personal" => g.base_personal,
        _ => None,
    };
    ruta.map(|r| servicio::reconocer_fuente(&estado.rutas.config, &fuente, &r))
        .transpose()
}

#[tauri::command]
pub async fn revisar(
    estado: State<'_, Estado>,
    reporte: String,
    rutas: Vec<String>,
    complementos: Vec<String>,
) -> Result<Vec<RevisionArchivo>, ErrorApp> {
    let r = estado.reporte(&reporte)?;
    let (principales, extra) = (self::rutas(&rutas), self::rutas(&complementos));
    if let Ok(mut revisados) = estado.revisados.lock() {
        // Solo Excel existentes: «revisar» no debe habilitar abrir cualquier ruta.
        revisados.extend(
            principales
                .iter()
                .chain(&extra)
                .filter(|r| servicio::abrible(r))
                .cloned(),
        );
    }
    let config = estado.rutas.config.clone();
    en_segundo_plano(move || servicio::revisar(&config, &r, &principales, &extra)).await
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn procesar(
    app: AppHandle,
    estado: State<'_, Estado>,
    reporte: String,
    rutas: Vec<String>,
    complementos: Vec<String>,
    excluir: Vec<String>,
    avance: Channel<Avance>,
) -> Result<Resultado, ErrorApp> {
    if estado.en_curso.swap(true, Ordering::SeqCst) {
        return Err(ErrorApp::nuevo("en_curso", "Ya hay un proceso en curso."));
    }
    let resultado =
        procesar_sin_guardia(&app, &estado, reporte, rutas, complementos, excluir, avance).await;
    estado.en_curso.store(false, Ordering::SeqCst);
    resultado
}

async fn procesar_sin_guardia(
    app: &AppHandle,
    estado: &State<'_, Estado>,
    reporte: String,
    rutas: Vec<String>,
    complementos: Vec<String>,
    excluir: Vec<String>,
    avance: Channel<Avance>,
) -> Result<Resultado, ErrorApp> {
    let r = estado.reporte(&reporte)?;
    let mut nuevas = Entradas::default();
    asignar(&mut nuevas, &r.fuente, self::rutas(&rutas));
    if let Some(fuente) = r.complementos.first() {
        asignar(&mut nuevas, fuente, self::rutas(&complementos));
    }
    estado.cancelar.store(false, Ordering::SeqCst);
    let cancelar = Arc::clone(&estado.cancelar);
    let (config, salida) = (estado.rutas.config.clone(), estado.rutas.salida.clone());
    let marca = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
    let inicio = Instant::now();
    let r2 = r.clone();
    let resultado = en_segundo_plano(move || {
        let ej = actualizar(
            &config,
            &salida,
            &nuevas,
            &excluir,
            &marca,
            &mut |a| {
                let _ = avance.send(a);
            },
            &cancelar,
        )?;
        servicio::resultado(&config, &salida, &r2, ej)
    })
    .await?;
    let mut a: Ajustes = ajustes::cargar(&estado.ajustes_ruta);
    a.duraciones_ms
        .insert(r.id.clone(), inicio.elapsed().as_millis() as u64);
    let _ = ajustes::guardar(&estado.ajustes_ruta, &a);
    if let Some(ventana) = app.get_webview_window("principal") {
        let _ = ventana.request_user_attention(Some(UserAttentionType::Informational));
    }
    Ok(resultado)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asignar_otras() {
        let mut e = Entradas::default();
        asignar(&mut e, "practicantes", vec![PathBuf::from("a.xlsx")]);
        asignar(&mut e, "altas_personal", vec![PathBuf::from("b.xlsx")]);
        asignar(&mut e, "altas_personal", vec![PathBuf::from("c.xlsx")]);
        assert_eq!(e.otras["practicantes"], [PathBuf::from("a.xlsx")]);
        assert_eq!(
            e.otras["altas_personal"],
            [PathBuf::from("b.xlsx"), PathBuf::from("c.xlsx")]
        );
        assert!(e.planillas.is_empty() && e.base_personal.is_none());
    }
}

fn asignar(e: &mut Entradas, fuente: &str, rutas: Vec<PathBuf>) {
    match fuente {
        "planilla" => e.planillas.extend(rutas),
        "licencias" => e.licencias = rutas.into_iter().last().or(e.licencias.take()),
        "base_personal" => e.base_personal = rutas.into_iter().last().or(e.base_personal.take()),
        // Plan 8: cualquier otra fuente es un reporte del molde.
        otra => e.otras.entry(otra.to_string()).or_default().extend(rutas),
    }
}

#[tauri::command]
pub fn detener(estado: State<'_, Estado>) {
    estado.cancelar.store(true, Ordering::SeqCst);
}

#[derive(Deserialize, Debug)]
#[serde(tag = "tipo", rename_all = "snake_case")]
pub enum Abrir {
    Carpeta,
    Excel { reporte: String },
    Archivo { ruta: String },
}

fn abrir_ruta(app: &AppHandle, ruta: &Path) -> Result<(), ErrorApp> {
    app.opener()
        .open_path(ruta.display().to_string(), None::<&str>)
        .map_err(|e| {
            ErrorApp::nuevo(
                "interno",
                format!("No se pudo abrir {}: {e}", ruta.display()),
            )
        })
}

#[tauri::command]
pub fn abrir(app: AppHandle, estado: State<'_, Estado>, que: Abrir) -> Result<(), ErrorApp> {
    match que {
        Abrir::Carpeta => {
            let salida = &estado.rutas.salida;
            // Se crea con su marcador para que un proceso posterior la acepte. Si es una carpeta ajena
            // que ya existía, se abre igual (mirarla no la cambia).
            match preparar_salida(salida) {
                Ok(()) | Err(ErrorProceso::CarpetaAjena(_)) => {}
                Err(e) => return Err(e.into()),
            }
            abrir_ruta(&app, salida)
        }
        Abrir::Excel { reporte } => {
            let r = estado.reporte(&reporte)?;
            let ruta = estado
                .rutas
                .salida
                .join("actual/excel")
                .join(format!("{}.xlsx", r.excel));
            if !ruta.is_file() {
                return Err(ErrorApp::nuevo(
                    "sin_excel",
                    "Todavía no hay un Excel de este reporte. Procésalo primero.",
                ));
            }
            abrir_ruta(&app, &ruta)
        }
        Abrir::Archivo { ruta } => {
            let ruta = PathBuf::from(ruta);
            let permitido = servicio::abrible(&ruta)
                && estado
                    .revisados
                    .lock()
                    .map(|r| r.contains(&ruta))
                    .unwrap_or(false);
            if !permitido {
                return Err(ErrorApp::nuevo(
                    "no_permitido",
                    "Solo se pueden abrir los archivos que se revisaron.",
                ));
            }
            abrir_ruta(&app, &ruta)
        }
    }
}

/// Plan 7: abre (o enfoca) la ventana del tablero, que apunta a la API local como en el navegador. Esa ventana
/// no tiene capacidades de Tauri. Asíncrono: crear ventanas en un comando síncrono traba Windows.
#[tauri::command]
pub async fn abrir_tablero(
    app: AppHandle,
    estado: State<'_, Estado>,
    pestana: Option<String>,
) -> Result<(), ErrorApp> {
    let pestana = pestana.filter(|p| p == "alertas" || p == "como-leer");
    let url = servicio::url_tablero(&estado.base_tablero, pestana.as_deref());
    let destino: tauri::Url = url.parse().map_err(|e| {
        ErrorApp::nuevo(
            "interno",
            format!("Dirección del tablero no válida ({url}): {e}"),
        )
    })?;
    let fallo =
        |e: tauri::Error| ErrorApp::nuevo("interno", format!("No se pudo abrir el tablero: {e}"));
    if let Some(ventana) = app.get_webview_window("tablero") {
        ventana.navigate(destino).map_err(fallo)?;
        let _ = ventana.unminimize();
        let _ = ventana.set_focus();
    } else {
        tauri::WebviewWindowBuilder::new(&app, "tablero", tauri::WebviewUrl::External(destino))
            .title("Tablero RR. HH.")
            .inner_size(1440.0, 900.0)
            .min_inner_size(960.0, 700.0)
            .build()
            .map_err(fallo)?;
    }
    Ok(())
}
