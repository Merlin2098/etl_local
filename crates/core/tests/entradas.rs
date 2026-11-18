//! Plan 6, Tarea 4: entradas guardadas por fuente y `actualizar` (actualizar un reporte a la vez).

mod comun;

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use comun::{base_personal, fixture, licencias, otro, planilla};
use etl_local_core::entradas::{actualizar, combinar, guardadas};
use etl_local_core::error::ErrorProceso;
use etl_local_core::proceso::{Ejecucion, Entradas};

const CONFIG: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../config");
const PERIODOS: [&str; 3] = ["2025-09", "2025-10", "2025-11"];

fn config() -> &'static Path {
    Path::new(CONFIG)
}

/// Salida con tildes, eñe y puntos, como «Documentos › Reportes RR. HH.».
fn salida(dir: &tempfile::TempDir) -> PathBuf {
    dir.path().join("Reportes RR. HH.").join("año ñ")
}

fn todo_limpio() -> Entradas {
    Entradas {
        planillas: PERIODOS.iter().map(|p| planilla("limpio", p)).collect(),
        licencias: Some(licencias("limpio")),
        base_personal: Some(base_personal("limpio")),
        ..Default::default()
    }
}

fn solo_base(ruta: PathBuf) -> Entradas {
    Entradas {
        planillas: Vec::new(),
        licencias: None,
        base_personal: Some(ruta),
        ..Default::default()
    }
}

fn act(
    salida: &Path,
    nuevas: &Entradas,
    excluir: &[String],
    marca: &str,
) -> Result<Ejecucion, ErrorProceso> {
    actualizar(
        config(),
        salida,
        nuevas,
        excluir,
        marca,
        &mut |_| {},
        &AtomicBool::new(false),
    )
}

fn nombres(carpeta: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(carpeta)
        .map(|d| {
            d.map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

#[test]
fn primera_actualizacion_guarda_las_entradas() {
    let dir = tempfile::tempdir().unwrap();
    let s = salida(&dir);
    act(&s, &todo_limpio(), &[], "m1").unwrap();
    let e = s.join("entradas");
    assert_eq!(nombres(&e.join("planilla")).len(), 3);
    assert_eq!(nombres(&e.join("licencias")).len(), 1);
    assert_eq!(nombres(&e.join("base_personal")).len(), 1);
    let original = std::fs::read(planilla("limpio", "2025-09")).unwrap();
    let copia = std::fs::read(e.join("planilla/Planilla 2025-09 Empleados.xlsx")).unwrap();
    assert_eq!(original, copia);
    let g = guardadas(config(), &s).unwrap();
    assert_eq!(g.planillas.len(), 3);
    assert!(g.licencias.is_some() && g.base_personal.is_some());
}

#[test]
fn actualizar_solo_base_conserva_la_planilla() {
    let dir = tempfile::tempdir().unwrap();
    let s = salida(&dir);
    act(&s, &todo_limpio(), &[], "m1").unwrap();
    let ej = act(&s, &solo_base(base_personal("limpio")), &[], "m2").unwrap();
    assert_eq!(ej.periodos, PERIODOS);
    assert!(ej.salidas.iter().any(|(r, _)| r == "gold/planilla.parquet"));
}

#[test]
fn mes_nuevo_reemplaza_al_guardado() {
    let dir = tempfile::tempdir().unwrap();
    let s = salida(&dir);
    act(&s, &todo_limpio(), &[], "m1").unwrap();
    let otra = dir.path().join("Planilla 2025-11 Empleados Final.xlsx");
    std::fs::copy(planilla("limpio", "2025-11"), &otra).unwrap();
    let nuevas = Entradas {
        planillas: vec![otra],
        ..Default::default()
    };
    act(&s, &nuevas, &[], "m2").unwrap();
    let en = nombres(&s.join("entradas/planilla"));
    assert!(
        en.contains(&"Planilla 2025-11 Empleados Final.xlsx".to_string()),
        "{en:?}"
    );
    assert!(
        !en.contains(&"Planilla 2025-11 Empleados.xlsx".to_string()),
        "{en:?}"
    );
    assert_eq!(en.len(), 3);
}

#[test]
fn dos_nuevas_del_mismo_mes_fallan() {
    let dir = tempfile::tempdir().unwrap();
    let otra = dir.path().join("Planilla 2025-11 Empleados Final.xlsx");
    std::fs::copy(planilla("limpio", "2025-11"), &otra).unwrap();
    let nuevas = Entradas {
        planillas: vec![planilla("limpio", "2025-11"), otra],
        ..Default::default()
    };
    match combinar(config(), &Entradas::default(), &nuevas) {
        Err(ErrorProceso::Datos(m)) => {
            assert!(
                m.contains("Planilla 2025-11 Empleados.xlsx")
                    && m.contains("Planilla 2025-11 Empleados Final.xlsx"),
                "{m}"
            );
        }
        otro => panic!("{otro:?}"),
    }
}

#[test]
fn planilla_sin_mes_en_el_nombre_falla() {
    let dir = tempfile::tempdir().unwrap();
    let otra = dir.path().join("planilla sin mes.xlsx");
    std::fs::copy(planilla("limpio", "2025-11"), &otra).unwrap();
    let nuevas = Entradas {
        planillas: vec![otra],
        ..Default::default()
    };
    match combinar(config(), &Entradas::default(), &nuevas) {
        Err(ErrorProceso::Datos(m)) => assert!(m.contains("planilla sin mes.xlsx"), "{m}"),
        otro => panic!("{otro:?}"),
    }
}

#[test]
fn elegir_un_archivo_ya_guardado_no_lo_trunca() {
    let dir = tempfile::tempdir().unwrap();
    let s = salida(&dir);
    act(&s, &todo_limpio(), &[], "m1").unwrap();
    let guardado = s
        .join("entradas/base_personal")
        .join(base_personal("limpio").file_name().unwrap());
    let tamano = std::fs::metadata(&guardado).unwrap().len();
    act(&s, &solo_base(guardado.clone()), &[], "m2").unwrap();
    assert_eq!(std::fs::metadata(&guardado).unwrap().len(), tamano);
    assert_eq!(nombres(&s.join("entradas/base_personal")).len(), 1);
}

#[test]
fn excluir_licencias_no_las_borra() {
    let dir = tempfile::tempdir().unwrap();
    let s = salida(&dir);
    act(&s, &todo_limpio(), &[], "m1").unwrap();
    let ej = act(&s, &Entradas::default(), &["licencias".to_string()], "m2").unwrap();
    assert!(
        !ej.salidas
            .iter()
            .any(|(r, _)| r == "silver/licencias.parquet")
    );
    assert_eq!(nombres(&s.join("entradas/licencias")).len(), 1);
}

#[test]
fn falla_no_toca_entradas() {
    let dir = tempfile::tempdir().unwrap();
    let s = salida(&dir);
    act(&s, &todo_limpio(), &[], "m1").unwrap();
    let antes = nombres(&s.join("entradas/planilla"));
    let caso = fixture("casos").join("CASO-01/Planilla 2025-11 Empleados.xlsx");
    let nuevas = Entradas {
        planillas: vec![caso],
        ..Default::default()
    };
    assert!(matches!(
        act(&s, &nuevas, &[], "m2"),
        Err(ErrorProceso::Entradas(_))
    ));
    assert_eq!(nombres(&s.join("entradas/planilla")), antes);
    let original = std::fs::read(planilla("limpio", "2025-11")).unwrap();
    let copia = std::fs::read(s.join("entradas/planilla/Planilla 2025-11 Empleados.xlsx")).unwrap();
    assert_eq!(
        original, copia,
        "el archivo guardado no debe reemplazarse si falla"
    );
}

#[test]
fn sin_nada_que_procesar_falla() {
    let dir = tempfile::tempdir().unwrap();
    match act(&salida(&dir), &Entradas::default(), &[], "m1") {
        Err(ErrorProceso::Datos(m)) => assert_eq!(m, "No hay archivos para procesar."),
        otro => panic!("{:?}", otro.map(|e| e.marca)),
    }
}

#[cfg(windows)]
#[test]
fn excel_abierto_bloquea_y_no_toca_entradas() {
    use std::os::windows::fs::OpenOptionsExt;
    let dir = tempfile::tempdir().unwrap();
    let s = salida(&dir);
    act(&s, &todo_limpio(), &[], "m1").unwrap();
    let otra_dir = tempfile::tempdir().unwrap();
    let otra = otra_dir.path().join("BD. 30.11.2025..xlsx");
    std::fs::copy(base_personal("limpio"), &otra).unwrap();
    let bloqueo = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(s.join("actual/excel/planilla.xlsx"))
        .unwrap();
    let r = act(&s, &solo_base(otra), &[], "m2");
    drop(bloqueo);
    assert!(
        matches!(r, Err(ErrorProceso::Publicar { .. })),
        "{:?}",
        r.map(|e| e.marca)
    );
    assert_eq!(
        nombres(&s.join("entradas/base_personal")),
        [base_personal("limpio")
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned()]
    );
}

#[test]
fn mismo_mes_con_otras_mayusculas_no_borra_el_guardado() {
    // Revisión final del Plan 6: en Windows el nombre guardado conserva sus mayúsculas al copiar encima;
    // la limpieza no debe tomarlo por sobrante y borrarlo.
    let dir = tempfile::tempdir().unwrap();
    let s = salida(&dir);
    act(&s, &todo_limpio(), &[], "m1").unwrap();
    let otra = dir.path().join("PLANILLA 2025-10 EMPLEADOS.xlsx");
    std::fs::copy(planilla("limpio", "2025-10"), &otra).unwrap();
    let nuevas = Entradas {
        planillas: vec![otra],
        ..Default::default()
    };
    act(&s, &nuevas, &[], "m2").unwrap();
    let g = guardadas(config(), &s).unwrap();
    assert_eq!(
        g.planillas.len(),
        3,
        "{:?}",
        nombres(&s.join("entradas/planilla"))
    );
}

// --- Plan 8: fuentes del molde (`Entradas.otras`) ------------------------------------------------

fn con_practicantes() -> Entradas {
    let mut e = todo_limpio();
    e.otras
        .insert("practicantes".into(), otro("limpio", "practicantes"));
    e
}

#[test]
fn actualizar_conserva_otras_guardadas() {
    let dir = tempfile::tempdir().unwrap();
    let s = salida(&dir);
    act(&s, &con_practicantes(), &[], "m1").unwrap();
    let nuevas = Entradas {
        planillas: vec![planilla("limpio", "2025-11")],
        ..Default::default()
    };
    let ej = act(&s, &nuevas, &[], "m2").unwrap();
    assert!(
        ej.salidas
            .iter()
            .any(|(r, _)| r == "gold/practicantes.parquet")
    );
    assert!(s.join("actual/gold/practicantes.parquet").exists());
    assert_eq!(
        nombres(&s.join("entradas/practicantes")),
        ["BD Practicantes 12.11.2025.xlsx"]
    );
    let g = guardadas(config(), &s).unwrap();
    assert_eq!(g.otras["practicantes"].len(), 1);
}

#[test]
fn otra_nueva_reemplaza_a_la_guardada() {
    let dir = tempfile::tempdir().unwrap();
    let s = salida(&dir);
    act(&s, &con_practicantes(), &[], "m1").unwrap();
    let otra = dir.path().join("BD Practicantes 13.11.2025.xlsx");
    std::fs::copy(&otro("limpio", "practicantes")[0], &otra).unwrap();
    let mut nuevas = Entradas::default();
    nuevas.otras.insert("practicantes".into(), vec![otra]);
    act(&s, &nuevas, &[], "m2").unwrap();
    assert_eq!(
        nombres(&s.join("entradas/practicantes")),
        ["BD Practicantes 13.11.2025.xlsx"]
    );

    let ej = act(
        &s,
        &Entradas::default(),
        &["practicantes".to_string()],
        "m3",
    )
    .unwrap();
    assert!(
        !ej.salidas
            .iter()
            .any(|(r, _)| r == "gold/practicantes.parquet")
    );
    assert_eq!(
        nombres(&s.join("entradas/practicantes")),
        ["BD Practicantes 13.11.2025.xlsx"]
    );
}

#[test]
fn dos_altas_del_mismo_mes_detienen() {
    let dir = tempfile::tempdir().unwrap();
    let original = otro("limpio", "altas_personal")
        .into_iter()
        .find(|r| r.to_string_lossy().contains("2025-11"))
        .unwrap();
    let a = dir.path().join("Altas de personal 2025-11.xlsx");
    let b = dir.path().join("Altas de personal 2025-11 corregido.xlsx");
    std::fs::copy(&original, &a).unwrap();
    std::fs::copy(&original, &b).unwrap();
    let mut nuevas = Entradas::default();
    nuevas.otras.insert("altas_personal".into(), vec![a, b]);
    match combinar(config(), &Entradas::default(), &nuevas) {
        Err(ErrorProceso::Datos(m)) => assert!(m.contains("del mismo mes (2025-11)"), "{m}"),
        otro => panic!("{otro:?}"),
    }
}
