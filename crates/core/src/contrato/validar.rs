//! Reglas V1–V11 del contrato (Plan 3, Tarea 2). Devuelve un detalle legible por regla incumplida.

use std::collections::HashMap;

use regex::Regex;

use super::{Contrato, TipoDato, VERSION_FORMATO};
use crate::normalizar::normalizar_cabecera;

const EXTENSIONES: [&str; 3] = ["xlsx", "xlsm", "xls"];

pub(crate) fn es_canonico(nombre: &str) -> bool {
    let mut chars = nombre.chars();
    chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// Compila un patrón exigiendo coincidencia completa (`^(?:…)$`).
pub(crate) fn patron_completo(patron: &str) -> Result<Regex, regex::Error> {
    Regex::new(&format!("^(?:{patron})$"))
}

fn compilar(patron: &str, donde: &str, detalles: &mut Vec<String>) -> Option<Regex> {
    match patron_completo(patron) {
        Ok(r) => Some(r),
        Err(e) => {
            detalles.push(format!(
                "{donde}: la expresión «{patron}» no es válida: {e}"
            ));
            None
        }
    }
}

fn tiene_grupo(r: &Regex, grupo: &str) -> bool {
    r.capture_names().flatten().any(|n| n == grupo)
}

pub(super) fn validar(c: &Contrato) -> Vec<String> {
    let mut d = Vec::new();
    if c.version != VERSION_FORMATO {
        d.push(format!(
            "version {} no soportada (este motor entiende la {VERSION_FORMATO})",
            c.version
        ));
    }
    if !es_canonico(&c.fuente) {
        d.push(format!(
            "fuente «{}» no es un nombre canónico (snake_case ASCII)",
            c.fuente
        ));
    }
    if c.archivo.extensiones.is_empty() {
        d.push("archivo.extensiones está vacía".into());
    }
    for ext in &c.archivo.extensiones {
        if !EXTENSIONES.contains(&ext.as_str()) {
            d.push(format!(
                "extensión «{ext}» no permitida (use xlsx, xlsm o xls)"
            ));
        }
    }
    let patron = c
        .archivo
        .patron_nombre
        .as_deref()
        .and_then(|p| compilar(p, "archivo.patron_nombre", &mut d));
    if let Some(fecha) = &c.archivo.fecha_en_nombre
        && !patron
            .as_ref()
            .is_some_and(|r| tiene_grupo(r, &fecha.grupo))
    {
        d.push(format!(
            "fecha_en_nombre usa el grupo «{}», pero patron_nombre no lo define",
            fecha.grupo
        ));
    }
    if c.hojas.is_empty() {
        d.push("hojas está vacía".into());
    }
    for hoja in &c.hojas {
        let h = hoja.nombre.lista().join(" / ");
        let nombres_hoja = hoja.nombre.lista();
        if nombres_hoja.is_empty() || nombres_hoja.iter().any(|n| n.trim().is_empty()) {
            d.push(format!(
                "hoja «{h}»: el nombre de hoja no puede estar vacío"
            ));
        }
        if hoja.filas_resumen.iter().any(|t| t.trim().is_empty()) {
            d.push(format!(
                "hoja «{h}»: filas_resumen no puede tener textos vacíos"
            ));
        }
        if hoja.columnas.is_empty() {
            d.push(format!("hoja «{h}»: no tiene columnas"));
        }
        let mut nombres: HashMap<&str, usize> = HashMap::new();
        let mut origenes: HashMap<String, &str> = HashMap::new();
        for col in &hoja.columnas {
            let donde = format!("hoja «{h}», columna «{}»", col.nombre);
            if !es_canonico(&col.nombre) {
                d.push(format!(
                    "{donde}: el nombre no es canónico (snake_case ASCII)"
                ));
            }
            *nombres.entry(col.nombre.as_str()).or_default() += 1;
            if col.origen.is_empty() {
                d.push(format!("{donde}: origen está vacío"));
            }
            let mut propios: Vec<String> = Vec::new();
            for o in &col.origen {
                let n = normalizar_cabecera(o);
                if propios.contains(&n) {
                    d.push(format!("{donde}: la columna repite el origen «{o}»"));
                    continue;
                }
                propios.push(n.clone());
                if let Some(otra) = origenes.insert(n.clone(), col.nombre.as_str()) {
                    d.push(format!(
                        "{donde}: el origen «{o}» se confunde con el de «{otra}» (ambos son «{n}»)"
                    ));
                }
            }
            if !col.valores.is_empty() && col.tipo != TipoDato::Texto {
                d.push(format!("{donde}: valores solo se permite con tipo texto"));
            }
            if (col.minimo.is_some() || col.maximo.is_some())
                && !matches!(col.tipo, TipoDato::Entero | TipoDato::Decimal)
            {
                d.push(format!(
                    "{donde}: minimo/maximo solo con tipo entero o decimal"
                ));
            }
            if !col.formatos.is_empty() && col.tipo != TipoDato::Fecha {
                d.push(format!("{donde}: formatos solo se permite con tipo fecha"));
            }
        }
        let mut repetidas: Vec<_> = nombres.into_iter().filter(|(_, v)| *v > 1).collect();
        repetidas.sort();
        for (nombre, _) in repetidas {
            d.push(format!("hoja «{h}»: la columna «{nombre}» está repetida"));
        }
        if hoja.columnas.iter().filter(|c| c.clave).count() > 1 {
            d.push(format!("hoja «{h}»: más de una columna con clave: true"));
        }
        for din in &hoja.columnas_dinamicas {
            let donde = format!("hoja «{h}», columna dinámica «{}»", din.prefijo);
            if !es_canonico(&din.prefijo) {
                d.push(format!(
                    "{donde}: el prefijo no es canónico (snake_case ASCII)"
                ));
            }
            if let Some(r) = compilar(&din.patron_origen, &donde, &mut d)
                && !tiene_grupo(&r, "motivo")
            {
                d.push(format!(
                    "{donde}: patron_origen debe tener el grupo «motivo»"
                ));
            }
        }
    }
    d
}
