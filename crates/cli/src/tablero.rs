//! Comando `tablero`: levanta la API local de solo lectura sobre `<salida>/actual/` (ADR-0004).

use std::path::PathBuf;
use std::process::ExitCode;

use clap::Args;
use etl_local_core::tablero::Tablero;

#[derive(Args)]
pub struct OpcionesTablero {
    /// Carpeta de configuración (con `indicadores.yaml`, `parametros.yaml` y `sql/indicadores/`).
    #[arg(long, default_value = "config")]
    pub config: PathBuf,
    /// Carpeta de salida de `procesar` (la que contiene `actual/`).
    #[arg(long)]
    pub salida: PathBuf,
    /// Puerto en 127.0.0.1. Con 0, el sistema elige uno libre.
    #[arg(long, default_value_t = 8787)]
    pub puerto: u16,
    /// Carpeta con la interfaz construida (`npm --prefix app/ui run build`). Por defecto, `app/ui/dist` si
    /// existe, o `ui/` junto al ejecutable; sin ninguna, se sirve solo la API.
    #[arg(long)]
    pub ui: Option<PathBuf>,
}

/// La carpeta de la interfaz: la pedida, `app/ui/dist` o `<ejecutable>/ui`, si contiene `tablero.html`.
fn carpeta_ui(pedida: Option<PathBuf>) -> Option<PathBuf> {
    let junto_al_exe = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(|d| d.join("ui")));
    pedida
        .into_iter()
        .chain([PathBuf::from("app/ui/dist")])
        .chain(junto_al_exe)
        .find(|d| d.join("tablero.html").is_file())
}

pub fn ejecutar(o: OpcionesTablero) -> ExitCode {
    // Revisión final: con una configuración inválida no se levanta un servidor que solo respondería errores.
    let tablero = Tablero::nuevo(&o.config, &o.salida);
    if let Err(e) = tablero.validar_config() {
        eprintln!("{e}");
        return ExitCode::from(2);
    }
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("No se pudo iniciar el servidor: {e}");
            return ExitCode::from(2);
        }
    };
    runtime.block_on(async {
        // ADR-0004 §1: solo en 127.0.0.1, nunca en todas las interfaces.
        let listener = match tokio::net::TcpListener::bind(("127.0.0.1", o.puerto)).await {
            Ok(l) => l,
            Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
                eprintln!(
                    "El puerto {} está ocupado. Use --puerto con otro número.",
                    o.puerto
                );
                return ExitCode::from(2);
            }
            Err(e) => {
                // En Windows, un puerto reservado (Hyper-V/WSL) da «permiso denegado», no «ocupado».
                eprintln!(
                    "No se pudo abrir el puerto {}: {e}. Use --puerto con otro número.",
                    o.puerto
                );
                return ExitCode::from(2);
            }
        };
        let puerto = match listener.local_addr() {
            Ok(d) => d.port(),
            Err(e) => {
                eprintln!("No se pudo leer el puerto: {e}");
                return ExitCode::from(2);
            }
        };
        // Si todavía no hay actual/, la API responde «sin_datos» y basta procesar: no hace falta reiniciar.
        let ui = carpeta_ui(o.ui.clone());
        match &ui {
            Some(_) => println!("Tablero en http://127.0.0.1:{puerto}/ (Ctrl+C para salir)"),
            None => {
                println!("Tablero en http://127.0.0.1:{puerto}/api/v1/estado (Ctrl+C para salir)");
                println!("Falta la interfaz: npm --prefix app/ui run build");
            }
        }
        let apagado = async {
            let _ = tokio::signal::ctrl_c().await;
        };
        match etl_local_api::servir(listener, tablero, ui, apagado).await {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("El servidor se detuvo: {e}");
                ExitCode::from(2)
            }
        }
    })
}
