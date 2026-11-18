//! Acceso al libro de Excel con `calamine`. Posiciones 1-based, como en Excel.
//!
//! Las hojas se cargan bajo demanda (`cargar`): solo las que pide el contrato. Así una hoja
//! dañada o enorme que el contrato no usa no impide leer el archivo.

use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use calamine::{Data, Range, Reader, Sheets, open_workbook_auto};

pub(crate) struct Libro {
    lector: Sheets<BufReader<File>>,
    nombres: Vec<String>,
    cargadas: Vec<(String, Range<Data>)>,
}

impl Libro {
    /// Abre el libro y lee la lista de hojas. El error es el detalle técnico de `calamine`.
    pub(crate) fn abrir(ruta: &Path) -> Result<Libro, String> {
        let lector = open_workbook_auto(ruta).map_err(|e| e.to_string())?;
        let nombres = lector.sheet_names();
        Ok(Libro {
            lector,
            nombres,
            cargadas: Vec::new(),
        })
    }

    pub(crate) fn tiene_hoja(&self, nombre: &str) -> bool {
        self.nombres.iter().any(|n| n == nombre)
    }

    /// Carga una hoja para poder leer sus celdas. El error es el detalle técnico de `calamine`.
    pub(crate) fn cargar(&mut self, hoja: &str) -> Result<(), String> {
        if self.rango(hoja).is_none() {
            let rango = self
                .lector
                .worksheet_range(hoja)
                .map_err(|e| e.to_string())?;
            self.cargadas.push((hoja.to_string(), rango));
        }
        Ok(())
    }

    fn rango(&self, hoja: &str) -> Option<&Range<Data>> {
        self.cargadas
            .iter()
            .find(|(n, _)| n == hoja)
            .map(|(_, r)| r)
    }

    /// Valor de una celda (1-based). `None` si está fuera del rango usado o la hoja no se cargó.
    pub(crate) fn valor(&self, hoja: &str, fila: u32, columna: u32) -> Option<&Data> {
        if fila == 0 || columna == 0 {
            return None;
        }
        self.rango(hoja)?.get_value((fila - 1, columna - 1))
    }

    /// Texto recortado de una celda, o `None` si está vacía (PRE-13).
    pub(crate) fn texto(&self, hoja: &str, fila: u32, columna: u32) -> Option<String> {
        let texto = match self.valor(hoja, fila, columna)? {
            Data::Empty => return None,
            Data::String(s) => s.trim().to_string(),
            Data::Float(f) if f.fract() == 0.0 && f.abs() < 1e15 => format!("{}", *f as i64),
            otro => otro.to_string().trim().to_string(),
        };
        (!texto.is_empty()).then_some(texto)
    }

    /// Última fila usada de la hoja (1-based), o 0 si está vacía.
    pub(crate) fn ultima_fila(&self, hoja: &str) -> u32 {
        self.rango(hoja)
            .and_then(Range::end)
            .map_or(0, |(fila, _)| fila + 1)
    }
}
