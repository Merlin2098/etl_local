//! Revisión de un archivo para la app (Plan 6): fecha del nombre, personas y recorte de la hoja (TR-D13).

use std::collections::BTreeSet;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::celda::Celda;
use crate::contrato::Contrato;
use crate::informe::Informe;
use crate::lectura::{Valor, leer_archivo};
use crate::libro::Libro;
use crate::preflight::{FechaNombre, fecha_en_nombre};

/// Filas y columnas máximas de un recorte.
const MAX_FILAS: u32 = 12;
const MAX_COLUMNAS: u32 = 8;
/// Caracteres máximos por celda del recorte.
const MAX_TEXTO: usize = 40;

/// Resultado de revisar un archivo: lo que la pantalla «Revisión» muestra por fila.
#[derive(schemars::JsonSchema, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Revision {
    /// Nombre base del archivo.
    pub archivo: String,
    pub fecha_nombre: Option<FechaNombre>,
    /// Valores distintos de la columna clave de la primera hoja; `None` si hay errores o no hay clave.
    pub personas: Option<u64>,
    pub informe: Informe,
}

/// Valida y lee el archivo. Nunca falla: los problemas quedan en `informe`.
pub fn revisar_archivo(contrato: &Contrato, ruta: &Path) -> Revision {
    let archivo = ruta.file_name().map_or_else(
        || ruta.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    );
    let fecha_nombre = fecha_en_nombre(contrato, &archivo);
    match leer_archivo(contrato, ruta) {
        Ok(lectura) => {
            let personas = contrato
                .hojas
                .first()
                .and_then(|h| h.clave())
                .zip(lectura.hojas.first())
                .and_then(|(clave, tabla)| tabla.columna(&clave.nombre))
                .map(|col| {
                    col.valores
                        .iter()
                        .filter_map(texto_clave)
                        .collect::<BTreeSet<_>>()
                        .len() as u64
                });
            Revision {
                archivo,
                fecha_nombre,
                personas,
                informe: lectura.informe,
            }
        }
        Err(informe) => Revision {
            archivo,
            fecha_nombre,
            personas: None,
            informe,
        },
    }
}

fn texto_clave(v: &Valor) -> Option<String> {
    match v {
        Valor::Nulo => None,
        Valor::Texto(t) => {
            let t = t.trim();
            (!t.is_empty()).then(|| t.to_string())
        }
        Valor::Entero(i) => Some(i.to_string()),
        Valor::Decimal(d) => Some(d.to_string()),
        Valor::Fecha(f) => Some(f.to_string()),
        Valor::Booleano(b) => Some(b.to_string()),
    }
}

/// Un trozo de hoja, como texto, para mostrar dónde está un problema.
#[derive(schemars::JsonSchema, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Recorte {
    pub hoja: String,
    pub fila_inicio: u32,
    pub columna_inicio: u32,
    /// `celdas[f][c]` es la celda de la fila `fila_inicio + f` y la columna `columna_inicio + c`.
    pub celdas: Vec<Vec<String>>,
}

/// Lee el rango `desde..=hasta` (a lo sumo 12 filas × 8 columnas, contadas desde `desde`).
/// El error es un texto técnico (archivo ilegible u hoja inexistente).
pub fn recorte(ruta: &Path, hoja: &str, desde: Celda, hasta: Celda) -> Result<Recorte, String> {
    let mut libro = Libro::abrir(ruta)?;
    if !libro.tiene_hoja(hoja) {
        return Err(format!("La hoja «{hoja}» no existe."));
    }
    libro.cargar(hoja)?;
    let fila_inicio = desde.fila.max(1);
    let columna_inicio = desde.columna.max(1);
    let fila_fin = hasta.fila.max(fila_inicio).min(fila_inicio + MAX_FILAS - 1);
    let columna_fin = hasta
        .columna
        .max(columna_inicio)
        .min(columna_inicio + MAX_COLUMNAS - 1);
    let celdas = (fila_inicio..=fila_fin)
        .map(|f| {
            (columna_inicio..=columna_fin)
                .map(|c| libro.texto(hoja, f, c).map(recortar).unwrap_or_default())
                .collect()
        })
        .collect();
    Ok(Recorte {
        hoja: hoja.to_string(),
        fila_inicio,
        columna_inicio,
        celdas,
    })
}

fn recortar(texto: String) -> String {
    if texto.chars().count() <= MAX_TEXTO {
        texto
    } else {
        let mut corto: String = texto.chars().take(MAX_TEXTO - 1).collect();
        corto.push('…');
        corto
    }
}
