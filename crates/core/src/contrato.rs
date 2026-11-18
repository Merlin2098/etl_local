//! Contrato de fuente (ADR-0003): un archivo YAML por fuente describe nombre de archivo, hojas,
//! cabecera y columnas, con su nombre canónico y su tipo.

pub(crate) mod validar;

use std::collections::BTreeMap;
use std::path::Path;

use schemars::JsonSchema;
use serde::Deserialize;

use crate::celda::Celda;
pub use crate::error::ErrorContrato;

/// Versión del formato que entiende este motor.
pub const VERSION_FORMATO: u32 = 1;

#[derive(Deserialize, JsonSchema, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct Contrato {
    /// Versión del formato de contrato (hoy, 1).
    pub version: u32,
    /// Nombre canónico de la fuente (`planilla`, `base_personal`…).
    pub fuente: String,
    /// Nombre que ve el usuario.
    pub titulo: String,
    pub archivo: ReglasArchivo,
    pub hojas: Vec<Hoja>,
}

#[derive(Deserialize, JsonSchema, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct ReglasArchivo {
    /// Extensiones permitidas, sin punto y en minúsculas.
    pub extensiones: Vec<String>,
    /// Expresión regular que debe coincidir con el nombre completo del archivo.
    #[serde(default)]
    pub patron_nombre: Option<String>,
    /// Fecha que se extrae del nombre del archivo (periodo o fecha de corte).
    #[serde(default)]
    pub fecha_en_nombre: Option<FechaEnNombre>,
}

#[derive(Deserialize, JsonSchema, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct FechaEnNombre {
    /// Grupo con nombre de `patron_nombre` que contiene la fecha.
    pub grupo: String,
    pub formato: FormatoFechaNombre,
}

#[derive(Deserialize, JsonSchema, Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatoFechaNombre {
    #[serde(rename = "AAAA-MM")]
    AnioMes,
    #[serde(rename = "DD.MM.AAAA")]
    DiaMesAnio,
}

#[derive(Deserialize, JsonSchema, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct Hoja {
    /// Nombre exacto de la hoja, o lista de nombres aceptados.
    pub nombre: NombresHoja,
    /// Celda donde empieza la cabecera (por ejemplo, `A6`).
    #[schemars(with = "String")]
    pub cabecera: Celda,
    #[serde(default)]
    pub fin_datos: FinDatos,
    #[serde(default)]
    pub columnas_extra: ColumnasExtra,
    pub columnas: Vec<Columna>,
    #[serde(default)]
    pub columnas_dinamicas: Vec<ColumnaDinamica>,
    /// Textos de filas de totales o subtotales (NOM-BRZ-10). Una fila cuya primera celda, sin `+`, `-`,
    /// `*` ni espacios al inicio, contiene alguno (comparación normalizada) se descarta sin aviso.
    #[serde(default)]
    pub filas_resumen: Vec<String>,
}

impl Hoja {
    /// La columna marcada con `clave: true`, si hay una.
    pub fn clave(&self) -> Option<&Columna> {
        self.columnas.iter().find(|c| c.clave)
    }
}

#[derive(Deserialize, JsonSchema, Debug, Clone)]
#[serde(untagged)]
pub enum NombresHoja {
    Uno(String),
    Varios(Vec<String>),
}

impl NombresHoja {
    pub fn lista(&self) -> Vec<&str> {
        match self {
            NombresHoja::Uno(n) => vec![n.as_str()],
            NombresHoja::Varios(v) => v.iter().map(String::as_str).collect(),
        }
    }
}

/// Hasta dónde se leen filas (TR-D2).
#[derive(Deserialize, JsonSchema, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum FinDatos {
    #[default]
    FinHoja,
    FilaVacia,
    ClaveVacia,
}

#[derive(Deserialize, JsonSchema, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ColumnasExtra {
    #[default]
    Ignorar,
    Conservar,
}

fn verdadero() -> bool {
    true
}

#[derive(Deserialize, JsonSchema, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct Columna {
    /// Nombre canónico (`snake_case` ASCII).
    pub nombre: String,
    /// Cabeceras posibles en la fuente; gana la primera presente.
    pub origen: Vec<String>,
    pub tipo: TipoDato,
    /// La cabecera debe existir.
    #[serde(default)]
    pub requerida: bool,
    /// Se aceptan filas sin valor.
    #[serde(default = "verdadero")]
    pub nula: bool,
    /// Columna que identifica una fila de datos; las filas sin clave se descartan.
    #[serde(default)]
    pub clave: bool,
    /// Formatos aceptados para `fecha` (vacío = solo fecha de Excel).
    #[serde(default)]
    pub formatos: Vec<FormatoFecha>,
    #[serde(default)]
    pub minimo: Option<f64>,
    #[serde(default)]
    pub maximo: Option<f64>,
    /// Valores permitidos para `texto`.
    #[serde(default)]
    pub valores: Vec<String>,
}

#[derive(Deserialize, serde::Serialize, JsonSchema, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum TipoDato {
    Texto,
    Entero,
    Decimal,
    Fecha,
    Periodo,
    Documento,
    Booleano,
}

#[derive(Deserialize, JsonSchema, Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatoFecha {
    #[serde(rename = "excel")]
    Excel,
    #[serde(rename = "DD/MM/AAAA")]
    DiaMesAnio,
    #[serde(rename = "AAAA-MM-DD")]
    Iso,
}

#[derive(Deserialize, JsonSchema, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct ColumnaDinamica {
    /// Prefijo del nombre canónico; se completa con el grupo `motivo`.
    pub prefijo: String,
    /// Expresión regular sobre la cabecera normalizada, con el grupo `motivo`.
    pub patron_origen: String,
    pub tipo: TipoDato,
}

impl Contrato {
    /// Lee y valida un contrato. `origen` identifica el texto en los mensajes (normalmente la ruta).
    pub fn desde_yaml(texto: &str, origen: &str) -> Result<Contrato, ErrorContrato> {
        let contrato: Contrato =
            serde_saphyr::from_str(texto).map_err(|e| ErrorContrato::Yaml {
                origen: origen.to_string(),
                mensaje: e.to_string(),
            })?;
        let detalles = validar::validar(&contrato);
        if detalles.is_empty() {
            Ok(contrato)
        } else {
            Err(ErrorContrato::Invalido {
                origen: origen.to_string(),
                detalles,
            })
        }
    }
}

/// Carga un contrato desde un archivo.
pub fn cargar_contrato(ruta: &Path) -> Result<Contrato, ErrorContrato> {
    let texto = std::fs::read_to_string(ruta).map_err(|fuente| ErrorContrato::Lectura {
        ruta: ruta.to_path_buf(),
        fuente,
    })?;
    Contrato::desde_yaml(&texto, &ruta.display().to_string())
}

/// Carga todos los `*.yaml` de una carpeta, indexados por `fuente` (V12).
pub fn cargar_contratos(carpeta: &Path) -> Result<BTreeMap<String, Contrato>, ErrorContrato> {
    let lectura = |fuente| ErrorContrato::Lectura {
        ruta: carpeta.to_path_buf(),
        fuente,
    };
    let mut rutas = Vec::new();
    for entrada in std::fs::read_dir(carpeta).map_err(lectura)? {
        let ruta = entrada.map_err(lectura)?.path();
        if ruta.extension().is_some_and(|x| x == "yaml") {
            rutas.push(ruta);
        }
    }
    rutas.sort();
    let mut todos = BTreeMap::new();
    for ruta in rutas {
        let contrato = cargar_contrato(&ruta)?;
        let esperado = ruta.file_stem().unwrap_or_default().to_string_lossy();
        if contrato.fuente != esperado {
            return Err(ErrorContrato::Invalido {
                origen: ruta.display().to_string(),
                detalles: vec![format!(
                    "la fuente «{}» no coincide con el nombre del archivo «{esperado}»",
                    contrato.fuente
                )],
            });
        }
        todos.insert(contrato.fuente.clone(), contrato);
    }
    Ok(todos)
}
