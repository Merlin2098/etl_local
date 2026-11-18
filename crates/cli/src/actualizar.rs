//! Comando `actualizar` (Plan 6): suma los archivos nuevos a los guardados en `<salida>/entradas/` y
//! reprocesa todo. Es lo que hace la app de escritorio, sin interfaz.

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::atomic::AtomicBool;

use clap::Args;
use etl_local_core::entradas::actualizar;
use etl_local_core::proceso::Entradas;

use crate::procesar::{informar, marca};

#[derive(Args)]
pub struct OpcionesActualizar {
    /// Carpeta de configuración (con `fuentes/`, `sql/` y `parametros.yaml`).
    #[arg(long, default_value = "config")]
    pub config: PathBuf,
    /// Carpeta de salida; guarda las entradas en `<salida>/entradas/` y los resultados en `actual/`.
    #[arg(long)]
    pub salida: PathBuf,
    /// Planillas nuevas (reemplazan a las guardadas del mismo periodo).
    #[arg(long, num_args = 1..)]
    pub planilla: Vec<PathBuf>,
    /// Control de licencias nuevo.
    #[arg(long)]
    pub licencias: Option<PathBuf>,
    /// Base de personal nueva.
    #[arg(long)]
    pub base_personal: Option<PathBuf>,
    /// Fuentes que no se usan en esta ejecución (sus archivos guardados se conservan).
    #[arg(long, num_args = 1..)]
    pub sin: Vec<String>,
    /// Salida en JSON en lugar de texto.
    #[arg(long)]
    pub json: bool,
}

pub fn ejecutar(o: OpcionesActualizar) -> ExitCode {
    let nuevas = Entradas {
        planillas: o.planilla,
        licencias: o.licencias,
        base_personal: o.base_personal,
        otras: Default::default(),
    };
    let resultado = actualizar(
        &o.config,
        &o.salida,
        &nuevas,
        &o.sin,
        &marca(),
        &mut |_| {},
        &AtomicBool::new(false),
    );
    informar(resultado, &o.salida, o.json)
}
