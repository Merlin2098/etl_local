//! Conversión de una celda de Excel al tipo canónico de su columna (Plan 3, Tarea 6).

use calamine::{Data, DataType};
use chrono::NaiveDate;
use serde::Serialize;

use crate::contrato::{FormatoFecha, TipoDato};
use crate::preflight::periodo_valido;

/// Valor ya convertido al tipo de su columna.
#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum Valor {
    Nulo,
    Texto(String),
    Entero(i64),
    Decimal(f64),
    Fecha(NaiveDate),
    Booleano(bool),
}

/// Por qué una celda no produjo un valor.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Falla {
    /// Celda con error de Excel (`#N/A`) o su texto (`#N/D`): TR-12, PRE-22.
    ErrorExcel(String),
    /// El contenido no se puede leer como el tipo de la columna: PRE-20.
    NoConvertible(String),
}

/// Textos que equivalen a vacío (TR-12).
const VACIOS: [&str; 3] = ["NONE", "NAN", "NAT"];
/// Textos de error de Excel guardados como texto (TR-12).
const ERRORES_TEXTO: [&str; 2] = ["#N/D", "#N/A"];

/// Nombre del tipo para los mensajes al usuario.
pub(crate) fn nombre_tipo(tipo: TipoDato) -> &'static str {
    match tipo {
        TipoDato::Texto => "texto",
        TipoDato::Entero => "número entero",
        TipoDato::Decimal => "número",
        TipoDato::Fecha => "fecha",
        TipoDato::Periodo => "periodo (AAAA-MM)",
        TipoDato::Documento => "número de documento",
        TipoDato::Booleano => "sí o no",
    }
}

fn entero_exacto(f: f64) -> Option<i64> {
    (f.fract() == 0.0 && f.abs() < 9.0e15).then_some(f as i64)
}

/// Número decimal escrito como texto; rechaza `inf` y `NaN`.
fn decimal_finito(t: &str) -> Option<f64> {
    t.parse::<f64>().ok().filter(|f| f.is_finite())
}

/// Documento escrito como texto: recorta, quita una parte decimal de solo ceros (`12345678.00`) y
/// resuelve la notación científica de Excel (`1.2345678E7`). Sin relleno de ceros a la izquierda.
fn limpiar_documento(t: &str) -> String {
    let t = t.trim();
    if let Some((entera, fraccion)) = t.split_once('.')
        && !entera.is_empty()
        && entera.chars().all(|c| c.is_ascii_digit())
        && !fraccion.is_empty()
        && fraccion.chars().all(|c| c == '0')
    {
        return entera.to_string();
    }
    if t.contains(['e', 'E'])
        && let Some(n) = decimal_finito(t).and_then(entero_exacto)
    {
        return n.to_string();
    }
    t.to_string()
}

/// Muestra un decimal como lo haría Excel: hasta 15 cifras significativas, sin ceros sobrantes.
fn decimal_como_excel(f: f64) -> String {
    if let Some(n) = entero_exacto(f) {
        return n.to_string();
    }
    let magnitud = f.abs().log10().floor() as i32;
    let decimales = (14 - magnitud).clamp(0, 20) as usize;
    let texto = format!("{f:.decimales$}");
    texto
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

/// Convierte una celda. `formatos` solo aplica a `fecha` (vacío = solo fecha de Excel).
pub(crate) fn convertir(
    dato: &Data,
    tipo: TipoDato,
    formatos: &[FormatoFecha],
) -> Result<Valor, Falla> {
    let no = || Falla::NoConvertible(dato.to_string().trim().to_string());
    let texto = match dato {
        Data::Empty => return Ok(Valor::Nulo),
        Data::Error(e) => return Err(Falla::ErrorExcel(e.to_string())),
        Data::String(s) => {
            let t = s.trim();
            let mayus = t.to_uppercase();
            if t.is_empty() || VACIOS.contains(&mayus.as_str()) {
                return Ok(Valor::Nulo);
            }
            if ERRORES_TEXTO.contains(&mayus.as_str()) {
                return Err(Falla::ErrorExcel(t.to_string()));
            }
            Some(t)
        }
        _ => None,
    };

    match tipo {
        TipoDato::Texto => Ok(Valor::Texto(match (dato, texto) {
            (_, Some(t)) => t.to_string(),
            (Data::Float(f), _) => decimal_como_excel(*f),
            (Data::DateTime(_), _) => dato
                .as_date()
                .ok_or_else(no)?
                .format("%Y-%m-%d")
                .to_string(),
            _ => dato.to_string(),
        })),
        TipoDato::Documento => match (dato, texto) {
            (_, Some(t)) => Ok(Valor::Texto(limpiar_documento(t))),
            (Data::Int(n), _) => Ok(Valor::Texto(n.to_string())),
            (Data::Float(f), _) => entero_exacto(*f)
                .map(|n| Valor::Texto(n.to_string()))
                .ok_or_else(no),
            _ => Err(no()),
        },
        TipoDato::Entero => match (dato, texto) {
            (_, Some(t)) => t
                .parse::<i64>()
                .ok()
                .or_else(|| decimal_finito(t).and_then(entero_exacto))
                .map(Valor::Entero)
                .ok_or_else(no),
            (Data::Int(n), _) => Ok(Valor::Entero(*n)),
            (Data::Float(f), _) => entero_exacto(*f).map(Valor::Entero).ok_or_else(no),
            _ => Err(no()),
        },
        TipoDato::Decimal => match (dato, texto) {
            (_, Some(t)) => decimal_finito(t).map(Valor::Decimal).ok_or_else(no),
            (Data::Int(n), _) => Ok(Valor::Decimal(*n as f64)),
            (Data::Float(f), _) => Ok(Valor::Decimal(*f)),
            _ => Err(no()),
        },
        TipoDato::Fecha => {
            let excel = formatos.is_empty() || formatos.contains(&FormatoFecha::Excel);
            match (dato, texto) {
                (_, Some(t)) => formatos
                    .iter()
                    .find_map(|f| match f {
                        FormatoFecha::DiaMesAnio => NaiveDate::parse_from_str(t, "%d/%m/%Y").ok(),
                        FormatoFecha::Iso => NaiveDate::parse_from_str(t, "%Y-%m-%d").ok(),
                        FormatoFecha::Excel => None,
                    })
                    .map(Valor::Fecha)
                    .ok_or_else(no),
                (Data::DateTime(_) | Data::DateTimeIso(_), _) if excel => {
                    dato.as_date().map(Valor::Fecha).ok_or_else(no)
                }
                _ => Err(no()),
            }
        }
        TipoDato::Periodo => match (dato, texto) {
            (_, Some(t)) if periodo_valido(t) => Ok(Valor::Texto(t.to_string())),
            (Data::DateTime(_) | Data::DateTimeIso(_), _) => dato
                .as_date()
                .map(|d| Valor::Texto(d.format("%Y-%m").to_string()))
                .ok_or_else(no),
            _ => Err(no()),
        },
        TipoDato::Booleano => match (dato, texto) {
            (_, Some(t)) => match t.to_uppercase().as_str() {
                "SI" | "SÍ" | "TRUE" | "1" => Ok(Valor::Booleano(true)),
                "NO" | "FALSE" | "0" => Ok(Valor::Booleano(false)),
                _ => Err(no()),
            },
            (Data::Bool(b), _) => Ok(Valor::Booleano(*b)),
            (Data::Int(n), _) if *n == 0 || *n == 1 => Ok(Valor::Booleano(*n == 1)),
            (Data::Float(f), _) if *f == 0.0 || *f == 1.0 => Ok(Valor::Booleano(*f == 1.0)),
            _ => Err(no()),
        },
    }
}

#[cfg(test)]
mod tests {
    use calamine::{CellErrorType, ExcelDateTime, ExcelDateTimeType};

    use super::*;

    fn s(t: &str) -> Data {
        Data::String(t.into())
    }

    fn fecha(a: i32, m: u32, d: u32) -> Valor {
        Valor::Fecha(NaiveDate::from_ymd_opt(a, m, d).unwrap())
    }

    /// 2025-11-12 como número de serie de Excel.
    fn celda_fecha() -> Data {
        Data::DateTime(ExcelDateTime::new(
            45973.0,
            ExcelDateTimeType::DateTime,
            false,
        ))
    }

    #[test]
    fn vacios_y_errores() {
        for t in ["", "  ", "None", "nan", "NaT"] {
            assert_eq!(
                convertir(&s(t), TipoDato::Texto, &[]),
                Ok(Valor::Nulo),
                "{t}"
            );
        }
        assert_eq!(
            convertir(&Data::Empty, TipoDato::Entero, &[]),
            Ok(Valor::Nulo)
        );
        assert!(matches!(
            convertir(&Data::Error(CellErrorType::NA), TipoDato::Texto, &[]),
            Err(Falla::ErrorExcel(_))
        ));
        assert!(matches!(
            convertir(&s("#N/D"), TipoDato::Texto, &[]),
            Err(Falla::ErrorExcel(_))
        ));
    }

    #[test]
    fn texto() {
        assert_eq!(
            convertir(&s(" Ana "), TipoDato::Texto, &[]),
            Ok(Valor::Texto("Ana".into()))
        );
        assert_eq!(
            convertir(&Data::Float(12.0), TipoDato::Texto, &[]),
            Ok(Valor::Texto("12".into()))
        );
        assert_eq!(
            convertir(&Data::Float(1.5), TipoDato::Texto, &[]),
            Ok(Valor::Texto("1.5".into()))
        );
        assert_eq!(
            convertir(&celda_fecha(), TipoDato::Texto, &[]),
            Ok(Valor::Texto("2025-11-12".into()))
        );
    }

    #[test]
    fn documento() {
        let t = |v: &str| Ok(Valor::Texto(v.into()));
        assert_eq!(
            convertir(&Data::Float(99123456.0), TipoDato::Documento, &[]),
            t("99123456")
        );
        assert_eq!(
            convertir(&Data::Int(99123456), TipoDato::Documento, &[]),
            t("99123456")
        );
        assert_eq!(
            convertir(&s("99123456.0"), TipoDato::Documento, &[]),
            t("99123456")
        );
        assert_eq!(
            convertir(&s("00123456"), TipoDato::Documento, &[]),
            t("00123456")
        );
        assert!(convertir(&Data::Float(1.5), TipoDato::Documento, &[]).is_err());
    }

    #[test]
    fn numeros() {
        assert_eq!(
            convertir(&Data::Float(3.0), TipoDato::Entero, &[]),
            Ok(Valor::Entero(3))
        );
        assert_eq!(
            convertir(&s("4"), TipoDato::Entero, &[]),
            Ok(Valor::Entero(4))
        );
        assert_eq!(
            convertir(&s("4.0"), TipoDato::Entero, &[]),
            Ok(Valor::Entero(4))
        );
        assert!(convertir(&Data::Float(3.5), TipoDato::Entero, &[]).is_err());
        assert!(convertir(&s("tres"), TipoDato::Entero, &[]).is_err());
        assert_eq!(
            convertir(&Data::Int(2), TipoDato::Decimal, &[]),
            Ok(Valor::Decimal(2.0))
        );
        assert_eq!(
            convertir(&s("2.5"), TipoDato::Decimal, &[]),
            Ok(Valor::Decimal(2.5))
        );
        assert!(convertir(&s("2,5"), TipoDato::Decimal, &[]).is_err());
    }

    #[test]
    fn fechas() {
        let ambos = [FormatoFecha::Excel, FormatoFecha::DiaMesAnio];
        assert_eq!(
            convertir(&celda_fecha(), TipoDato::Fecha, &[]),
            Ok(fecha(2025, 11, 12))
        );
        assert_eq!(
            convertir(&s("12/11/2025"), TipoDato::Fecha, &ambos),
            Ok(fecha(2025, 11, 12))
        );
        assert_eq!(
            convertir(&s("2025-11-12"), TipoDato::Fecha, &[FormatoFecha::Iso]),
            Ok(fecha(2025, 11, 12))
        );
        assert!(convertir(&s("31/02/2025"), TipoDato::Fecha, &ambos).is_err());
        // Texto sin formato de texto declarado: no se adivina.
        assert!(convertir(&s("12/11/2025"), TipoDato::Fecha, &[]).is_err());
        // Solo DD/MM/AAAA: una celda de fecha de Excel no se acepta.
        assert!(convertir(&celda_fecha(), TipoDato::Fecha, &[FormatoFecha::DiaMesAnio]).is_err());
    }

    #[test]
    fn periodo() {
        assert_eq!(
            convertir(&s("2025-11"), TipoDato::Periodo, &[]),
            Ok(Valor::Texto("2025-11".into()))
        );
        assert_eq!(
            convertir(&celda_fecha(), TipoDato::Periodo, &[]),
            Ok(Valor::Texto("2025-11".into()))
        );
        assert!(convertir(&s("2025-13"), TipoDato::Periodo, &[]).is_err());
    }

    #[test]
    fn documento_con_ceros_o_notacion_cientifica() {
        let t = |v: &str| Ok(Valor::Texto(v.into()));
        assert_eq!(
            convertir(&s("12345678.00"), TipoDato::Documento, &[]),
            t("12345678")
        );
        assert_eq!(
            convertir(&s("1.2345678E7"), TipoDato::Documento, &[]),
            t("12345678")
        );
        assert_eq!(
            convertir(&s("CE-001234"), TipoDato::Documento, &[]),
            t("CE-001234")
        );
    }

    #[test]
    fn numeros_no_finitos() {
        // "NaN" no aparece: por TR-12 es vacío, no un número inválido.
        for t in ["inf", "-inf", "infinity", "1e999"] {
            assert!(convertir(&s(t), TipoDato::Decimal, &[]).is_err(), "{t}");
            assert!(convertir(&s(t), TipoDato::Entero, &[]).is_err(), "{t}");
        }
    }

    #[test]
    fn texto_desde_decimal_como_excel() {
        let t = |v: &str| Ok(Valor::Texto(v.into()));
        assert_eq!(
            convertir(&Data::Float(0.1 + 0.2), TipoDato::Texto, &[]),
            t("0.3")
        );
        assert_eq!(
            convertir(&Data::Float(1234.5678), TipoDato::Texto, &[]),
            t("1234.5678")
        );
        assert_eq!(
            convertir(&Data::Float(-0.000125), TipoDato::Texto, &[]),
            t("-0.000125")
        );
    }

    #[test]
    fn booleano() {
        assert_eq!(
            convertir(&s("Sí"), TipoDato::Booleano, &[]),
            Ok(Valor::Booleano(true))
        );
        assert_eq!(
            convertir(&s("no"), TipoDato::Booleano, &[]),
            Ok(Valor::Booleano(false))
        );
        assert_eq!(
            convertir(&Data::Bool(true), TipoDato::Booleano, &[]),
            Ok(Valor::Booleano(true))
        );
        assert!(convertir(&s("quizá"), TipoDato::Booleano, &[]).is_err());
    }
}
