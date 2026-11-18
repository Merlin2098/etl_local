//! Catálogo de reportes de la app (`config/reportes.yaml`, Plan 6) y su vigencia para la pantalla de inicio.

use std::collections::BTreeSet;
use std::path::Path;

use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};

use crate::contrato::cargar_contratos;
use crate::error::{ErrorContrato, ErrorProceso};
use crate::preflight::{FechaNombre, fecha_en_nombre};

#[derive(schemars::JsonSchema, Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Genero {
    Femenino,
    Masculino,
}

/// Un reporte que el usuario puede actualizar (una tarjeta de la pantalla 1).
#[derive(schemars::JsonSchema, Deserialize, Serialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Reporte {
    pub id: String,
    pub titulo: String,
    pub descripcion: String,
    /// Contrato principal (`config/fuentes/<fuente>.yaml`).
    pub fuente: String,
    /// Se pueden elegir varios archivos (uno por periodo).
    pub varios_archivos: bool,
    /// Contratos que se pueden agregar a este reporte (por ejemplo, `licencias` en la planilla).
    #[serde(default)]
    pub complementos: Vec<String>,
    /// Nombre del archivo de `actual/excel/` (sin extensión) que abre «Abrir en Excel».
    pub excel: String,
    pub genero: Genero,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Catalogo {
    reportes: Vec<Reporte>,
}

fn identificador_valido(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_lowercase() || c == '_')
}

/// Lee y valida `<config>/reportes.yaml` contra los contratos de `<config>/fuentes/`.
pub fn cargar_reportes(config: &Path) -> Result<Vec<Reporte>, ErrorContrato> {
    let ruta = config.join("reportes.yaml");
    let origen = ruta.display().to_string();
    let texto = std::fs::read_to_string(&ruta).map_err(|fuente| ErrorContrato::Lectura {
        ruta: ruta.clone(),
        fuente,
    })?;
    let catalogo: Catalogo = serde_saphyr::from_str(&texto).map_err(|e| ErrorContrato::Yaml {
        origen: origen.clone(),
        mensaje: e.to_string(),
    })?;
    let contratos = cargar_contratos(&config.join("fuentes"))?;
    let mut detalles = Vec::new();
    let mut vistos = BTreeSet::new();
    for r in &catalogo.reportes {
        if !identificador_valido(&r.id) {
            detalles.push(format!("el id «{}» solo puede tener minúsculas y _", r.id));
        }
        if !vistos.insert(r.id.as_str()) {
            detalles.push(format!("el id «{}» está repetido", r.id));
        }
        for fuente in std::iter::once(&r.fuente).chain(&r.complementos) {
            if !contratos.contains_key(fuente) {
                detalles.push(format!(
                    "{}: la fuente «{fuente}» no tiene contrato en config/fuentes/",
                    r.id
                ));
            }
        }
        if !identificador_valido(&r.excel) {
            detalles.push(format!(
                "{}: excel «{}» solo puede tener minúsculas y _",
                r.id, r.excel
            ));
        }
    }
    if detalles.is_empty() {
        Ok(catalogo.reportes)
    } else {
        Err(ErrorContrato::Invalido { origen, detalles })
    }
}

#[derive(schemars::JsonSchema, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Vigencia {
    Nunca,
    AlDia,
    Atrasado,
}

#[derive(schemars::JsonSchema, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct EstadoReporte {
    pub reporte: Reporte,
    pub vigencia: Vigencia,
    /// La fecha más reciente entre los archivos guardados de la fuente del reporte.
    pub ultimo: Option<FechaNombre>,
}

/// Estado de cada reporte según `<salida>/entradas/<fuente>/`. `hoy` lo da quien llama (AGENTS.md §4.5).
pub fn estado_reportes(
    config: &Path,
    salida: &Path,
    hoy: NaiveDate,
) -> Result<Vec<EstadoReporte>, ErrorProceso> {
    let reportes = cargar_reportes(config)?;
    let contratos = cargar_contratos(&config.join("fuentes"))?;
    let mut estados = Vec::new();
    for reporte in reportes {
        let contrato = &contratos[&reporte.fuente];
        let ultimo = archivos_de(&salida.join("entradas").join(&reporte.fuente))
            .iter()
            .filter_map(|n| fecha_en_nombre(contrato, n))
            .max_by(comparar);
        // Plan 8: un contrato sin fecha en el nombre (p. ej. exámenes de salida) está al día si tiene archivos.
        let sin_fecha = contrato.archivo.fecha_en_nombre.is_none()
            && !archivos_de(&salida.join("entradas").join(&reporte.fuente)).is_empty();
        let vigencia = match &ultimo {
            None if sin_fecha => Vigencia::AlDia,
            None => Vigencia::Nunca,
            Some(f) if al_dia(f, hoy) => Vigencia::AlDia,
            Some(_) => Vigencia::Atrasado,
        };
        estados.push(EstadoReporte {
            reporte,
            vigencia,
            ultimo,
        });
    }
    Ok(estados)
}

/// Nombres de archivo de una carpeta (sin temporales de Excel `~$…`); vacío si no existe.
pub(crate) fn archivos_de(carpeta: &Path) -> Vec<String> {
    let Ok(dir) = std::fs::read_dir(carpeta) else {
        return Vec::new();
    };
    let mut v: Vec<String> = dir
        .filter_map(Result::ok)
        .filter(|e| e.path().is_file())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| !n.starts_with("~$") && !n.starts_with('.'))
        .collect();
    v.sort();
    v
}

fn comparar(a: &FechaNombre, b: &FechaNombre) -> std::cmp::Ordering {
    use FechaNombre::{FechaCorte, Periodo};
    match (a, b) {
        (Periodo(x), Periodo(y)) => x.cmp(y),
        (FechaCorte(x), FechaCorte(y)) => x.cmp(y),
        (Periodo(_), FechaCorte(_)) => std::cmp::Ordering::Less,
        (FechaCorte(_), Periodo(_)) => std::cmp::Ordering::Greater,
    }
}

fn al_dia(f: &FechaNombre, hoy: NaiveDate) -> bool {
    match f {
        FechaNombre::Periodo(p) => {
            let (anio, mes) = if hoy.month() == 1 {
                (hoy.year() - 1, 12)
            } else {
                (hoy.year(), hoy.month() - 1)
            };
            p.as_str() >= format!("{anio:04}-{mes:02}").as_str()
        }
        FechaNombre::FechaCorte(c) => (hoy - *c).num_days() <= 31,
    }
}
