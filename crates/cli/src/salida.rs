//! Procesamiento por archivo y formato de salida (texto o JSON).

use std::path::PathBuf;

use etl_local_core::contrato::Contrato;
use etl_local_core::informe::{Informe, Problema, Severidad};
use etl_local_core::lectura::{Lectura, leer_archivo};
use etl_local_core::preflight::{FechaNombre, validar_archivo};
use serde_json::json;

pub struct Resultado {
    pub archivo: String,
    pub informe: Informe,
    pub lectura: Option<Lectura>,
}

pub fn procesar(contrato: &Contrato, archivos: &[PathBuf], leer: bool) -> Vec<Resultado> {
    archivos
        .iter()
        .map(|ruta| {
            let archivo = ruta.display().to_string();
            if !leer {
                let informe = validar_archivo(contrato, ruta);
                return Resultado {
                    archivo,
                    informe,
                    lectura: None,
                };
            }
            match leer_archivo(contrato, ruta) {
                Ok(lectura) => Resultado {
                    archivo,
                    informe: lectura.informe.clone(),
                    lectura: Some(lectura),
                },
                Err(informe) => Resultado {
                    archivo,
                    informe,
                    lectura: None,
                },
            }
        })
        .collect()
}

pub(crate) fn linea(p: &Problema) -> String {
    let lugar = [
        Some(p.archivo.as_str()),
        p.hoja.as_deref(),
        p.celda.as_deref(),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" · ");
    let prefijo = if p.severidad == Severidad::Aviso {
        "Aviso: "
    } else {
        ""
    };
    format!("{prefijo}{lugar}: {} [{}]", p.mensaje, p.codigo)
}

fn fecha(f: &FechaNombre) -> String {
    match f {
        FechaNombre::Periodo(p) => format!("periodo {p}"),
        FechaNombre::FechaCorte(d) => format!("fecha de corte {}", d.format("%d/%m/%Y")),
    }
}

pub fn humana(resultados: &[Resultado], leer: bool) -> String {
    let mut lineas = Vec::new();
    for r in resultados {
        if leer && let Some(l) = &r.lectura {
            let cuando = l.fecha_nombre.as_ref().map(fecha).unwrap_or_default();
            lineas.push(format!("{} {cuando}", l.archivo).trim_end().to_string());
            for h in &l.hojas {
                lineas.push(format!(
                    "  {}: {} filas, {} columnas",
                    h.hoja,
                    h.filas(),
                    h.columnas.len()
                ));
            }
        }
        lineas.extend(r.informe.problemas.iter().map(linea));
    }
    let con_errores = resultados
        .iter()
        .filter(|r| r.informe.tiene_errores())
        .count();
    let avisos: usize = resultados.iter().map(|r| r.informe.avisos().count()).sum();
    let revisados = match resultados.len() {
        1 => "1 archivo revisado".to_string(),
        n => format!("{n} archivos revisados"),
    };
    let avisos = if avisos == 1 {
        "1 aviso".to_string()
    } else {
        format!("{avisos} avisos")
    };
    lineas.push(format!("{revisados}: {con_errores} con errores, {avisos}."));
    lineas.join("\n")
}

pub fn json(resultados: &[Resultado], leer: bool) -> String {
    let lista: Vec<_> = resultados
        .iter()
        .map(|r| {
            if !leer {
                return json!({ "archivo": r.archivo, "informe": r.informe });
            }
            let (fecha_nombre, hojas) = match &r.lectura {
                Some(l) => (
                    json!(l.fecha_nombre),
                    l.hojas
                        .iter()
                        .map(|h| {
                            json!({
                                "hoja": h.hoja,
                                "filas": h.filas(),
                                "columnas": h.columnas.iter().map(|c| &c.nombre).collect::<Vec<_>>(),
                            })
                        })
                        .collect::<Vec<_>>(),
                ),
                None => (serde_json::Value::Null, Vec::new()),
            };
            json!({ "archivo": r.archivo, "fecha_nombre": fecha_nombre, "hojas": hojas, "informe": r.informe })
        })
        .collect();
    serde_json::to_string_pretty(&lista).expect("JSON serializable")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn aviso() -> Problema {
        Problema {
            codigo: "PRE-22".into(),
            severidad: Severidad::Aviso,
            archivo: "a.xlsx".into(),
            hoja: None,
            celda: None,
            celda_encontrada: None,
            mensaje: "m".into(),
            detalle: None,
        }
    }

    #[test]
    fn resumen_en_singular() {
        let r = Resultado {
            archivo: "a.xlsx".into(),
            informe: Informe {
                problemas: vec![aviso()],
            },
            lectura: None,
        };
        let texto = humana(&[r], false);
        assert!(
            texto.ends_with("1 archivo revisado: 0 con errores, 1 aviso."),
            "{texto}"
        );
    }
}
