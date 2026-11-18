//! Respuestas del tablero (ADR-0004 §3): lo que la API entrega como JSON.
//!
//! Ninguna de estas estructuras lleva nombres ni números de documento, salvo `RespuestaDetalle` (TR-D18).

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use super::Filtros;

/// Sentido de "mejor" de un indicador (indicadores-tablero §4).
#[derive(schemars::JsonSchema, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MejorSi {
    Baja,
    Sube,
    Neutro,
}

/// Comparación con el mes anterior, en palabras (nunca solo el color, §4).
#[derive(schemars::JsonSchema, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Comparacion {
    Mejor,
    Peor,
    Igual,
}

/// Una parte del desglose de un indicador.
#[derive(schemars::JsonSchema, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Parte {
    pub categoria: String,
    pub etiqueta: String,
    pub valor: f64,
}

/// De dónde sale un número (TR-D17).
#[derive(schemars::JsonSchema, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Origen {
    pub fuente: String,
    pub archivos: Vec<String>,
    pub fecha_corte: Option<NaiveDate>,
    pub periodo: Option<String>,
    pub reglas: Vec<String>,
}

#[derive(schemars::JsonSchema, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Indicador {
    pub id: String,
    pub nombre: String,
    pub pregunta: String,
    /// `personas`, `fraccion` (0.0073 = 0,73 %) u `horas`.
    pub unidad: String,
    pub valor: Option<f64>,
    pub anterior: Option<f64>,
    pub variacion: Option<f64>,
    pub mejor_si: MejorSi,
    pub comparacion: Option<Comparacion>,
    pub meta: Option<f64>,
    pub desglose: Vec<Parte>,
    /// Plan 7: personas por gerencia (IND-01, IND-07, IND-08), de mayor a menor; vacío en el resto.
    pub por_gerencia: Vec<Parte>,
    pub origen: Origen,
    pub nota: Option<String>,
}

#[derive(schemars::JsonSchema, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct RespuestaIndicadores {
    pub periodo: Option<String>,
    pub periodo_anterior: Option<String>,
    pub fecha_corte: Option<NaiveDate>,
    /// Filtros aplicados, con el periodo ya resuelto.
    pub filtros: Filtros,
    pub indicadores: Vec<Indicador>,
}

/// "Datos usados" (TR-D16), con el aviso de periodos mezclados (IND-C2).
#[derive(schemars::JsonSchema, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Estado {
    pub marca: String,
    pub version_motor: String,
    pub fecha_corte: Option<NaiveDate>,
    pub periodos: Vec<String>,
    /// Archivos usados, con la fecha leída de su nombre (TR-D3).
    pub fuentes: Vec<crate::proceso::Fuente>,
    /// Número de avisos de la ejecución (lectura y controles).
    pub avisos: usize,
    pub periodos_mezclados: bool,
    pub aviso_periodos: Option<String>,
}

/// Valores posibles de cada filtro.
#[derive(schemars::JsonSchema, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct OpcionesFiltro {
    pub periodos: Vec<String>,
    pub gerencias: Vec<String>,
    pub sedes: Vec<String>,
    pub tipos_trabajador: Vec<String>,
}

#[derive(schemars::JsonSchema, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Punto {
    pub periodo: String,
    pub valor: Option<f64>,
}

#[derive(schemars::JsonSchema, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Serie {
    pub id: String,
    pub nombre: String,
    pub unidad: String,
    pub puntos: Vec<Punto>,
}

#[derive(schemars::JsonSchema, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct RespuestaSeries {
    pub filtros: Filtros,
    pub series: Vec<Serie>,
}

#[derive(schemars::JsonSchema, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Check {
    pub id: String,
    pub valor: Option<f64>,
    pub mensaje: String,
}

#[derive(schemars::JsonSchema, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct RespuestaChecks {
    pub periodo: Option<String>,
    pub checks: Vec<Check>,
}

/// Detalle nominal (TR-D18): la única respuesta con nombres y documentos.
#[derive(schemars::JsonSchema, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct RespuestaDetalle {
    pub tipo: super::Detalle,
    pub periodo: Option<String>,
    pub filas: Vec<serde_json::Map<String, serde_json::Value>>,
}

/// Cuerpo de todo error de la API: `{ "codigo", "mensaje", "opciones"? }` (ADR-0004 §3).
#[derive(schemars::JsonSchema, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ErrorApi {
    pub codigo: String,
    pub mensaje: String,
    /// Valores válidos, cuando el error es un filtro inválido.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opciones: Option<Vec<String>>,
}
