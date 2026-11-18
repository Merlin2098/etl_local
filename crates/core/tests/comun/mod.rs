//! Apoyo compartido por los tests de integración: fixtures demo y contratos de `config/`.
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use etl_local_core::contrato::{Contrato, cargar_contratos};

pub const CARPETA_CONTRATOS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../config/fuentes");

/// Carpeta de un modo de los fixtures demo (`limpio`, `sucio`, `casos`).
pub fn fixture(modo: &str) -> PathBuf {
    let ruta = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/demo")
        .join(modo);
    assert!(
        ruta.join(".generar-demo").exists(),
        "Falta el fixture {modo}. Regenerarlo con los comandos de crates/core/tests/fixtures/README.md"
    );
    ruta
}

/// Manifiesto del generador para un modo.
pub fn manifiesto(modo: &str) -> serde_json::Value {
    let texto = std::fs::read_to_string(fixture(modo).join("manifiesto.json")).unwrap();
    serde_json::from_str(&texto).unwrap()
}

pub fn contratos() -> BTreeMap<String, Contrato> {
    cargar_contratos(Path::new(CARPETA_CONTRATOS)).unwrap()
}

pub fn contrato(fuente: &str) -> Contrato {
    contratos().remove(fuente).unwrap()
}

/// Archivos `.xlsx` de una carpeta (no recursivo), ordenados.
pub fn excels(carpeta: &Path) -> Vec<PathBuf> {
    let mut v: Vec<_> = std::fs::read_dir(carpeta)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "xlsx"))
        .collect();
    v.sort();
    v
}

/// Planilla de un periodo (`AAAA-MM`) dentro de un modo.
pub fn planilla(modo: &str, periodo: &str) -> PathBuf {
    fixture(modo)
        .join("planilla")
        .join(format!("Planilla {periodo} Empleados.xlsx"))
}

/// Único archivo de la base de personal de un modo.
pub fn base_personal(modo: &str) -> PathBuf {
    excels(&fixture(modo).join("base_personal")).remove(0)
}

pub fn licencias(modo: &str) -> PathBuf {
    fixture(modo).join("planilla/licencias/CONTROL DE LICENCIAS.xlsx")
}

/// Plan 8: los Excel de una fuente del molde dentro de un modo (`<modo>/<fuente>/`).
pub fn otro(modo: &str, fuente: &str) -> Vec<PathBuf> {
    excels(&fixture(modo).join(fuente))
}

/// Rutas relativas de todos los archivos bajo `carpeta`, ordenadas (para comparar antes y después).
pub fn arbol(carpeta: &Path) -> Vec<String> {
    fn recorrer(base: &Path, dir: &Path, v: &mut Vec<String>) {
        for e in std::fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                recorrer(base, &p, v);
            } else {
                v.push(p.strip_prefix(base).unwrap().display().to_string());
            }
        }
    }
    let mut v = Vec::new();
    recorrer(carpeta, carpeta, &mut v);
    v.sort();
    v
}
