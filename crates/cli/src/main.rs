//! `etl-local`: valida, lee y procesa archivos fuente con los contratos de `config/fuentes/` (ADR-0003) y sirve el tablero (ADR-0004).
//!
//! Códigos de salida: 0 sin errores (puede haber avisos), 1 algún archivo con errores,
//! 2 problema de uso, de contrato, de configuración, de salida o fuente desconocida.

mod actualizar;
mod procesar;
mod salida;
mod tablero;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};
use etl_local_core::contrato::cargar_contratos;

#[derive(Parser)]
#[command(
    name = "etl-local",
    version,
    about = "Valida, lee y procesa archivos fuente (planilla, base de personal, licencias) y sirve el tablero"
)]
struct Cli {
    #[command(subcommand)]
    comando: Comando,
}

#[derive(Subcommand)]
enum Comando {
    /// Revisa nombre, hojas y cabeceras de los archivos, sin leer los datos.
    Validar(Opciones),
    /// Valida y lee los datos; informa filas, columnas y avisos de cada hoja.
    Leer(Opciones),
    /// Construye Silver y Gold en Parquet: `<salida>/actual/` y una copia en `historico/`.
    Procesar(procesar::OpcionesProcesar),
    /// Suma archivos nuevos a los guardados en `<salida>/entradas/` y reprocesa todo (lo que hace la app).
    Actualizar(actualizar::OpcionesActualizar),
    /// Levanta la API local del tablero (solo lectura) sobre `<salida>/actual/`.
    Tablero(tablero::OpcionesTablero),
}

#[derive(Args)]
struct Opciones {
    /// Fuente a la que pertenecen los archivos (planilla, base_personal, licencias…).
    #[arg(long)]
    fuente: String,
    /// Carpeta de contratos.
    #[arg(long, default_value = "config/fuentes")]
    config: PathBuf,
    /// Salida en JSON en lugar de texto.
    #[arg(long)]
    json: bool,
    /// Archivos a revisar.
    #[arg(required = true)]
    archivos: Vec<PathBuf>,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let (leer, opciones) = match cli.comando {
        Comando::Validar(o) => (false, o),
        Comando::Leer(o) => (true, o),
        Comando::Procesar(o) => return procesar::ejecutar(o),
        Comando::Actualizar(o) => return actualizar::ejecutar(o),
        Comando::Tablero(o) => return tablero::ejecutar(o),
    };
    let mut contratos = match cargar_contratos(&opciones.config) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    let Some(contrato) = contratos.remove(&opciones.fuente) else {
        let disponibles = contratos.keys().cloned().collect::<Vec<_>>().join(", ");
        eprintln!(
            "La fuente «{}» no existe en {}. Fuentes disponibles: {disponibles}.",
            opciones.fuente,
            opciones.config.display()
        );
        return ExitCode::from(2);
    };

    let resultados = salida::procesar(&contrato, &opciones.archivos, leer);
    let texto = if opciones.json {
        salida::json(&resultados, leer)
    } else {
        salida::humana(&resultados, leer)
    };
    println!("{texto}");
    if resultados.iter().any(|r| r.informe.tiene_errores()) {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}
