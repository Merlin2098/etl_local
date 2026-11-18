mod comun;

use std::path::{Path, PathBuf};

use chrono::NaiveDate;
use comun::{base_personal, fixture, licencias, manifiesto, planilla};
use etl_local_core::error::ErrorProceso;
use etl_local_core::proceso::{Entradas, EstadoEtapa, procesar};

const CONFIG: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../config");

const PARQUET: [&str; 7] = [
    "silver/planilla.parquet",
    "silver/licencias.parquet",
    "silver/base_personal.parquet",
    "gold/planilla.parquet",
    "gold/base_personal_empleados.parquet",
    "gold/base_personal_practicantes.parquet",
    "gold/centros_costo.parquet",
];

fn periodos() -> Vec<String> {
    manifiesto("limpio")["periodos"]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect()
}

fn entradas_limpio() -> Entradas {
    Entradas {
        planillas: periodos().iter().map(|p| planilla("limpio", p)).collect(),
        licencias: Some(licencias("limpio")),
        base_personal: Some(base_personal("limpio")),
        otras: Default::default(),
    }
}

fn archivos(carpeta: &Path) -> Vec<String> {
    let mut v = Vec::new();
    for sub in ["silver", "gold"] {
        if let Ok(d) = std::fs::read_dir(carpeta.join(sub)) {
            v.extend(d.map(|e| format!("{sub}/{}", e.unwrap().file_name().to_string_lossy())));
        }
    }
    if carpeta.join("ejecucion.json").exists() {
        v.push("ejecucion.json".into());
    }
    v.sort();
    v
}

fn marca_en(carpeta: &Path) -> String {
    let texto = std::fs::read_to_string(carpeta.join("ejecucion.json")).unwrap();
    let json: serde_json::Value = serde_json::from_str(&texto).unwrap();
    json["marca"].as_str().unwrap().to_string()
}

#[test]
fn proceso_completo_limpio() {
    let dir = tempfile::tempdir().unwrap();
    let salida = dir.path().join("salida");
    let ej = procesar(Path::new(CONFIG), &entradas_limpio(), &salida, "m1").unwrap();
    let actual = salida.join("actual");
    for p in PARQUET {
        assert!(actual.join(p).is_file(), "falta {p}");
    }
    assert_eq!(marca_en(&actual), "m1");
    for nombre in ["planilla", "licencias", "base_personal", "controles"] {
        let e = ej
            .etapas
            .iter()
            .find(|e| e.nombre == nombre)
            .unwrap_or_else(|| panic!("{nombre}"));
        assert!(
            matches!(e.estado, EstadoEtapa::Ok | EstadoEtapa::Aviso),
            "{nombre}: {:?}",
            e.estado
        );
    }
    assert_eq!(ej.fecha_corte, NaiveDate::from_ymd_opt(2025, 11, 12));
    assert_eq!(ej.periodos, periodos());
    assert_eq!(archivos(&salida.join("historico/m1")), archivos(&actual));
    assert!(salida.join(".etl-local").is_file());
    let restos: Vec<_> = std::fs::read_dir(&salida)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with(".en-curso") || n.starts_with(".actual"))
        .collect();
    assert!(restos.is_empty(), "{restos:?}");
}

#[test]
fn solo_planillas() {
    // Revisión 1: lo primero que alguien prueba.
    let dir = tempfile::tempdir().unwrap();
    let entradas = Entradas {
        planillas: periodos().iter().map(|p| planilla("limpio", p)).collect(),
        licencias: None,
        base_personal: None,
        otras: Default::default(),
    };
    let ej = procesar(Path::new(CONFIG), &entradas, dir.path(), "m1").unwrap();
    let estado = |n: &str| ej.etapas.iter().find(|e| e.nombre == n).unwrap().estado;
    assert_eq!(estado("licencias"), EstadoEtapa::Omitida);
    assert_eq!(estado("base_personal"), EstadoEtapa::Omitida);
    assert!(dir.path().join("actual/gold/planilla.parquet").is_file());
    assert!(
        !dir.path()
            .join("actual/gold/base_personal_empleados.parquet")
            .exists()
    );
    assert_eq!(ej.fecha_corte, None);
}

#[test]
fn segunda_ejecucion_reemplaza_actual() {
    // Revisión 5.
    let dir = tempfile::tempdir().unwrap();
    procesar(Path::new(CONFIG), &entradas_limpio(), dir.path(), "m1").unwrap();
    procesar(Path::new(CONFIG), &entradas_limpio(), dir.path(), "m2").unwrap();
    assert_eq!(marca_en(&dir.path().join("actual")), "m2");
    assert_eq!(marca_en(&dir.path().join("historico/m1")), "m1");
    assert_eq!(marca_en(&dir.path().join("historico/m2")), "m2");
}

#[test]
fn marca_repetida_es_error() {
    let dir = tempfile::tempdir().unwrap();
    procesar(Path::new(CONFIG), &entradas_limpio(), dir.path(), "m1").unwrap();
    assert!(matches!(
        procesar(Path::new(CONFIG), &entradas_limpio(), dir.path(), "m1"),
        Err(ErrorProceso::Datos(_))
    ));
}

#[test]
fn sin_entradas_es_error() {
    let dir = tempfile::tempdir().unwrap();
    let vacias = Entradas {
        planillas: vec![],
        licencias: None,
        base_personal: None,
        otras: Default::default(),
    };
    assert!(matches!(
        procesar(Path::new(CONFIG), &vacias, dir.path(), "m1"),
        Err(ErrorProceso::Datos(_))
    ));
}

#[test]
fn archivo_con_error_no_escribe() {
    let dir = tempfile::tempdir().unwrap();
    let salida = dir.path().join("salida");
    let caso: PathBuf = fixture("casos").join("CASO-03/Planilla 2025-11 Empleados.xlsx");
    let entradas = Entradas {
        planillas: vec![caso],
        licencias: None,
        base_personal: None,
        otras: Default::default(),
    };
    match procesar(Path::new(CONFIG), &entradas, &salida, "m1") {
        Err(ErrorProceso::Entradas(informes)) => {
            assert_eq!(informes.len(), 1);
            assert!(informes[0].tiene_errores());
        }
        otro => panic!("{:?}", otro.map(|e| e.marca)),
    }
    assert!(!salida.join("actual").exists());
    let restos = std::fs::read_dir(&salida)
        .map(|d| {
            d.filter(|e| {
                e.as_ref()
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".en-curso")
            })
            .count()
        })
        .unwrap_or(0);
    assert_eq!(restos, 0);
}

#[test]
fn carpeta_ajena_no_se_toca() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("mio.txt"), "x").unwrap();
    assert!(matches!(
        procesar(Path::new(CONFIG), &entradas_limpio(), dir.path(), "m1"),
        Err(ErrorProceso::CarpetaAjena(_))
    ));
    assert!(dir.path().join("mio.txt").exists());
    assert!(!dir.path().join("actual").exists());
}

#[cfg(windows)]
#[test]
fn actual_bloqueado_conserva_el_anterior() {
    // Revisión 2: un archivo abierto (Excel, sincronización) impide renombrar `actual/` en Windows.
    use std::os::windows::fs::OpenOptionsExt;
    let dir = tempfile::tempdir().unwrap();
    procesar(Path::new(CONFIG), &entradas_limpio(), dir.path(), "m1").unwrap();
    let bloqueo = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(dir.path().join("actual/gold/planilla.parquet"))
        .unwrap();
    match procesar(Path::new(CONFIG), &entradas_limpio(), dir.path(), "m2") {
        Err(e @ ErrorProceso::Publicar { .. }) => {
            let texto = e.to_string();
            assert!(texto.contains("actual"), "{texto}");
            assert!(!texto.contains("  "), "espacios de más: {texto}");
            assert!(
                texto.contains("abierto") && texto.contains("historico"),
                "{texto}"
            );
            assert!(dir.path().join("historico/m2/ejecucion.json").is_file());
        }
        otro => panic!("{:?}", otro.map(|e| e.marca)),
    }
    drop(bloqueo);
    assert_eq!(marca_en(&dir.path().join("actual")), "m1");
}

#[test]
fn ejecucion_json_se_relee() {
    // Plan 5: la API relee ejecucion.json (TR-D16).
    let dir = tempfile::tempdir().unwrap();
    let ej = procesar(Path::new(CONFIG), &entradas_limpio(), dir.path(), "m1").unwrap();
    let texto = std::fs::read_to_string(dir.path().join("actual/ejecucion.json")).unwrap();
    let relei: etl_local_core::proceso::Ejecucion = serde_json::from_str(&texto).unwrap();
    assert_eq!(relei.periodos, ej.periodos);
    assert_eq!(relei.fecha_corte, ej.fecha_corte);
    assert_eq!(relei.fuentes.len(), ej.fuentes.len());
}

// --- Plan 6, Tarea 2: Excel de revisión ----------------------------------------------------------

fn columnas_parquet(ruta: &Path) -> Vec<String> {
    let a = etl_local_core::almacen::Almacen::nuevo().unwrap();
    a.vista_parquet("t", ruta).unwrap();
    a.columnas("t").unwrap()
}

#[test]
fn escribe_excel_de_revision() {
    use calamine::{Data, Reader, open_workbook_auto};
    let dir = tempfile::tempdir().unwrap();
    let ej = procesar(Path::new(CONFIG), &entradas_limpio(), dir.path(), "m1").unwrap();
    let excel = dir.path().join("actual/excel");
    for n in [
        "planilla",
        "base_personal_empleados",
        "base_personal_practicantes",
        "centros_costo",
    ] {
        assert!(excel.join(format!("{n}.xlsx")).is_file(), "falta {n}.xlsx");
    }
    let filas_de = |ruta: &str| {
        ej.salidas
            .iter()
            .find(|(r, _)| r == ruta)
            .unwrap_or_else(|| panic!("{ruta} no está en salidas"))
            .1
    };
    let mut libro = open_workbook_auto(excel.join("planilla.xlsx")).unwrap();
    let hoja = libro.worksheet_range("planilla").unwrap();
    let cabecera: Vec<String> = hoja
        .rows()
        .next()
        .unwrap()
        .iter()
        .map(|c| c.to_string())
        .collect();
    assert_eq!(
        cabecera,
        columnas_parquet(&dir.path().join("actual/gold/planilla.parquet"))
    );
    assert_eq!(hoja.height() as u64 - 1, filas_de("gold/planilla.parquet"));
    assert_eq!(
        filas_de("excel/planilla.xlsx"),
        filas_de("gold/planilla.parquet")
    );

    let mut libro = open_workbook_auto(excel.join("base_personal_empleados.xlsx")).unwrap();
    let hoja = libro.worksheet_range("base_personal_empleados").unwrap();
    let cabecera: Vec<String> = hoja
        .rows()
        .next()
        .unwrap()
        .iter()
        .map(|c| c.to_string())
        .collect();
    let col = cabecera
        .iter()
        .position(|c| c == "fecha_nacimiento")
        .unwrap();
    let fechas = hoja
        .rows()
        .skip(1)
        .filter(|f| matches!(f[col], Data::DateTime(_)))
        .count();
    assert!(fechas > 0, "fecha_nacimiento no se escribió como fecha");
}

#[test]
fn solo_base_no_escribe_excel_de_planilla() {
    let dir = tempfile::tempdir().unwrap();
    let entradas = Entradas {
        planillas: Vec::new(),
        licencias: None,
        base_personal: Some(base_personal("limpio")),
        otras: Default::default(),
    };
    procesar(Path::new(CONFIG), &entradas, dir.path(), "m1").unwrap();
    let excel = dir.path().join("actual/excel");
    assert!(excel.join("base_personal_empleados.xlsx").is_file());
    assert!(!excel.join("planilla.xlsx").exists());
}

// --- Plan 6, Tarea 3: avance y cancelación -------------------------------------------------------

use etl_local_core::proceso::{Avance, Paso, pasos, procesar_con};
use std::sync::atomic::{AtomicBool, Ordering};

#[test]
fn avance_recorre_los_pasos_en_orden() {
    let dir = tempfile::tempdir().unwrap();
    let entradas = entradas_limpio();
    let mut vistos: Vec<Avance> = Vec::new();
    let no = AtomicBool::new(false);
    procesar_con(
        Path::new(CONFIG),
        &entradas,
        dir.path(),
        "m1",
        &mut |a| vistos.push(a),
        &no,
    )
    .unwrap();
    let esperados = pasos(&entradas);
    assert_eq!(esperados.len(), 6);
    assert_eq!(vistos.iter().map(|a| a.paso).collect::<Vec<_>>(), esperados);
    for (i, a) in vistos.iter().enumerate() {
        assert_eq!((a.indice, a.total), (i, 6));
        assert_eq!(a.pasos, esperados, "cada avance trae la lista completa");
    }
}

#[test]
fn pasos_solo_base() {
    let entradas = Entradas {
        planillas: Vec::new(),
        licencias: None,
        base_personal: Some(base_personal("limpio")),
        otras: Default::default(),
    };
    assert_eq!(
        pasos(&entradas),
        vec![
            Paso::Leer,
            Paso::BasePersonal,
            Paso::Controles,
            Paso::Guardar
        ]
    );
}

#[test]
fn cancelar_antes_de_guardar_no_cambia_nada() {
    let dir = tempfile::tempdir().unwrap();
    procesar(Path::new(CONFIG), &entradas_limpio(), dir.path(), "m1").unwrap();
    let antes = std::fs::read_to_string(dir.path().join("actual/ejecucion.json")).unwrap();
    let cancelar = AtomicBool::new(false);
    let r = procesar_con(
        Path::new(CONFIG),
        &entradas_limpio(),
        dir.path(),
        "m2",
        &mut |a| {
            if a.paso == Paso::Controles {
                cancelar.store(true, Ordering::Relaxed);
            }
        },
        &cancelar,
    );
    assert!(
        matches!(r, Err(ErrorProceso::Cancelado)),
        "{:?}",
        r.map(|e| e.marca)
    );
    assert_eq!(
        std::fs::read_to_string(dir.path().join("actual/ejecucion.json")).unwrap(),
        antes
    );
    let restos: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with(".en-curso"))
        .collect();
    assert!(restos.is_empty(), "{restos:?}");
    assert!(!dir.path().join("historico/m2").exists());
    assert_eq!(
        ErrorProceso::Cancelado.to_string(),
        "Se detuvo el proceso; no se cambió ningún reporte."
    );
}

#[test]
fn licencias_de_meses_no_cargados_no_cuentan_como_huerfanas() {
    // LIC-ENR-04 mira «la planilla del periodo»: las licencias de junio a septiembre no tienen planilla
    // cargada en este proceso y no son licencias sin persona (prueba manual del Plan 6).
    let dir = tempfile::tempdir().unwrap();
    let ultimos: Vec<String> = periodos().into_iter().rev().take(2).collect();
    let entradas = Entradas {
        planillas: ultimos.iter().map(|p| planilla("limpio", p)).collect(),
        licencias: Some(licencias("limpio")),
        base_personal: None,
        otras: Default::default(),
    };
    let ej = procesar(
        Path::new(CONFIG),
        &entradas,
        &dir.path().join("salida"),
        "m1",
    )
    .unwrap();
    let huerfanas: Vec<_> = ej
        .problemas
        .iter()
        .filter(|p| p.codigo == "LIC-ENR-04")
        .collect();
    assert!(huerfanas.is_empty(), "{huerfanas:?}");
}

#[test]
fn preparar_salida_antes_de_procesar_deja_la_carpeta_usable() {
    // Revisión final del Plan 6: «Abrir carpeta» en el inicio crea la carpeta de salida; si la crea sin
    // marcador y el usuario deja algo dentro, ningún proceso posterior la acepta.
    let dir = tempfile::tempdir().unwrap();
    let salida = dir.path().join("Reportes RR. HH.");
    etl_local_core::proceso::preparar_salida(&salida).unwrap();
    std::fs::write(salida.join("nota del usuario.txt"), "hola").unwrap();
    procesar(Path::new(CONFIG), &entradas_limpio(), &salida, "m1").unwrap();
}
