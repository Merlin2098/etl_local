//! Lectura de datos tipados con nombres canónicos (ADR-0003; PRE-20…PRE-23).

use std::path::Path;

use calamine::Data;
use serde::Serialize;

use crate::celda::Celda;
use crate::contrato::{ColumnasExtra, Contrato, FinDatos, Hoja, TipoDato};
pub use crate::convertir::Valor;
use crate::convertir::{Falla, convertir, nombre_tipo};
use crate::informe::{Informe, Registro};
use crate::libro::Libro;
use crate::normalizar::{normalizar_cabecera, slug};
use crate::preflight::{FechaNombre, HojaUbicada, inspeccionar};

#[derive(Serialize, Debug, Clone)]
pub struct ColumnaLeida {
    pub nombre: String,
    pub tipo: TipoDato,
    pub valores: Vec<Valor>,
}

#[derive(Serialize, Debug, Clone)]
pub struct TablaHoja {
    pub hoja: String,
    /// Número de fila de Excel (1-based) de cada fila conservada.
    pub filas_origen: Vec<u32>,
    pub columnas: Vec<ColumnaLeida>,
}

impl TablaHoja {
    pub fn columna(&self, nombre: &str) -> Option<&ColumnaLeida> {
        self.columnas.iter().find(|c| c.nombre == nombre)
    }

    pub fn filas(&self) -> usize {
        self.filas_origen.len()
    }
}

#[derive(Serialize, Debug, Clone)]
pub struct Lectura {
    pub fuente: String,
    pub archivo: String,
    pub fecha_nombre: Option<FechaNombre>,
    pub hojas: Vec<TablaHoja>,
    /// Solo avisos: si hubiera errores, `leer_archivo` devuelve `Err`.
    pub informe: Informe,
}

/// Valida y lee un archivo. Devuelve `Err` con el informe completo si hay algún error.
pub fn leer_archivo(contrato: &Contrato, ruta: &Path) -> Result<Lectura, Informe> {
    let inspeccion = inspeccionar(contrato, ruta);
    let Some(libro) = inspeccion.libro.as_ref() else {
        return Err(inspeccion.informe);
    };
    if inspeccion.informe.tiene_errores() {
        return Err(inspeccion.informe);
    }
    let archivo = ruta
        .file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
    let mut reg = Registro::con_informe(archivo.clone(), inspeccion.informe.clone());
    let hojas = inspeccion
        .hojas
        .iter()
        .map(|ub| leer_hoja(libro, &contrato.hojas[ub.indice_contrato], ub, &mut reg))
        .collect();
    if reg.informe.tiene_errores() {
        return Err(reg.informe);
    }
    Ok(Lectura {
        fuente: contrato.fuente.clone(),
        archivo,
        fecha_nombre: inspeccion.fecha_nombre,
        hojas,
        informe: reg.informe,
    })
}

/// Columna de salida: de dónde sale y cómo se valida.
struct Salida<'a> {
    nombre: String,
    tipo: TipoDato,
    /// Columna de Excel (1-based); `None` si el archivo no la trae.
    excel: Option<u32>,
    /// Regla del contrato, si la columna es del contrato (no dinámica ni extra).
    regla: Option<&'a crate::contrato::Columna>,
}

fn salidas<'a>(hoja: &'a Hoja, ub: &HojaUbicada) -> Vec<Salida<'a>> {
    let mut v: Vec<Salida> = hoja
        .columnas
        .iter()
        .enumerate()
        .map(|(i, col)| Salida {
            nombre: col.nombre.clone(),
            tipo: col.tipo,
            excel: ub.columnas.iter().find(|(_, k)| *k == i).map(|(c, _)| *c),
            regla: Some(col),
        })
        .collect();
    for (excel, nombre, tipo) in &ub.dinamicas {
        v.push(Salida {
            nombre: nombre.clone(),
            tipo: *tipo,
            excel: Some(*excel),
            regla: None,
        });
    }
    if hoja.columnas_extra == ColumnasExtra::Conservar {
        for (excel, texto) in &ub.sin_mapear {
            let nombre = slug(texto);
            if !nombre.is_empty() && !v.iter().any(|s| s.nombre == nombre) {
                v.push(Salida {
                    nombre,
                    tipo: TipoDato::Texto,
                    excel: Some(*excel),
                    regla: None,
                });
            }
        }
    }
    v
}

fn vacia(libro: &Libro, hoja: &str, fila: u32, columna: u32) -> bool {
    libro.texto(hoja, fila, columna).is_none()
}

/// Estado de la celda clave de una fila (TR-12, decisión TR-D20).
enum EstadoClave {
    Presente,
    EnBlanco,
    /// Valor que equivale a vacío (`#N/A`, `nan`, `None`…): se muestra en el aviso.
    Nula(String),
}

fn estado_clave(libro: &Libro, hoja: &str, fila: u32, salida: &Salida) -> EstadoClave {
    let Some(col) = salida.excel else {
        return EstadoClave::EnBlanco;
    };
    let Some(texto) = libro.texto(hoja, fila, col) else {
        return EstadoClave::EnBlanco;
    };
    let dato = libro.valor(hoja, fila, col).unwrap_or(&Data::Empty);
    let formatos = salida.regla.map_or(&[][..], |r| r.formatos.as_slice());
    match convertir(dato, salida.tipo, formatos) {
        Ok(Valor::Nulo) | Err(Falla::ErrorExcel(_)) => EstadoClave::Nula(texto),
        _ => EstadoClave::Presente,
    }
}

/// Texto de la cabecera de una columna de Excel, o el primer `origen` si no se puede leer.
fn cabecera(libro: &Libro, hoja: &str, ub: &HojaUbicada, col: u32, salida: &Salida) -> String {
    libro
        .texto(hoja, ub.fila_cabecera, col)
        .or_else(|| salida.regla.map(|r| r.origen[0].clone()))
        .unwrap_or_else(|| salida.nombre.clone())
}

/// `1200` → `1.200` (separador de miles en español).
fn miles(n: usize) -> String {
    let digitos = n.to_string();
    let mut salida = String::new();
    for (i, c) in digitos.chars().enumerate() {
        if i > 0 && (digitos.len() - i) % 3 == 0 {
            salida.push('.');
        }
        salida.push(c);
    }
    salida
}

/// `0.0` → `0`; `2.5` → `2.5`.
fn numero(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        n.to_string()
    }
}

/// NOM-BRZ-10: la primera celda es texto y, sin `+`, `-`, `*` ni espacios al inicio, contiene uno de
/// los textos de `filas_resumen` (ya normalizados). Una celda que no es texto nunca es fila de totales.
fn es_fila_resumen(libro: &Libro, hoja: &str, fila: u32, columna: u32, textos: &[String]) -> bool {
    if textos.is_empty() {
        return false;
    }
    let Some(Data::String(s)) = libro.valor(hoja, fila, columna) else {
        return false;
    };
    let limpio =
        normalizar_cabecera(s.trim_start_matches(|c: char| "+-*".contains(c) || c.is_whitespace()));
    textos.iter().any(|t| limpio.contains(t.as_str()))
}

fn leer_hoja(libro: &Libro, hoja: &Hoja, ub: &HojaUbicada, reg: &mut Registro) -> TablaHoja {
    let nombre = ub.nombre_real.as_str();
    let salidas = salidas(hoja, ub);
    let mut todas: Vec<u32> = salidas.iter().filter_map(|s| s.excel).collect();
    todas.extend(ub.sin_mapear.iter().map(|(c, _)| *c));
    let clave = hoja.clave().and_then(|col| {
        salidas
            .iter()
            .find(|s| s.nombre == col.nombre && s.excel.is_some())
    });

    let mut tabla = TablaHoja {
        hoja: nombre.to_string(),
        filas_origen: Vec::new(),
        columnas: salidas
            .iter()
            .map(|s| ColumnaLeida {
                nombre: s.nombre.clone(),
                tipo: s.tipo,
                valores: Vec::new(),
            })
            .collect(),
    };
    // Filas con un dato obligatorio vacío, por columna de salida (PRE-21 se agrupa).
    let mut faltantes: Vec<Vec<u32>> = vec![Vec::new(); salidas.len()];

    let resumen: Vec<String> = hoja
        .filas_resumen
        .iter()
        .map(|t| normalizar_cabecera(t))
        .collect();
    for fila in ub.fila_cabecera + 1..=libro.ultima_fila(nombre) {
        if es_fila_resumen(libro, nombre, fila, hoja.cabecera.columna, &resumen) {
            continue;
        }
        let fila_vacia = todas.iter().all(|c| vacia(libro, nombre, fila, *c));
        let estado = clave.map(|s| estado_clave(libro, nombre, fila, s));
        let sin_clave = !matches!(estado, Some(EstadoClave::Presente));
        match hoja.fin_datos {
            FinDatos::FilaVacia if fila_vacia => break,
            FinDatos::ClaveVacia if (clave.is_some() && sin_clave) || fila_vacia => break,
            _ => {}
        }
        match (clave, estado) {
            (None, _) if fila_vacia => continue,
            (Some(_), Some(EstadoClave::EnBlanco)) if fila_vacia => continue,
            (Some(s), Some(EstadoClave::EnBlanco)) => {
                let col = s.excel.unwrap_or_default();
                let cab = cabecera(libro, nombre, ub, col, s);
                reg.aviso(
                    "PRE-24",
                    Some(nombre),
                    Some(Celda::nueva(fila, col).to_string()),
                    format!("La fila {fila} tiene datos pero no tiene {cab}; no se incluyó."),
                );
                continue;
            }
            (Some(s), Some(EstadoClave::Nula(valor))) => {
                let col = s.excel.unwrap_or_default();
                let cab = cabecera(libro, nombre, ub, col, s);
                reg.aviso(
                    "PRE-24",
                    Some(nombre),
                    Some(Celda::nueva(fila, col).to_string()),
                    format!(
                        "La fila {fila} tiene «{valor}» en {cab}; se tomó como vacía y la fila no se incluyó."
                    ),
                );
                continue;
            }
            _ => {}
        }
        tabla.filas_origen.push(fila);
        for (i, (salida, columna)) in salidas.iter().zip(tabla.columnas.iter_mut()).enumerate() {
            let valor = leer_celda(libro, nombre, fila, salida, reg);
            let obligatoria = salida.regla.is_some_and(|r| !r.nula);
            if valor == Valor::Nulo && obligatoria && salida.excel.is_some() {
                faltantes[i].push(fila);
            }
            columna.valores.push(valor);
        }
    }

    for (salida, filas) in salidas.iter().zip(&faltantes) {
        let (Some(col), Some(primera)) = (salida.excel, filas.first()) else {
            continue;
        };
        let cab = cabecera(libro, nombre, ub, col, salida);
        let celdas: Vec<String> = filas
            .iter()
            .map(|f| Celda::nueva(*f, col).to_string())
            .collect();
        let mensaje = match celdas.len() {
            1 => format!("Falta {cab} en la fila {primera}."),
            n if n <= 4 => format!("Falta {cab} en {n} filas ({}).", celdas.join(", ")),
            n => format!(
                "Falta {cab} en {} filas ({}… {}).",
                miles(n),
                celdas[..3].join(", "),
                celdas[n - 1]
            ),
        };
        reg.error("PRE-21", Some(nombre), Some(celdas[0].clone()), mensaje);
    }
    tabla
}

fn leer_celda(libro: &Libro, hoja: &str, fila: u32, salida: &Salida, reg: &mut Registro) -> Valor {
    let Some(col) = salida.excel else {
        return Valor::Nulo;
    };
    let celda = Celda::nueva(fila, col);
    let dato = libro.valor(hoja, fila, col).unwrap_or(&Data::Empty);
    let formatos = salida.regla.map_or(&[][..], |r| r.formatos.as_slice());
    let valor = match convertir(dato, salida.tipo, formatos) {
        Ok(v) => v,
        Err(Falla::ErrorExcel(e)) => {
            reg.aviso(
                "PRE-22",
                Some(hoja),
                Some(celda.to_string()),
                format!("La celda tiene un error de Excel ({e}); se tomó como vacía."),
            );
            Valor::Nulo
        }
        Err(Falla::NoConvertible(t)) => {
            reg.aviso(
                "PRE-20",
                Some(hoja),
                Some(celda.to_string()),
                format!("No se pudo leer «{t}» como {}.", nombre_tipo(salida.tipo)),
            );
            Valor::Nulo
        }
    };
    let Some(regla) = salida.regla else {
        return valor;
    };
    let fuera_de_rango =
        |n: f64| regla.minimo.is_some_and(|m| n < m) || regla.maximo.is_some_and(|m| n > m);
    let rango = match (regla.minimo, regla.maximo) {
        (Some(a), Some(b)) => format!("entre {} y {}", numero(a), numero(b)),
        (Some(a), None) => format!("mínimo {}", numero(a)),
        (None, Some(b)) => format!("máximo {}", numero(b)),
        (None, None) => String::new(),
    };
    let fuera = match &valor {
        Valor::Texto(t)
            if !regla.valores.is_empty()
                && !regla
                    .valores
                    .iter()
                    .any(|v| normalizar_cabecera(v) == normalizar_cabecera(t)) =>
        {
            Some(format!(
                "El valor «{t}» no es uno de los esperados ({}).",
                regla.valores.join(", ")
            ))
        }
        Valor::Entero(n) if fuera_de_rango(*n as f64) => Some(format!(
            "El valor «{n}» está fuera del rango permitido ({rango})."
        )),
        Valor::Decimal(n) if fuera_de_rango(*n) => Some(format!(
            "El valor «{}» está fuera del rango permitido ({rango}).",
            numero(*n)
        )),
        _ => None,
    };
    if let Some(mensaje) = fuera {
        reg.aviso("PRE-23", Some(hoja), Some(celda.to_string()), mensaje);
    }
    valor
}
