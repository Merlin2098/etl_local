//! Validación previa de un archivo fuente contra su contrato
//! (PRE-05…PRE-11 de `validacion-preflight.md`, más TR-D13 y PRE-19).

use std::path::Path;

use chrono::NaiveDate;
use regex::Regex;
use serde::Serialize;

use crate::celda::Celda;
use crate::contrato::validar::patron_completo;
use crate::contrato::{Columna, Contrato, FormatoFechaNombre, Hoja, TipoDato};
use crate::informe::{Informe, Registro};
use crate::libro::Libro;
use crate::normalizar::{normalizar_cabecera, slug};

/// Columnas que se escanean a la derecha del inicio de la cabecera (PRE-11).
const MAX_COLUMNAS: u32 = 250;
/// Celdas vacías seguidas que terminan la cabecera (PRE-11).
const VACIAS_FIN: u32 = 3;
/// Radio de búsqueda de una cabecera desplazada (TR-D13).
const RADIO_DESPLAZAMIENTO: i64 = 3;

/// Fecha extraída del nombre del archivo (TR-D3).
#[derive(schemars::JsonSchema, Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FechaNombre {
    Periodo(String),
    FechaCorte(NaiveDate),
}

/// Valida un archivo contra su contrato y devuelve todos los problemas encontrados.
pub fn validar_archivo(contrato: &Contrato, ruta: &Path) -> Informe {
    inspeccionar(contrato, ruta).informe
}

pub(crate) struct Inspeccion {
    pub informe: Informe,
    pub fecha_nombre: Option<FechaNombre>,
    pub libro: Option<Libro>,
    pub hojas: Vec<HojaUbicada>,
}

/// Hoja encontrada y cabecera resuelta: qué columna de Excel alimenta cada columna del contrato.
pub(crate) struct HojaUbicada {
    pub indice_contrato: usize,
    pub nombre_real: String,
    pub fila_cabecera: u32,
    /// (columna de Excel 1-based, índice en `Hoja::columnas`)
    pub columnas: Vec<(u32, usize)>,
    /// (columna de Excel 1-based, nombre canónico, tipo)
    pub dinamicas: Vec<(u32, String, TipoDato)>,
    /// Cabeceras no mapeadas: (columna de Excel 1-based, texto original)
    pub sin_mapear: Vec<(u32, String)>,
}

pub(crate) fn inspeccionar(contrato: &Contrato, ruta: &Path) -> Inspeccion {
    let archivo = ruta.file_name().map_or_else(
        || ruta.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    );
    let mut reg = Registro::nuevo(archivo.clone());
    let resultado = |reg: Registro, fecha, libro, hojas| Inspeccion {
        informe: reg.informe,
        fecha_nombre: fecha,
        libro,
        hojas,
    };

    if ruta.is_dir() {
        reg.error(
            "PRE-05",
            None,
            None,
            "Es una carpeta, no un archivo: elija el archivo de Excel que está dentro.".into(),
        );
        return resultado(reg, None, None, Vec::new());
    }
    if !ruta.is_file() {
        reg.error("PRE-05", None, None, "El archivo no existe.".into());
        return resultado(reg, None, None, Vec::new());
    }
    let extension = ruta
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    if !contrato.archivo.extensiones.contains(&extension) {
        let esperadas = contrato
            .archivo
            .extensiones
            .iter()
            .map(|e| format!(".{e}"))
            .collect::<Vec<_>>()
            .join(", ");
        let actual = if extension.is_empty() {
            "El archivo no tiene extensión".to_string()
        } else {
            format!("El archivo es .{extension}")
        };
        reg.error(
            "PRE-06",
            None,
            None,
            format!("{actual}; se esperaba {esperadas}."),
        );
        return resultado(reg, None, None, Vec::new());
    }
    let fecha_nombre = revisar_nombre(contrato, &archivo, &mut reg);
    let mut libro = match Libro::abrir(ruta) {
        Ok(libro) => libro,
        Err(detalle) => {
            let mensaje = motivo_apertura(ruta, &archivo, &extension);
            reg.error_con_detalle("PRE-08", None, None, mensaje, detalle);
            return resultado(reg, fecha_nombre, None, Vec::new());
        }
    };

    let mut hojas = Vec::new();
    for (indice, hoja) in contrato.hojas.iter().enumerate() {
        let nombres = hoja.nombre.lista();
        let Some(nombre_real) = nombres.iter().find(|n| libro.tiene_hoja(n)) else {
            reg.error(
                "PRE-09",
                Some(nombres[0]),
                None,
                format!("Falta la hoja «{}».", nombres[0]),
            );
            continue;
        };
        // Solo se cargan las hojas del contrato: una hoja dañada que no se usa no molesta.
        if let Err(detalle) = libro.cargar(nombre_real) {
            reg.error_con_detalle(
                "PRE-08",
                Some(nombre_real),
                None,
                format!("No se pudo leer la hoja «{nombre_real}»; puede estar dañada."),
                detalle,
            );
            continue;
        }
        if let Some(ubicada) = ubicar_hoja(&libro, hoja, indice, nombre_real, &mut reg) {
            hojas.push(ubicada);
        }
    }
    resultado(reg, fecha_nombre, Some(libro), hojas)
}

/// PRE-08: explica por qué no se pudo abrir el libro, mirando el nombre y los primeros bytes.
fn motivo_apertura(ruta: &Path, archivo: &str, extension: &str) -> String {
    const ZIP: &[u8] = b"PK\x03\x04";
    const OLE: &[u8] = &[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
    let rehacer = "Ábralo en Excel y guárdelo como «Libro de Excel (.xlsx)».";
    if archivo.starts_with("~$") {
        return "Es un archivo temporal que Excel crea mientras el libro está abierto. \
                Cierre Excel y use el archivo original."
            .into();
    }
    let mut inicio = [0u8; 512];
    let leidos = std::fs::File::open(ruta)
        .and_then(|mut f| std::io::Read::read(&mut f, &mut inicio))
        .unwrap_or(0);
    let inicio = &inicio[..leidos];
    let texto = String::from_utf8_lossy(inicio);
    let texto = texto
        .trim_start_matches('\u{feff}')
        .trim_start()
        .to_lowercase();
    if texto.starts_with('<') {
        return format!("El archivo es una página web guardada con extensión de Excel. {rehacer}");
    }
    let es_zip = inicio.starts_with(ZIP);
    let es_ole = inicio.starts_with(OLE);
    if (es_zip && extension == "xls") || (es_ole && extension != "xls") {
        return format!("El contenido no corresponde a la extensión .{extension}. {rehacer}");
    }
    "No se pudo abrir el archivo: puede estar dañado o abierto en otro programa.".into()
}

/// PRE-07: patrón del nombre (coincidencia completa) y fecha embebida.
fn revisar_nombre(contrato: &Contrato, archivo: &str, reg: &mut Registro) -> Option<FechaNombre> {
    match leer_nombre(contrato, archivo) {
        Ok(fecha) => fecha,
        Err(mensaje) => {
            reg.error("PRE-07", None, None, mensaje);
            None
        }
    }
}

/// Fecha que trae el nombre del archivo (TR-D3), sin abrirlo. `None` si el contrato no la pide o el nombre
/// no la trae (o trae una fecha imposible).
pub fn fecha_en_nombre(contrato: &Contrato, nombre_archivo: &str) -> Option<FechaNombre> {
    leer_nombre(contrato, nombre_archivo).ok().flatten()
}

/// Revisa el nombre contra el contrato. `Err` lleva el mensaje de PRE-07.
fn leer_nombre(contrato: &Contrato, archivo: &str) -> Result<Option<FechaNombre>, String> {
    let Some(patron) = contrato.archivo.patron_nombre.as_deref() else {
        return Ok(None);
    };
    let regex = patron_completo(patron).expect("patrón validado al cargar el contrato");
    let Some(captura) = regex.captures(archivo) else {
        return Err(format!(
            "El nombre del archivo no tiene el formato esperado para {}.",
            contrato.titulo
        ));
    };
    let Some(regla) = contrato.archivo.fecha_en_nombre.as_ref() else {
        return Ok(None);
    };
    let Some(texto) = captura.name(&regla.grupo).map(|m| m.as_str()) else {
        return Ok(None);
    };
    let fecha = match regla.formato {
        FormatoFechaNombre::AnioMes => {
            periodo_valido(texto).then(|| FechaNombre::Periodo(texto.into()))
        }
        FormatoFechaNombre::DiaMesAnio => NaiveDate::parse_from_str(texto, "%d.%m.%Y")
            .ok()
            .map(FechaNombre::FechaCorte),
    };
    match fecha {
        Some(f) => Ok(Some(f)),
        None => Err(format!(
            "La fecha del nombre ({texto}) no existe en el calendario."
        )),
    }
}

/// `AAAA-MM` con mes 1–12.
pub(crate) fn periodo_valido(texto: &str) -> bool {
    let Some((anio, mes)) = texto.split_once('-') else {
        return false;
    };
    anio.len() == 4
        && mes.len() == 2
        && anio.chars().all(|c| c.is_ascii_digit())
        && mes.parse::<u32>().is_ok_and(|m| (1..=12).contains(&m))
}

/// Lee la fila de cabecera desde una celda: hasta 250 columnas, y termina tras 3 vacías seguidas.
fn escanear(libro: &Libro, hoja: &str, fila: u32, columna: u32) -> Vec<(u32, String)> {
    let mut cabeceras = Vec::new();
    let mut vacias = 0;
    for c in columna..=columna + MAX_COLUMNAS {
        match libro.texto(hoja, fila, c) {
            Some(texto) => {
                vacias = 0;
                cabeceras.push((c, texto));
            }
            None if !cabeceras.is_empty() => {
                vacias += 1;
                if vacias >= VACIAS_FIN {
                    break;
                }
            }
            None => {}
        }
    }
    cabeceras
}

fn columna_presente(columna: &Columna, normalizadas: &[String]) -> bool {
    columna
        .origen
        .iter()
        .any(|o| normalizadas.contains(&normalizar_cabecera(o)))
}

fn requeridas_faltantes<'a>(hoja: &'a Hoja, cabeceras: &[(u32, String)]) -> Vec<&'a Columna> {
    let normalizadas: Vec<String> = cabeceras
        .iter()
        .map(|(_, t)| normalizar_cabecera(t))
        .collect();
    hoja.columnas
        .iter()
        .filter(|c| c.requerida && !columna_presente(c, &normalizadas))
        .collect()
}

/// TR-D13: busca la cabecera completa en ±3 filas y columnas, de la más cercana a la más lejana.
fn buscar_desplazada(libro: &Libro, nombre: &str, hoja: &Hoja) -> Option<Celda> {
    if !hoja.columnas.iter().any(|c| c.requerida) {
        return None;
    }
    let base = hoja.cabecera;
    let r = RADIO_DESPLAZAMIENTO;
    let mut candidatos: Vec<(i64, i64)> = (-r..=r)
        .flat_map(|df| (-r..=r).map(move |dc| (df, dc)))
        .filter(|&d| d != (0, 0))
        .collect();
    candidatos.sort_by_key(|&(df, dc)| (df.abs() + dc.abs(), df, dc));
    candidatos.into_iter().find_map(|(df, dc)| {
        let fila = u32::try_from(i64::from(base.fila) + df)
            .ok()
            .filter(|f| *f >= 1)?;
        let columna = u32::try_from(i64::from(base.columna) + dc)
            .ok()
            .filter(|c| *c >= 1)?;
        libro.texto(nombre, fila, columna)?;
        let cabeceras = escanear(libro, nombre, fila, columna);
        requeridas_faltantes(hoja, &cabeceras)
            .is_empty()
            .then(|| Celda::nueva(fila, columna))
    })
}

/// PRE-10, PRE-11 y PRE-19 para una hoja presente; devuelve el mapeo de columnas si se puede leer.
fn ubicar_hoja(
    libro: &Libro,
    hoja: &Hoja,
    indice: usize,
    nombre: &str,
    reg: &mut Registro,
) -> Option<HojaUbicada> {
    let cab = hoja.cabecera;
    let cabeceras = escanear(libro, nombre, cab.fila, cab.columna);
    let faltantes = requeridas_faltantes(hoja, &cabeceras);
    let ancla_vacia = libro.texto(nombre, cab.fila, cab.columna).is_none();

    if ancla_vacia || !faltantes.is_empty() {
        // Si las columnas obligatorias están donde se espera, solo falta el texto de la primera
        // cabecera: no es un desplazamiento.
        if !faltantes.is_empty()
            && let Some(encontrada) = buscar_desplazada(libro, nombre, hoja)
        {
            reg.error_desplazado(
                "PRE-10",
                Some(nombre),
                cab.to_string(),
                encontrada.to_string(),
                format!(
                    "La cabecera debería empezar en {cab}, pero está en {encontrada}. \
                     ¿Se insertó o borró una fila o columna?"
                ),
            );
            return None;
        }
        if ancla_vacia {
            reg.error(
                "PRE-10",
                Some(nombre),
                Some(cab.to_string()),
                format!("La celda {cab} debería tener la primera cabecera y está vacía."),
            );
        }
        if !faltantes.is_empty() {
            let lista = faltantes
                .iter()
                .map(|c| c.origen[0].as_str())
                .collect::<Vec<_>>()
                .join(", ");
            reg.error(
                "PRE-11",
                Some(nombre),
                Some(format!("fila {}", cab.fila)),
                format!("Faltan columnas obligatorias: {lista}."),
            );
            return None;
        }
    }

    revisar_duplicadas(hoja, nombre, cab.fila, &cabeceras, reg);

    let normalizadas: Vec<String> = cabeceras
        .iter()
        .map(|(_, t)| normalizar_cabecera(t))
        .collect();
    let mut columnas = Vec::new();
    for (i, col) in hoja.columnas.iter().enumerate() {
        let encontrada = col.origen.iter().find_map(|o| {
            let n = normalizar_cabecera(o);
            normalizadas.iter().position(|x| *x == n)
        });
        if let Some(pos) = encontrada {
            columnas.push((cabeceras[pos].0, i));
        }
    }

    let patrones: Vec<(Regex, &str, TipoDato)> = hoja
        .columnas_dinamicas
        .iter()
        .map(|d| {
            let r =
                patron_completo(&d.patron_origen).expect("patrón validado al cargar el contrato");
            (r, d.prefijo.as_str(), d.tipo)
        })
        .collect();
    // Nombre canónico ya usado → columna de Excel que lo alimenta (si el archivo la trae).
    let mut usados: Vec<(String, Option<u32>)> = hoja
        .columnas
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let excel = columnas.iter().find(|(_, k)| *k == i).map(|(e, _)| *e);
            (c.nombre.clone(), excel)
        })
        .collect();
    let mut dinamicas = Vec::new();
    let mut sin_mapear = Vec::new();
    for ((col, texto), normal) in cabeceras.iter().zip(&normalizadas) {
        if columnas.iter().any(|(c, _)| c == col) {
            continue;
        }
        let dinamica = patrones.iter().find_map(|(r, prefijo, tipo)| {
            let motivo = slug(r.captures(normal)?.name("motivo")?.as_str());
            // Un motivo sin letras ni números (`DIAS SUB. (*)`) se agrupa como «otros».
            let motivo = if motivo.is_empty() {
                "otros".to_string()
            } else {
                motivo
            };
            Some((format!("{prefijo}{motivo}"), *tipo))
        });
        let Some((canonico, tipo)) = dinamica else {
            sin_mapear.push((*col, texto.clone()));
            continue;
        };
        if let Some((_, previa)) = usados.iter().find(|(n, _)| *n == canonico) {
            // Dos cabeceras distintas que dan el mismo nombre: sumar o elegir una perdería datos.
            let actual = Celda::nueva(cab.fila, *col);
            let (celda, mensaje) = match previa {
                Some(p) => {
                    let primera = Celda::nueva(cab.fila, *p);
                    (
                        primera,
                        format!(
                            "Las columnas {primera} y {actual} se leen las dos como «{canonico}»;                              deje solo una o renombre una de ellas."
                        ),
                    )
                }
                None => (
                    actual,
                    format!(
                        "La columna «{texto}» se lee como «{canonico}», que ya existe en el contrato."
                    ),
                ),
            };
            reg.error("PRE-19", Some(nombre), Some(celda.to_string()), mensaje);
            continue;
        }
        usados.push((canonico.clone(), Some(*col)));
        dinamicas.push((*col, canonico, tipo));
    }

    Some(HojaUbicada {
        indice_contrato: indice,
        nombre_real: nombre.to_string(),
        fila_cabecera: cab.fila,
        columnas,
        dinamicas,
        sin_mapear,
    })
}

/// PRE-19: cabecera repetida (tras normalizar) que corresponde a una columna del contrato.
fn revisar_duplicadas(
    hoja: &Hoja,
    nombre: &str,
    fila: u32,
    cabeceras: &[(u32, String)],
    reg: &mut Registro,
) {
    let origenes: Vec<String> = hoja
        .columnas
        .iter()
        .flat_map(|c| c.origen.iter().map(|o| normalizar_cabecera(o)))
        .collect();
    let mut vistas: Vec<(String, u32)> = Vec::new();
    for (col, texto) in cabeceras {
        let n = normalizar_cabecera(texto);
        match vistas.iter().find(|(v, _)| *v == n) {
            Some((_, primera)) if origenes.contains(&n) => {
                let c1 = Celda::nueva(fila, *primera);
                let c2 = Celda::nueva(fila, *col);
                reg.error(
                    "PRE-19",
                    Some(nombre),
                    Some(c1.to_string()),
                    format!("La columna «{texto}» aparece dos veces ({c1} y {c2})."),
                );
            }
            Some(_) => {}
            None => vistas.push((n, *col)),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::*;
    use crate::contrato::cargar_contratos;

    fn contrato(fuente: &str) -> Contrato {
        let carpeta = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../config/fuentes");
        cargar_contratos(&carpeta).unwrap().remove(fuente).unwrap()
    }

    fn fixture(relativa: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/demo")
            .join(relativa)
    }

    #[test]
    fn periodo_y_fecha_de_corte_del_nombre() {
        let i = inspeccionar(
            &contrato("planilla"),
            &fixture("limpio/planilla/Planilla 2025-11 Empleados.xlsx"),
        );
        assert_eq!(i.fecha_nombre, Some(FechaNombre::Periodo("2025-11".into())));
        let i = inspeccionar(
            &contrato("base_personal"),
            &fixture("limpio/base_personal/BD. 12.11.2025..xlsx"),
        );
        assert_eq!(
            i.fecha_nombre,
            Some(FechaNombre::FechaCorte(
                NaiveDate::from_ymd_opt(2025, 11, 12).unwrap()
            ))
        );
    }

    #[test]
    fn alias_en_orden() {
        let c = contrato("planilla");
        let hoja = &c.hojas[0];
        let dir = tempfile::tempdir().unwrap();
        let mut wb = rust_xlsxwriter::Workbook::new();
        let ws = wb.add_worksheet();
        ws.set_name("Planilla").unwrap();
        let mut textos: Vec<&str> = hoja
            .columnas
            .iter()
            .filter(|c| c.nombre != "horas_extra_50")
            .map(|c| c.origen[0].as_str())
            .collect();
        textos.extend([
            "HRS SIMPLES AL 50%",
            "#HRS SIMP.50%",
            "DIAS SUB. MATERNIDAD",
        ]);
        for (j, t) in textos.iter().enumerate() {
            ws.write_string(5, j as u16, *t).unwrap();
        }
        let ruta = dir.path().join("Planilla 2025-11 Empleados.xlsx");
        wb.save(&ruta).unwrap();

        let i = inspeccionar(&c, &ruta);
        assert!(!i.informe.tiene_errores(), "{:#?}", i.informe);
        let ub = &i.hojas[0];
        let indice = hoja
            .columnas
            .iter()
            .position(|c| c.nombre == "horas_extra_50")
            .unwrap();
        let columna_excel = ub.columnas.iter().find(|(_, k)| *k == indice).unwrap().0;
        let esperada = textos.iter().position(|t| *t == "#HRS SIMP.50%").unwrap() as u32 + 1;
        assert_eq!(columna_excel, esperada);
        assert_eq!(
            ub.dinamicas
                .iter()
                .map(|d| d.1.as_str())
                .collect::<Vec<_>>(),
            vec!["dias_subsidio_maternidad"]
        );
        // La otra variante queda sin mapear (no es error).
        assert!(ub.sin_mapear.iter().any(|(_, t)| t == "HRS SIMPLES AL 50%"));
    }

    #[test]
    fn dinamicas_que_chocan_son_error() {
        let c = contrato("planilla");
        let dir = tempfile::tempdir().unwrap();
        let mut wb = rust_xlsxwriter::Workbook::new();
        let ws = wb.add_worksheet();
        ws.set_name("Planilla").unwrap();
        let mut textos: Vec<&str> = c.hojas[0]
            .columnas
            .iter()
            .map(|c| c.origen[0].as_str())
            .collect();
        let base = textos.len() as u16;
        textos.extend([
            "DIAS SUB. MATERNIDAD",
            "DIAS SUB.MATERNIDAD",
            "DIAS SUB. (*)",
        ]);
        for (j, t) in textos.iter().enumerate() {
            ws.write_string(5, j as u16, *t).unwrap();
        }
        let ruta = dir.path().join("Planilla 2025-11 Empleados.xlsx");
        wb.save(&ruta).unwrap();

        let i = inspeccionar(&c, &ruta);
        let errores: Vec<_> = i.informe.errores().collect();
        assert_eq!(errores.len(), 1, "{errores:#?}");
        assert_eq!(errores[0].codigo, "PRE-19");
        let primera = Celda::nueva(6, u32::from(base) + 1).to_string();
        let segunda = Celda::nueva(6, u32::from(base) + 2).to_string();
        assert_eq!(errores[0].celda.as_deref(), Some(primera.as_str()));
        assert!(
            errores[0].mensaje.contains(&segunda),
            "{}",
            errores[0].mensaje
        );
        // Un motivo que no deja nombre se agrupa como «otros», nunca como `dias_subsidio_`.
        let nombres: Vec<_> = i.hojas[0].dinamicas.iter().map(|d| d.1.as_str()).collect();
        assert!(nombres.contains(&"dias_subsidio_otros"), "{nombres:?}");
        assert!(!nombres.contains(&"dias_subsidio_"), "{nombres:?}");
    }
}
