//! Normalización única de cabeceras (TR-D1, ADR-0003 §3).
//!
//! La usan igual la validación previa y la lectura: mayúsculas, sin tildes, espacios colapsados
//! y recortados. La puntuación se conserva (`N°` ≠ `N`).

use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;

/// Normaliza una cabecera para compararla con el `origen` de un contrato.
pub fn normalizar_cabecera(texto: &str) -> String {
    let sin_tildes: String = texto.nfd().filter(|c| !is_combining_mark(*c)).collect();
    sin_tildes
        .to_uppercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Convierte una cabecera en un nombre canónico `snake_case` ASCII.
pub fn slug(texto: &str) -> String {
    let mut salida = String::new();
    for c in normalizar_cabecera(texto).to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            salida.push(c);
        } else if !salida.ends_with('_') {
            salida.push('_');
        }
    }
    salida.trim_matches('_').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normaliza_mayusculas_tildes_y_espacios() {
        assert_eq!(normalizar_cabecera("  Condición "), "CONDICION");
        assert_eq!(normalizar_cabecera("FECHA  DE\nCESE"), "FECHA DE CESE");
        assert_eq!(normalizar_cabecera("FECHA\u{a0}ING"), "FECHA ING");
        assert_eq!(normalizar_cabecera("año"), "ANO");
        assert_eq!(normalizar_cabecera("N°"), "N°");
        assert_ne!(
            normalizar_cabecera("F. RENOVACION"),
            normalizar_cabecera("F RENOVACION")
        );
    }

    #[test]
    fn slug_para_columnas_dinamicas() {
        assert_eq!(slug("MATERNIDAD"), "maternidad");
        assert_eq!(slug("POR ACCIDENTE (P)"), "por_accidente_p");
    }
}
