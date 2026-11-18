//! Parámetros de negocio versionados (TR-D10), leídos de `config/parametros.yaml`.
//!
//! El SQL de `config/sql/` los recibe como variables de DuckDB (ver `Almacen::fijar_variables`).

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;

use crate::contrato::validar::es_canonico;
use crate::error::ErrorContrato;

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct Parametros {
    pub version: u32,
    pub jubilacion: Jubilacion,
    pub contrato_obra: ReglaContrato,
    pub contrato_incremento: ReglaContrato,
    pub practicante: Practicante,
    pub licencias: Licencias,
    pub subsidios: Subsidios,
    /// Plan 8, CP-FLG-04…07: ventanas de días de servicio del control de practicantes.
    pub control_practicantes: ControlPracticantes,
    /// Plan 8, ER-GLD-06: cargo que identifica a los practicantes en los exámenes de salida.
    pub examenes_salida: ExamenesSalida,
    /// Metas del tablero por indicador (indicadores-tablero.md §4). Opcional.
    #[serde(default)]
    pub metas: BTreeMap<String, f64>,
}

/// LIC-SLV-05: hoja del control de licencias → `tipo_licencia`.
#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct Licencias {
    pub tipos: BTreeMap<String, String>,
}

/// BD-FLG-05…07, TR-D15.
#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct Jubilacion {
    pub edad: u32,
    pub edad_extendida: u32,
}

/// BD-FLG-08…10.
#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct ReglaContrato {
    pub modalidad: String,
    pub anios_alerta: f64,
}

/// BD-GLD-06.
#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct Practicante {
    pub modalidad_contiene: String,
}

/// CP-FLG-04…07: condición que tiene alertas y ventanas de días (inclusivas) de cada alerta.
#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct ControlPracticantes {
    pub condicion: String,
    pub por_cumplir_1: [u32; 2],
    pub cumplio_1: u32,
    pub por_cumplir_2: [u32; 2],
}

/// ER-GLD-06: los exámenes de salida excluyen las filas con este cargo (comparado normalizado).
#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct ExamenesSalida {
    pub cargo_practicante: String,
}

/// Columnas dinámicas de subsidios de la planilla (vocabulario §4).
#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct Subsidios {
    pub prefijo: String,
    pub conservar: Vec<String>,
    pub resto: String,
    #[serde(default)]
    pub fijas: Vec<String>,
}

fn validar(p: &Parametros) -> Vec<String> {
    let mut d = Vec::new();
    if p.version != 1 {
        d.push(format!(
            "version {} no soportada (este motor entiende la 1)",
            p.version
        ));
    }
    if p.jubilacion.edad == 0 || p.jubilacion.edad_extendida <= p.jubilacion.edad {
        d.push("jubilacion: edad debe ser mayor que 0 y edad_extendida mayor que edad".into());
    }
    for (nombre, regla) in [
        ("contrato_obra", &p.contrato_obra),
        ("contrato_incremento", &p.contrato_incremento),
    ] {
        if regla.modalidad.trim().is_empty() {
            d.push(format!("{nombre}.modalidad está vacía"));
        }
        if !(regla.anios_alerta.is_finite() && regla.anios_alerta > 0.0) {
            d.push(format!(
                "{nombre}.anios_alerta debe ser un número mayor que 0"
            ));
        }
    }
    if p.practicante.modalidad_contiene.trim().is_empty() {
        d.push("practicante.modalidad_contiene está vacía".into());
    }
    if p.licencias.tipos.is_empty()
        || p.licencias
            .tipos
            .iter()
            .any(|(h, t)| h.trim().is_empty() || t.trim().is_empty())
    {
        d.push("licencias.tipos debe tener al menos una hoja y ningún texto vacío".into());
    }
    // El SQL de la planilla Gold (config/sql/planilla/20_gold.sql) separa los motivos por estos dos valores.
    for tipo in p.licencias.tipos.values() {
        if tipo != "CON_GOCE" && tipo != "SIN_GOCE" {
            d.push(format!(
                "licencias.tipos: «{tipo}» no es un tipo válido (use CON_GOCE o SIN_GOCE)"
            ));
        }
    }
    for (clave, valor) in &p.metas {
        let valida = clave.len() == 6
            && clave.starts_with("IND-0")
            && matches!(clave.as_bytes()[5], b'1'..=b'8');
        if !valida {
            d.push(format!(
                "metas: «{clave}» no es un indicador (use IND-01 … IND-08)"
            ));
        }
        if !(valor.is_finite() && *valor >= 0.0) {
            d.push(format!(
                "metas: {clave} debe ser un número mayor o igual que 0"
            ));
        }
    }
    let s = &p.subsidios;
    for nombre in std::iter::once(&s.prefijo)
        .chain(std::iter::once(&s.resto))
        .chain(&s.conservar)
        .chain(&s.fijas)
    {
        if !es_canonico(nombre) {
            d.push(format!(
                "subsidios: «{nombre}» no es un nombre canónico (snake_case ASCII)"
            ));
        }
    }
    for nombre in s
        .conservar
        .iter()
        .chain(&s.fijas)
        .chain(std::iter::once(&s.resto))
    {
        if !nombre.starts_with(&s.prefijo) {
            d.push(format!(
                "subsidios: «{nombre}» no empieza por el prefijo «{}»",
                s.prefijo
            ));
        }
    }
    d
}

/// Lee y valida `parametros.yaml`. Reutiliza los errores de contrato (YAML inválido o regla incumplida).
pub fn cargar_parametros(ruta: &Path) -> Result<Parametros, ErrorContrato> {
    let origen = ruta.display().to_string();
    let texto = std::fs::read_to_string(ruta).map_err(|fuente| ErrorContrato::Lectura {
        ruta: ruta.to_path_buf(),
        fuente,
    })?;
    let p: Parametros = serde_saphyr::from_str(&texto).map_err(|e| ErrorContrato::Yaml {
        origen: origen.clone(),
        mensaje: e.to_string(),
    })?;
    let detalles = validar(&p);
    if detalles.is_empty() {
        Ok(p)
    } else {
        Err(ErrorContrato::Invalido { origen, detalles })
    }
}
