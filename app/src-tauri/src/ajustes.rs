//! Ajustes del usuario (`app_config_dir()/ajustes.json`). Un archivo roto o ausente da los valores por defecto.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
#[serde(default)]
pub struct Ajustes {
    /// Carpeta de salida elegida por el usuario (sin pantalla de ajustes todavía: pendiente de diseño).
    pub carpeta_salida: Option<PathBuf>,
    /// Duración de la última ejecución de cada reporte, en ms (para el tiempo estimado).
    pub duraciones_ms: BTreeMap<String, u64>,
}

pub fn cargar(ruta: &Path) -> Ajustes {
    std::fs::read_to_string(ruta)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

pub fn guardar(ruta: &Path, a: &Ajustes) -> std::io::Result<()> {
    if let Some(carpeta) = ruta.parent() {
        std::fs::create_dir_all(carpeta)?;
    }
    let texto = serde_json::to_string_pretty(a).expect("Ajustes es serializable");
    std::fs::write(ruta, texto)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ajustes_inexistentes_dan_defecto() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(cargar(&dir.path().join("no.json")), Ajustes::default());
        std::fs::write(dir.path().join("roto.json"), "{no es json").unwrap();
        assert_eq!(cargar(&dir.path().join("roto.json")), Ajustes::default());
    }

    #[test]
    fn ajustes_ida_y_vuelta() {
        let dir = tempfile::tempdir().unwrap();
        let ruta = dir.path().join("sub/ajustes.json");
        let mut a = Ajustes {
            carpeta_salida: Some(PathBuf::from("C:/Reportes")),
            ..Default::default()
        };
        a.duraciones_ms.insert("planilla".into(), 12_000);
        guardar(&ruta, &a).unwrap();
        assert_eq!(cargar(&ruta), a);
    }
}
