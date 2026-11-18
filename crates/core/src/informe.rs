//! Problemas encontrados al validar o leer un archivo (ADR-0003 §4).

use serde::{Deserialize, Serialize};

#[derive(schemars::JsonSchema, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Severidad {
    Error,
    Aviso,
}

#[derive(schemars::JsonSchema, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Problema {
    /// ID de regla de `docs/logica-negocio/` (por ejemplo, `PRE-10`).
    pub codigo: String,
    pub severidad: Severidad,
    /// Nombre base del archivo.
    pub archivo: String,
    pub hoja: Option<String>,
    /// Celda (`A6`) o ubicación (`fila 6`).
    pub celda: Option<String>,
    /// TR-D13: dónde se encontró lo que se esperaba en `celda` (por ejemplo, la cabecera desplazada a `A7`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub celda_encontrada: Option<String>,
    /// Texto en español claro para el usuario.
    pub mensaje: String,
    /// Detalle técnico para soporte (por ejemplo, el error del lector de Excel).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detalle: Option<String>,
}

#[derive(schemars::JsonSchema, Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct Informe {
    pub problemas: Vec<Problema>,
}

impl Informe {
    pub fn tiene_errores(&self) -> bool {
        self.errores().next().is_some()
    }

    pub fn errores(&self) -> impl Iterator<Item = &Problema> {
        self.problemas
            .iter()
            .filter(|p| p.severidad == Severidad::Error)
    }

    pub fn avisos(&self) -> impl Iterator<Item = &Problema> {
        self.problemas
            .iter()
            .filter(|p| p.severidad == Severidad::Aviso)
    }
}

/// Acumula problemas de un mismo archivo.
pub(crate) struct Registro {
    archivo: String,
    pub(crate) informe: Informe,
}

impl Registro {
    pub(crate) fn nuevo(archivo: String) -> Self {
        Self {
            archivo,
            informe: Informe::default(),
        }
    }

    pub(crate) fn con_informe(archivo: String, informe: Informe) -> Self {
        Self { archivo, informe }
    }

    pub(crate) fn agregar(
        &mut self,
        severidad: Severidad,
        codigo: &str,
        hoja: Option<&str>,
        celda: Option<String>,
        mensaje: String,
    ) {
        self.informe.problemas.push(Problema {
            codigo: codigo.to_string(),
            severidad,
            archivo: self.archivo.clone(),
            hoja: hoja.map(String::from),
            celda,
            celda_encontrada: None,
            mensaje,
            detalle: None,
        });
    }

    /// TR-D13: error con la celda esperada y la celda donde se encontró el dato.
    pub(crate) fn error_desplazado(
        &mut self,
        codigo: &str,
        hoja: Option<&str>,
        celda: String,
        encontrada: String,
        mensaje: String,
    ) {
        self.error(codigo, hoja, Some(celda), mensaje);
        if let Some(p) = self.informe.problemas.last_mut() {
            p.celda_encontrada = Some(encontrada);
        }
    }

    pub(crate) fn error(
        &mut self,
        codigo: &str,
        hoja: Option<&str>,
        celda: Option<String>,
        mensaje: String,
    ) {
        self.agregar(Severidad::Error, codigo, hoja, celda, mensaje);
    }

    pub(crate) fn error_con_detalle(
        &mut self,
        codigo: &str,
        hoja: Option<&str>,
        celda: Option<String>,
        mensaje: String,
        detalle: String,
    ) {
        self.error(codigo, hoja, celda, mensaje);
        if let Some(p) = self.informe.problemas.last_mut() {
            p.detalle = Some(detalle);
        }
    }

    pub(crate) fn aviso(
        &mut self,
        codigo: &str,
        hoja: Option<&str>,
        celda: Option<String>,
        mensaje: String,
    ) {
        self.agregar(Severidad::Aviso, codigo, hoja, celda, mensaje);
    }
}
