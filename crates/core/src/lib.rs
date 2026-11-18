//! Motor de `etl_local`: contratos de fuente (ADR-0003), validación previa y lectura de Excel.

pub mod almacen;
pub mod celda;
pub mod contrato;
mod convertir;
pub mod entradas;
pub mod error;
pub mod informe;
pub mod lectura;
mod libro;
pub mod normalizar;
pub mod parametros;
pub mod preflight;
pub mod proceso;
pub mod reportes;
pub mod revision;
pub mod tablero;
