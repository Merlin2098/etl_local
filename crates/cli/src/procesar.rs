//! Comando `procesar`: Silver y Gold de las fuentes entregadas, con salida en `<salida>/actual/`.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::Args;
use etl_local_core::error::ErrorProceso;
use etl_local_core::proceso::{Ejecucion, Entradas, EstadoEtapa, procesar};

use crate::salida::linea;

#[derive(Args)]
pub struct OpcionesProcesar {
    /// Carpeta de configuración (con `fuentes/`, `sql/` y `parametros.yaml`).
    #[arg(long, default_value = "config")]
    pub config: PathBuf,
    /// Carpeta de salida; los resultados quedan en `<salida>/actual/` y una copia en `historico/`.
    #[arg(long)]
    pub salida: PathBuf,
    /// Planillas mensuales (una por periodo).
    #[arg(long, num_args = 1..)]
    pub planilla: Vec<PathBuf>,
    /// Control de licencias.
    #[arg(long)]
    pub licencias: Option<PathBuf>,
    /// Base de personal.
    #[arg(long)]
    pub base_personal: Option<PathBuf>,
    /// Salida en JSON en lugar de texto.
    #[arg(long)]
    pub json: bool,
}

fn estado(e: EstadoEtapa) -> &'static str {
    match e {
        EstadoEtapa::Ok => "ok",
        EstadoEtapa::Aviso => "con avisos",
        EstadoEtapa::Fallo => "falló",
        EstadoEtapa::Omitida => "omitida",
    }
}

fn humana(ej: &Ejecucion, salida: &Path) -> String {
    let mut lineas = Vec::new();
    for e in &ej.etapas {
        let filas = e.filas.map(|f| format!(" ({f} filas)")).unwrap_or_default();
        lineas.push(format!("{}: {}{filas}", e.nombre, estado(e.estado)));
    }
    lineas.extend(ej.problemas.iter().map(linea));
    lineas.push(format!("Resultados en {}", salida.join("actual").display()));
    lineas.join("\n")
}

pub fn ejecutar(o: OpcionesProcesar) -> ExitCode {
    let entradas = Entradas {
        planillas: o.planilla,
        licencias: o.licencias,
        base_personal: o.base_personal,
        otras: Default::default(),
    };
    let marca = marca();
    informar(
        procesar(&o.config, &entradas, &o.salida, &marca),
        &o.salida,
        o.json,
    )
}

/// Único uso del reloj: la marca de la ejecución (AGENTS.md §4.5; el núcleo la recibe).
pub(crate) fn marca() -> String {
    chrono::Local::now().format("%Y%m%d-%H%M%S").to_string()
}

/// Imprime el resultado de `procesar` o `actualizar` y devuelve el código de salida.
pub(crate) fn informar(
    resultado: Result<Ejecucion, ErrorProceso>,
    salida: &Path,
    json: bool,
) -> ExitCode {
    match resultado {
        Ok(ej) => {
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&ej).expect("Ejecucion es serializable")
                );
            } else {
                println!("{}", humana(&ej, salida));
            }
            ExitCode::SUCCESS
        }
        Err(ErrorProceso::Entradas(informes)) => {
            for i in &informes {
                for p in &i.problemas {
                    println!("{}", linea(p));
                }
            }
            println!("Hay archivos con errores; no se procesó nada.");
            ExitCode::from(1)
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(2)
        }
    }
}
