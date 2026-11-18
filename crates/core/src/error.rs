//! Errores al cargar contratos de fuente.

use std::path::PathBuf;

#[derive(thiserror::Error, Debug)]
pub enum ErrorContrato {
    #[error("No se pudo leer {}: {fuente}", ruta.display())]
    Lectura {
        ruta: PathBuf,
        fuente: std::io::Error,
    },
    #[error("{origen}: el YAML no es válido: {mensaje}")]
    Yaml { origen: String, mensaje: String },
    #[error("{origen}: el contrato no es válido:\n- {}", detalles.join("\n- "))]
    Invalido {
        origen: String,
        detalles: Vec<String>,
    },
}

/// Errores del proceso Silver/Gold (Plan 4).
#[derive(thiserror::Error, Debug)]
pub enum ErrorProceso {
    #[error(transparent)]
    Contrato(#[from] ErrorContrato),
    #[error("Hay archivos con errores; no se procesó nada.")]
    Entradas(Vec<crate::informe::Informe>),
    /// Regla de proceso incumplida (por ejemplo, dos archivos del mismo periodo).
    #[error("{0}")]
    Datos(String),
    #[error("Error de DuckDB en {donde}: {detalle}")]
    Duckdb { donde: String, detalle: String },
    #[error("No se pudo escribir en {}: {fuente}", ruta.display())]
    Salida {
        ruta: PathBuf,
        fuente: std::io::Error,
    },
    /// TR-D7: la ejecución terminó y quedó en `historico/<marca>`, pero no se pudo reemplazar `actual/`.
    #[error(
        "No se pudo reemplazar {}: {fuente}. ¿Hay un archivo abierto (por ejemplo, en Excel) o una sincronización en curso? Esta ejecución quedó guardada en historico/{marca}; ciérrelos y vuelva a procesar.",
        ruta.display()
    )]
    Publicar {
        ruta: PathBuf,
        marca: String,
        fuente: std::io::Error,
    },
    #[error(
        "{} no es una carpeta de salida de etl-local (falta .etl-local); no se toca.",
        .0.display()
    )]
    CarpetaAjena(PathBuf),
    /// Plan 6: el usuario pulsó «Detener» antes de publicar.
    #[error("Se detuvo el proceso; no se cambió ningún reporte.")]
    Cancelado,
}

/// Errores del tablero (Plan 5, ADR-0004). `codigo` es el que ve la API.
#[derive(thiserror::Error, Debug)]
pub enum ErrorTablero {
    #[error("Todavía no hay resultados en {}. Procese los archivos primero.", .0.display())]
    SinDatos(PathBuf),
    #[error("{filtro}: «{valor}» no existe en los datos. Valores posibles: {}.", opciones.join(", "))]
    FiltroInvalido {
        filtro: String,
        valor: String,
        opciones: Vec<String>,
    },
    /// Plan 7: `actual/` se está reemplazando (falta un Parquet que la última ejecución procesó).
    #[error("Falta {} aunque la última ejecución procesó «{fuente}». Si se están actualizando los datos, espere un momento y vuelva a intentar.", ruta.display())]
    Actualizando { ruta: PathBuf, fuente: String },
    #[error(transparent)]
    Proceso(#[from] ErrorProceso),
    #[error(transparent)]
    Contrato(#[from] ErrorContrato),
}

impl ErrorTablero {
    pub fn codigo(&self) -> &'static str {
        match self {
            ErrorTablero::SinDatos(_) => "sin_datos",
            ErrorTablero::FiltroInvalido { .. } => "filtro_invalido",
            ErrorTablero::Actualizando { .. } => "actualizando",
            ErrorTablero::Proceso(_) | ErrorTablero::Contrato(_) => "interno",
        }
    }
}
