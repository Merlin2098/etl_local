// Sin consola extra en Windows en la versión empaquetada.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::collections::HashSet;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use etl_local_app::comandos::{self, Estado};
use etl_local_app::{ajustes, servicio};
use etl_local_core::tablero::Tablero;
use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let rutas_app = app.path();
            let ajustes_ruta = rutas_app.app_config_dir()?.join("ajustes.json");
            let documentos = rutas_app.document_dir()?;
            let recursos = rutas_app.resource_dir()?;
            let a = ajustes::cargar(&ajustes_ruta);
            let entorno = |v: &str| std::env::var_os(v).map(Into::into);
            let rutas = servicio::resolver_rutas(
                entorno("ETL_LOCAL_CONFIG"),
                entorno("ETL_LOCAL_SALIDA"),
                &recursos,
                &documentos,
                &a,
            );
            // Plan 7: la API del tablero corre dentro de la app, solo en 127.0.0.1.
            let debug = cfg!(debug_assertions);
            let pedido = servicio::puerto_api(debug, std::env::var("ETL_LOCAL_PUERTO").ok());
            let escucha = match std::net::TcpListener::bind(("127.0.0.1", pedido)) {
                Ok(l) => l,
                Err(e) if debug => {
                    eprintln!("El puerto {pedido} está ocupado ({e}); el tablero usará otro.");
                    std::net::TcpListener::bind(("127.0.0.1", 0))?
                }
                Err(e) => return Err(e.into()),
            };
            escucha.set_nonblocking(true)?;
            let puerto = escucha.local_addr()?.port();
            let tablero = Tablero::nuevo(&rutas.config, &rutas.salida);
            let ui = (!debug).then(|| recursos.join("ui"));
            tauri::async_runtime::spawn(async move {
                let resultado = match tokio::net::TcpListener::from_std(escucha) {
                    Ok(l) => etl_local_api::servir(l, tablero, ui, std::future::pending()).await,
                    Err(e) => Err(e),
                };
                if let Err(e) = resultado {
                    eprintln!("El tablero se detuvo: {e}");
                }
            });
            // En debug, Vite sirve la interfaz y reenvía /api al puerto fijo.
            let base_tablero = if debug && puerto == pedido {
                "http://localhost:5173".to_string()
            } else {
                format!("http://127.0.0.1:{puerto}")
            };
            app.manage(Estado {
                base_tablero,
                rutas,
                documentos,
                ajustes_ruta,
                cancelar: Arc::new(AtomicBool::new(false)),
                en_curso: AtomicBool::new(false),
                revisados: Mutex::new(HashSet::new()),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            comandos::inicio,
            comandos::reconocer,
            comandos::complemento_guardado,
            comandos::revisar,
            comandos::procesar,
            comandos::detener,
            comandos::abrir,
            comandos::abrir_tablero,
        ])
        .run(tauri::generate_context!())
        .expect("no se pudo iniciar la aplicación");
}
