//! Lógica de la app sin tipos de Tauri: rutas, reconocimiento y revisión de archivos, resumen de «Listo».
//! Se prueba con `cargo test -p etl-local-app --lib`.

use std::path::{Path, PathBuf};

use etl_local_core::celda::Celda;
use etl_local_core::contrato::{Contrato, cargar_contratos};
use etl_local_core::informe::{Problema, Severidad};
use etl_local_core::preflight::{FechaNombre, fecha_en_nombre};
use etl_local_core::proceso::Ejecucion;
use etl_local_core::reportes::{EstadoReporte, Reporte};
use etl_local_core::revision::{Recorte, recorte, revisar_archivo};
use etl_local_core::tablero::{Filtros, Tablero};
use serde::Serialize;

use crate::ajustes::Ajustes;
use crate::error::ErrorApp;

/// Carpeta de salida por defecto, dentro de Documentos.
pub const CARPETA_SALIDA: &str = "Reportes RR. HH.";

#[derive(Debug, Clone, PartialEq)]
pub struct Rutas {
    pub config: PathBuf,
    pub salida: PathBuf,
}

/// Config: `ETL_LOCAL_CONFIG`, luego los recursos empaquetados y, en debug, `config/` del repo.
/// Salida: `ETL_LOCAL_SALIDA`, luego los ajustes y, por defecto, `Documentos/Reportes RR. HH.`.
pub fn resolver_rutas(
    env_config: Option<PathBuf>,
    env_salida: Option<PathBuf>,
    recursos: &Path,
    documentos: &Path,
    ajustes: &Ajustes,
) -> Rutas {
    let empaquetada = recursos.join("config");
    let config = env_config.unwrap_or_else(|| {
        if empaquetada.join("fuentes").is_dir() || !cfg!(debug_assertions) {
            empaquetada
        } else {
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../config")
        }
    });
    let salida = env_salida
        .or_else(|| ajustes.carpeta_salida.clone())
        .unwrap_or_else(|| documentos.join(CARPETA_SALIDA));
    Rutas { config, salida }
}

/// Texto para la pantalla de inicio: «Documentos › Reportes RR. HH.» o la ruta completa.
pub fn carpeta_visible(salida: &Path, documentos: &Path) -> String {
    match salida.strip_prefix(documentos) {
        Ok(resto) => std::iter::once("Documentos".to_string())
            .chain(
                resto
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy().into_owned()),
            )
            .collect::<Vec<_>>()
            .join(" › "),
        Err(_) => salida.display().to_string(),
    }
}

#[derive(schemars::JsonSchema, Serialize, Debug, Clone, PartialEq)]
pub struct ArchivoElegido {
    pub ruta: String,
    pub nombre: String,
    /// Las dos últimas carpetas, como «Documentos › Planillas».
    pub carpeta: String,
    pub bytes: u64,
    /// Fecha de modificación (`AAAA-MM-DD`).
    pub modificado: Option<String>,
    pub fecha_nombre: Option<FechaNombre>,
}

#[derive(schemars::JsonSchema, Serialize, Debug, Clone, PartialEq)]
pub struct ProblemaVista {
    pub problema: Problema,
    pub recorte: Option<Recorte>,
}

#[derive(schemars::JsonSchema, Serialize, Debug, Clone, PartialEq)]
pub struct RevisionArchivo {
    pub archivo: ArchivoElegido,
    pub personas: Option<u64>,
    pub problemas: Vec<ProblemaVista>,
}

#[derive(schemars::JsonSchema, Serialize, Debug, Clone, PartialEq)]
pub struct TarjetaReporte {
    pub estado: EstadoReporte,
    pub duracion_prevista_ms: Option<u64>,
}

#[derive(schemars::JsonSchema, Serialize, Debug, Clone, PartialEq)]
pub struct Inicio {
    pub tarjetas: Vec<TarjetaReporte>,
    pub carpeta_visible: String,
}

#[derive(schemars::JsonSchema, Serialize, Debug, Clone, PartialEq)]
pub struct Cifra {
    pub valor: String,
    pub etiqueta: String,
}

#[derive(schemars::JsonSchema, Serialize, Debug, Clone)]
pub struct Resultado {
    pub ejecucion: Ejecucion,
    pub titulo_periodos: String,
    pub cifras: Vec<Cifra>,
    pub para_revisar: Vec<Problema>,
}

/// Rango del recorte de hoja que acompaña a un problema (TR-D13).
pub fn rango_recorte(p: &Problema) -> Option<(Celda, Celda)> {
    p.hoja.as_ref()?;
    let celda: Celda = p.celda.as_deref()?.parse().ok()?;
    let encontrada: Option<Celda> = p.celda_encontrada.as_deref().and_then(|c| c.parse().ok());
    let (fila_min, fila_max) = match encontrada {
        Some(e) => (celda.fila.min(e.fila), celda.fila.max(e.fila)),
        None => (celda.fila, celda.fila + 1),
    };
    let desde = Celda::nueva(fila_min.saturating_sub(1).max(1), celda.columna);
    let hasta = Celda::nueva(fila_max + 1, celda.columna + 3);
    Some((desde, hasta))
}

fn contrato<'a>(
    contratos: &'a std::collections::BTreeMap<String, Contrato>,
    fuente: &str,
) -> Result<&'a Contrato, ErrorApp> {
    contratos
        .get(fuente)
        .ok_or_else(|| ErrorApp::nuevo("interno", format!("Falta el contrato «{fuente}».")))
}

fn elegido(contrato: &Contrato, ruta: &Path) -> ArchivoElegido {
    let nombre = ruta.file_name().map_or_else(
        || ruta.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    );
    let carpeta = ruta
        .parent()
        .map(|p| {
            let partes: Vec<String> = p
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .filter(|c| !c.is_empty() && c != "\\" && c != "/")
                .collect();
            let desde = partes.len().saturating_sub(2);
            partes[desde..].join(" › ")
        })
        .unwrap_or_default();
    let meta = std::fs::metadata(ruta).ok();
    let modificado = meta.as_ref().and_then(|m| m.modified().ok()).map(|t| {
        chrono::DateTime::<chrono::Local>::from(t)
            .format("%Y-%m-%d")
            .to_string()
    });
    ArchivoElegido {
        ruta: ruta.display().to_string(),
        fecha_nombre: fecha_en_nombre(contrato, &nombre),
        nombre,
        carpeta,
        bytes: meta.map_or(0, |m| m.len()),
        modificado,
    }
}

/// Nombre, carpeta, tamaño y fecha del nombre de cada archivo, sin abrirlo.
pub fn reconocer(
    config: &Path,
    reporte: &Reporte,
    rutas: &[PathBuf],
) -> Result<Vec<ArchivoElegido>, ErrorApp> {
    let contratos = cargar_contratos(&config.join("fuentes"))?;
    let c = contrato(&contratos, &reporte.fuente)?;
    Ok(rutas.iter().map(|r| elegido(c, r)).collect())
}

/// Lo mismo que `reconocer`, con el contrato de otra fuente (el complemento guardado).
pub fn reconocer_fuente(
    config: &Path,
    fuente: &str,
    ruta: &Path,
) -> Result<ArchivoElegido, ErrorApp> {
    let contratos = cargar_contratos(&config.join("fuentes"))?;
    Ok(elegido(contrato(&contratos, fuente)?, ruta))
}

/// Revisa los archivos del reporte (con su contrato) y los complementos (con el contrato del primer
/// complemento del reporte), con un recorte de hoja para cada problema ubicado en una celda.
pub fn revisar(
    config: &Path,
    reporte: &Reporte,
    rutas: &[PathBuf],
    complementos: &[PathBuf],
) -> Result<Vec<RevisionArchivo>, ErrorApp> {
    let contratos = cargar_contratos(&config.join("fuentes"))?;
    let principal = contrato(&contratos, &reporte.fuente)?;
    let complemento = match reporte.complementos.first() {
        Some(f) => Some(contrato(&contratos, f)?),
        None => None,
    };
    let mut salida = Vec::new();
    let pares = rutas.iter().map(|r| (principal, r)).chain(
        complementos
            .iter()
            .filter_map(|r| complemento.map(|c| (c, r))),
    );
    for (c, ruta) in pares {
        let r = revisar_archivo(c, ruta);
        let problemas = r
            .informe
            .problemas
            .into_iter()
            .map(|p| {
                let recorte = rango_recorte(&p).and_then(|(desde, hasta)| {
                    recorte(ruta, p.hoja.as_deref().unwrap_or_default(), desde, hasta).ok()
                });
                ProblemaVista {
                    problema: p,
                    recorte,
                }
            })
            .collect();
        salida.push(RevisionArchivo {
            archivo: elegido(c, ruta),
            personas: r.personas,
            problemas,
        });
    }
    Ok(salida)
}

/// Miles con `.` y decimales con `,` (como la UI).
pub fn numero(n: f64) -> String {
    let signo = if n < 0.0 { "-" } else { "" };
    let texto = if n.fract() == 0.0 {
        format!("{}", n.abs() as u64)
    } else {
        format!("{:.2}", n.abs())
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    };
    let (entera, decimales) = texto.split_once('.').unwrap_or((&texto, ""));
    let mut con_miles = String::new();
    for (i, c) in entera.chars().enumerate() {
        if i > 0 && (entera.len() - i) % 3 == 0 {
            con_miles.push('.');
        }
        con_miles.push(c);
    }
    if decimales.is_empty() {
        format!("{signo}{con_miles}")
    } else {
        format!("{signo}{con_miles},{decimales}")
    }
}

const MESES: [&str; 12] = [
    "enero",
    "febrero",
    "marzo",
    "abril",
    "mayo",
    "junio",
    "julio",
    "agosto",
    "septiembre",
    "octubre",
    "noviembre",
    "diciembre",
];

fn mes(periodo: &str) -> (&str, &'static str) {
    let (anio, m) = periodo.split_once('-').unwrap_or((periodo, "1"));
    let i = m.parse::<usize>().unwrap_or(1).clamp(1, 12) - 1;
    (anio, MESES[i])
}

fn mayuscula(t: &str) -> String {
    let mut c = t.chars();
    c.next()
        .map(|p| p.to_uppercase().collect::<String>() + c.as_str())
        .unwrap_or_default()
}

/// «Septiembre a noviembre de 2025», «Noviembre 2025» o «Diciembre 2024 a enero 2025».
pub fn titulo_periodos(periodos: &[String]) -> String {
    let mut ps = periodos.to_vec();
    ps.sort();
    match (ps.first(), ps.last()) {
        (Some(a), Some(b)) if a == b => {
            let (anio, m) = mes(a);
            format!("{} {anio}", mayuscula(m))
        }
        (Some(a), Some(b)) => {
            let ((anio_a, ma), (anio_b, mb)) = (mes(a), mes(b));
            if anio_a == anio_b {
                format!("{} a {mb} de {anio_b}", mayuscula(ma))
            } else {
                format!("{} {anio_a} a {mb} {anio_b}", mayuscula(ma))
            }
        }
        _ => String::new(),
    }
}

/// Cifras clave y avisos de la pantalla «Listo».
fn cifra_avisos(n: usize) -> Cifra {
    Cifra {
        valor: numero(n as f64),
        etiqueta: if n == 1 {
            "aviso para revisar".into()
        } else {
            "avisos para revisar".into()
        },
    }
}

/// «Corte: 12 nov 2025».
fn texto_corte(f: chrono::NaiveDate) -> String {
    let periodo = f.format("%Y-%m").to_string();
    let (anio, m) = mes(&periodo);
    format!(
        "Corte: {} {} {anio}",
        f.format("%-d"),
        m.chars().take(3).collect::<String>()
    )
}

pub fn resultado(
    config: &Path,
    salida: &Path,
    reporte: &Reporte,
    ejecucion: Ejecucion,
) -> Result<Resultado, ErrorApp> {
    let para_revisar: Vec<Problema> = ejecucion
        .problemas
        .iter()
        .filter(|p| p.severidad == Severidad::Aviso)
        .cloned()
        .collect();
    let tablero = Tablero::nuevo(config, salida);
    let indicadores = tablero.indicadores(&Filtros::default()).ok();
    let valor = |id: &str| {
        indicadores
            .as_ref()
            .and_then(|r| r.indicadores.iter().find(|i| i.id == id))
            .and_then(|i| i.valor)
    };
    let mut cifras = Vec::new();
    let titulo = if reporte.fuente == "planilla" {
        if let (Some(v), Some(p)) = (
            valor("IND-01"),
            indicadores.as_ref().and_then(|r| r.periodo.clone()),
        ) {
            cifras.push(Cifra {
                valor: numero(v),
                etiqueta: format!("personas en la planilla de {}", mes(&p).1),
            });
        }
        let n = ejecucion.periodos.len();
        cifras.push(Cifra {
            valor: if n == 1 {
                "1 mes".into()
            } else {
                format!("{n} meses")
            },
            etiqueta: ejecucion
                .periodos
                .iter()
                .map(|p| mes(p).1.chars().take(3).collect::<String>())
                .collect::<Vec<_>>()
                .join(" · "),
        });
        cifras.push(cifra_avisos(para_revisar.len()));
        titulo_periodos(&ejecucion.periodos)
    } else if reporte.fuente != "base_personal" {
        // Plan 8: reporte del molde. Filas de su Gold, avisos y la fecha de sus archivos.
        let gold = format!("gold/{}.parquet", reporte.fuente);
        if let Some((_, filas)) = ejecucion.salidas.iter().find(|(r, _)| *r == gold) {
            cifras.push(Cifra {
                valor: numero(*filas as f64),
                etiqueta: "filas en el reporte".into(),
            });
        }
        cifras.push(cifra_avisos(para_revisar.len()));
        let fechas: Vec<&FechaNombre> = ejecucion
            .fuentes
            .iter()
            .filter(|f| f.fuente == reporte.fuente)
            .filter_map(|f| f.fecha_nombre.as_ref())
            .collect();
        let periodos: Vec<String> = fechas
            .iter()
            .filter_map(|f| match f {
                FechaNombre::Periodo(p) => Some(p.clone()),
                FechaNombre::FechaCorte(_) => None,
            })
            .collect();
        let corte = fechas.iter().find_map(|f| match f {
            FechaNombre::FechaCorte(c) => Some(*c),
            FechaNombre::Periodo(_) => None,
        });
        if !periodos.is_empty() {
            let mut periodos = periodos;
            periodos.sort();
            periodos.dedup();
            titulo_periodos(&periodos)
        } else {
            corte.map(texto_corte).unwrap_or_default()
        }
    } else {
        if let Some((_, filas)) = ejecucion
            .salidas
            .iter()
            .find(|(r, _)| r == "gold/base_personal_empleados.parquet")
        {
            cifras.push(Cifra {
                valor: numero(*filas as f64),
                etiqueta: "empleados".into(),
            });
        }
        for (id, etiqueta) in [
            ("IND-07", "contratos por superar su plazo"),
            ("IND-08", "cumplen la edad de jubilación"),
        ] {
            if let Some(v) = valor(id) {
                cifras.push(Cifra {
                    valor: numero(v),
                    etiqueta: etiqueta.into(),
                });
            }
        }
        ejecucion.fecha_corte.map(texto_corte).unwrap_or_default()
    };
    Ok(Resultado {
        ejecucion,
        titulo_periodos: titulo,
        cifras,
        para_revisar,
    })
}

/// Archivo que la app acepta abrir con el programa del sistema: un Excel que existe.
pub fn abrible(ruta: &Path) -> bool {
    let excel = ruta
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| ["xlsx", "xlsm", "xls"].contains(&e.to_ascii_lowercase().as_str()));
    excel && ruta.is_file()
}

/// Puerto de la API del tablero dentro de la app (Plan 7). En debug, fijo (`ETL_LOCAL_PUERTO` o 8787) para que
/// Vite (`localhost:5173`) le reenvíe `/api`; en release, 0: el sistema elige uno libre.
pub fn puerto_api(debug: bool, entorno: Option<String>) -> u16 {
    if !debug {
        return 0;
    }
    entorno.and_then(|v| v.parse().ok()).unwrap_or(8787)
}

/// Dirección de la ventana del tablero: `{base}/tablero.html`, más `#{pestana}` si viene.
pub fn url_tablero(base: &str, pestana: Option<&str>) -> String {
    match pestana {
        Some(p) => format!("{base}/tablero.html#{p}"),
        None => format!("{base}/tablero.html"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONFIG: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../config");
    const DEMO: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../crates/core/tests/fixtures/demo"
    );

    fn config() -> &'static Path {
        Path::new(CONFIG)
    }

    fn planillas() -> Vec<PathBuf> {
        ["2025-09", "2025-10", "2025-11"]
            .iter()
            .map(|p| Path::new(DEMO).join(format!("limpio/planilla/Planilla {p} Empleados.xlsx")))
            .collect()
    }

    fn licencias() -> PathBuf {
        Path::new(DEMO).join("limpio/planilla/licencias/CONTROL DE LICENCIAS.xlsx")
    }

    fn base() -> PathBuf {
        Path::new(DEMO).join("limpio/base_personal/BD. 12.11.2025..xlsx")
    }

    fn reporte(id: &str) -> Reporte {
        etl_local_core::reportes::cargar_reportes(config())
            .unwrap()
            .into_iter()
            .find(|r| r.id == id)
            .unwrap()
    }

    fn problema(hoja: Option<&str>, celda: Option<&str>, encontrada: Option<&str>) -> Problema {
        Problema {
            codigo: "PRE-10".into(),
            severidad: Severidad::Error,
            archivo: "a.xlsx".into(),
            hoja: hoja.map(String::from),
            celda: celda.map(String::from),
            celda_encontrada: encontrada.map(String::from),
            mensaje: "m".into(),
            detalle: None,
        }
    }

    fn c(t: &str) -> Celda {
        t.parse().unwrap()
    }

    #[test]
    fn puerto_de_la_api() {
        assert_eq!(puerto_api(true, None), 8787);
        assert_eq!(puerto_api(true, Some("9000".into())), 9000);
        assert_eq!(puerto_api(true, Some("no".into())), 8787);
        assert_eq!(puerto_api(false, Some("9000".into())), 0);
    }

    #[test]
    fn direccion_del_tablero() {
        assert_eq!(
            url_tablero("http://127.0.0.1:5000", None),
            "http://127.0.0.1:5000/tablero.html"
        );
        assert_eq!(
            url_tablero("http://127.0.0.1:5000", Some("alertas")),
            "http://127.0.0.1:5000/tablero.html#alertas"
        );
    }

    #[test]
    fn solo_se_abren_excels_que_existen() {
        // Revisión final del Plan 6: «revisar» no debe habilitar abrir cualquier ruta.
        assert!(abrible(&planillas()[0]));
        assert!(!abrible(Path::new("C:/Windows/System32/notepad.exe")));
        assert!(!abrible(Path::new(
            "C:/no/existe/Planilla 2025-11 Empleados.xlsx"
        )));
        let dir = tempfile::tempdir().unwrap();
        let falso = dir.path().join("x.xlsx.exe");
        std::fs::write(&falso, "").unwrap();
        assert!(!abrible(&falso));
    }

    #[test]
    fn rutas_prioridad() {
        let dir = tempfile::tempdir().unwrap();
        let docs = dir.path().join("Documentos");
        let mut ajustes = Ajustes::default();
        let r = resolver_rutas(None, None, dir.path(), &docs, &ajustes);
        assert_eq!(r.salida, docs.join("Reportes RR. HH."));
        assert!(
            r.config.join("fuentes").is_dir(),
            "en debug cae en config/ del repo"
        );
        ajustes.carpeta_salida = Some(dir.path().join("elegida"));
        let r = resolver_rutas(None, None, dir.path(), &docs, &ajustes);
        assert_eq!(r.salida, dir.path().join("elegida"));
        let r = resolver_rutas(
            Some(dir.path().join("cfg")),
            Some(dir.path().join("env")),
            dir.path(),
            &docs,
            &ajustes,
        );
        assert_eq!(r.salida, dir.path().join("env"));
        assert_eq!(r.config, dir.path().join("cfg"));
        std::fs::create_dir_all(dir.path().join("config/fuentes")).unwrap();
        let r = resolver_rutas(None, None, dir.path(), &docs, &ajustes);
        assert_eq!(r.config, dir.path().join("config"));
    }

    #[test]
    fn carpeta_visible_en_documentos() {
        let docs = Path::new("C:/Usuarios/demo/Documentos");
        assert_eq!(
            carpeta_visible(&docs.join("Reportes RR. HH."), docs),
            "Documentos › Reportes RR. HH."
        );
    }

    #[test]
    fn carpeta_visible_fuera() {
        let docs = Path::new("C:/Usuarios/demo/Documentos");
        let otra = Path::new("D:/Compartido/Reportes");
        assert_eq!(carpeta_visible(otra, docs), otra.display().to_string());
    }

    #[test]
    fn rango_recorte_desplazada() {
        let p = problema(Some("Planilla"), Some("A6"), Some("A7"));
        assert_eq!(rango_recorte(&p), Some((c("A5"), c("D8"))));
    }

    #[test]
    fn rango_recorte_solo_celda() {
        let p = problema(Some("Planilla"), Some("C10"), None);
        assert_eq!(rango_recorte(&p), Some((c("C9"), c("F12"))));
    }

    #[test]
    fn rango_recorte_sin_hoja_o_sin_celda() {
        assert_eq!(rango_recorte(&problema(None, Some("A6"), None)), None);
        assert_eq!(rango_recorte(&problema(Some("Planilla"), None, None)), None);
        assert_eq!(
            rango_recorte(&problema(Some("Planilla"), Some("fila 6"), None)),
            None
        );
    }

    #[test]
    fn reconocer_planilla() {
        let r = reconocer(config(), &reporte("planilla"), &planillas()).unwrap();
        let periodos: Vec<_> = r.iter().map(|a| a.fecha_nombre.clone()).collect();
        assert_eq!(
            periodos,
            ["2025-09", "2025-10", "2025-11"].map(|p| Some(FechaNombre::Periodo(p.into())))
        );
        assert!(r.iter().all(|a| a.bytes > 0 && a.modificado.is_some()));
        assert!(
            r[0].carpeta.ends_with("limpio › planilla"),
            "{}",
            r[0].carpeta
        );
    }

    #[test]
    fn revisar_caso_01_trae_recorte() {
        let caso = Path::new(DEMO).join("casos/CASO-01/Planilla 2025-11 Empleados.xlsx");
        let r = revisar(config(), &reporte("planilla"), &[caso], &[]).unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].problemas.len(), 1);
        let recorte = r[0].problemas[0].recorte.as_ref().expect("trae recorte");
        assert_eq!(recorte.fila_inicio, 5);
    }

    #[test]
    fn revisar_planilla_con_licencias_como_complemento() {
        let r = revisar(
            config(),
            &reporte("planilla"),
            &planillas()[..1],
            &[licencias()],
        )
        .unwrap();
        assert_eq!(r.len(), 2);
        let errores: Vec<_> = r
            .iter()
            .flat_map(|x| &x.problemas)
            .filter(|p| p.problema.severidad == Severidad::Error)
            .collect();
        assert!(errores.is_empty(), "{errores:#?}");
        assert_eq!(r[0].personas, Some(1226));
    }

    #[test]
    fn numero_y_titulos() {
        assert_eq!(numero(1239.0), "1.239");
        assert_eq!(numero(86.0), "86");
        assert_eq!(numero(1_234_567.0), "1.234.567");
        assert_eq!(numero(7.29), "7,29");
        let p = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(
            titulo_periodos(&p(&["2025-09", "2025-10", "2025-11"])),
            "Septiembre a noviembre de 2025"
        );
        assert_eq!(titulo_periodos(&p(&["2025-11"])), "Noviembre 2025");
        assert_eq!(
            titulo_periodos(&p(&["2024-12", "2025-01"])),
            "Diciembre 2024 a enero 2025"
        );
    }

    fn actualizar_todo(salida: &Path) -> Ejecucion {
        let nuevas = etl_local_core::proceso::Entradas {
            planillas: planillas(),
            licencias: Some(licencias()),
            base_personal: Some(base()),
            otras: Default::default(),
        };
        etl_local_core::entradas::actualizar(
            config(),
            salida,
            &nuevas,
            &[],
            "m1",
            &mut |_| {},
            &std::sync::atomic::AtomicBool::new(false),
        )
        .unwrap()
    }

    #[test]
    fn resultado_planilla_y_base() {
        let dir = tempfile::tempdir().unwrap();
        let ej = actualizar_todo(dir.path());
        let r = resultado(config(), dir.path(), &reporte("planilla"), ej.clone()).unwrap();
        assert_eq!(r.titulo_periodos, "Septiembre a noviembre de 2025");
        assert_eq!(r.cifras[0].etiqueta, "personas en la planilla de noviembre");
        assert!(!r.cifras[0].valor.is_empty());
        assert_eq!(r.cifras[1].valor, "3 meses");
        assert_eq!(r.cifras[1].etiqueta, "sep · oct · nov");

        let r = resultado(config(), dir.path(), &reporte("base_personal"), ej).unwrap();
        assert_eq!(r.titulo_periodos, "Corte: 12 nov 2025");
        assert_eq!(r.cifras[0].etiqueta, "empleados");
        assert_eq!(r.cifras.len(), 3, "{:?}", r.cifras);
    }

    /// Plan 8: «Listo» de un reporte del molde cuenta las filas de su Gold.
    #[test]
    fn resultado_de_un_reporte_del_molde() {
        let dir = tempfile::tempdir().unwrap();
        let practicantes =
            Path::new(DEMO).join("limpio/practicantes/BD Practicantes 12.11.2025.xlsx");
        let mut nuevas = etl_local_core::proceso::Entradas::default();
        nuevas
            .otras
            .insert("practicantes".into(), vec![practicantes]);
        let ej = etl_local_core::entradas::actualizar(
            config(),
            dir.path(),
            &nuevas,
            &[],
            "m1",
            &mut |_| {},
            &std::sync::atomic::AtomicBool::new(false),
        )
        .unwrap();
        let r = resultado(config(), dir.path(), &reporte("practicantes"), ej).unwrap();
        assert_eq!(r.cifras[0].valor, "60");
        assert_eq!(r.cifras[0].etiqueta, "filas en el reporte");
        assert!(
            r.titulo_periodos.starts_with("Corte:"),
            "{}",
            r.titulo_periodos
        );
        assert!(r.cifras[1].etiqueta.contains("aviso"), "{:?}", r.cifras);
    }
}
