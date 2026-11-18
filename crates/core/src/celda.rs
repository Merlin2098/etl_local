//! Referencias de celda estilo Excel (`A6`), 1-based.

use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Celda {
    pub fila: u32,
    pub columna: u32,
}

impl Celda {
    pub fn nueva(fila: u32, columna: u32) -> Self {
        Self { fila, columna }
    }
}

/// Convierte un número de columna 1-based en letras (`1` → `A`, `27` → `AA`).
pub fn letras_columna(mut columna: u32) -> String {
    let mut letras = Vec::new();
    while columna > 0 {
        let resto = (columna - 1) % 26;
        letras.push(char::from(b'A' + resto as u8));
        columna = (columna - 1) / 26;
    }
    letras.iter().rev().collect()
}

impl FromStr for Celda {
    type Err = String;
    fn from_str(texto: &str) -> Result<Self, Self::Err> {
        let invalida = || format!("«{texto}» no es una celda válida (ejemplo: A6)");
        let corte = texto
            .find(|c: char| !c.is_ascii_alphabetic())
            .ok_or_else(invalida)?;
        let (letras, numero) = texto.split_at(corte);
        if letras.is_empty() || letras.len() > 3 || !numero.chars().all(|c| c.is_ascii_digit()) {
            return Err(invalida());
        }
        let fila: u32 = numero.parse().map_err(|_| invalida())?;
        if fila == 0 {
            return Err(invalida());
        }
        let columna = letras.bytes().fold(0u32, |acc, b| {
            acc * 26 + u32::from(b.to_ascii_uppercase() - b'A' + 1)
        });
        Ok(Celda::nueva(fila, columna))
    }
}

impl fmt::Display for Celda {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", letras_columna(self.columna), self.fila)
    }
}

impl<'de> serde::Deserialize<'de> for Celda {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let texto = String::deserialize(d)?;
        texto.parse().map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn celda_ida_y_vuelta() {
        assert_eq!("A6".parse::<Celda>().unwrap(), Celda::nueva(6, 1));
        assert_eq!("AA10".parse::<Celda>().unwrap(), Celda::nueva(10, 27));
        assert_eq!(Celda::nueva(2, 2).to_string(), "B2");
        for malo in ["", "6A", "A0", "a6x", "A"] {
            assert!(malo.parse::<Celda>().is_err(), "{malo}");
        }
    }
}
