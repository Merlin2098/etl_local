//! Plan 8: el molde de reportes (`Entradas.otras`) y los dominios restantes.

mod comun;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use comun::{base_personal, licencias, manifiesto, otro, planilla};
use etl_local_core::almacen::Almacen;
use etl_local_core::entradas::actualizar;
use etl_local_core::error::ErrorProceso;
use etl_local_core::proceso::{Ejecucion, Entradas, Paso, pasos, procesar};
use rust_xlsxwriter::{ExcelDateTime, Format, Workbook};

const CONFIG: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../config");

fn config() -> &'static Path {
    Path::new(CONFIG)
}

fn solo(fuente: &str, rutas: Vec<PathBuf>) -> Entradas {
    Entradas {
        otras: BTreeMap::from([(fuente.to_string(), rutas)]),
        ..Default::default()
    }
}

/// Filas de una consulta sobre un Parquet de `actual/` (la tabla se llama `t`).
fn consulta(salida: &Path, parquet: &str, sql: &str) -> Vec<Vec<Option<String>>> {
    let a = Almacen::nuevo().unwrap();
    let ruta = salida.join("actual").join(parquet);
    assert!(ruta.exists(), "falta {}", ruta.display());
    a.ejecutar(&format!(
        "CREATE VIEW t AS SELECT * FROM read_parquet('{}');",
        ruta.display().to_string().replace('\'', "''")
    ))
    .unwrap();
    a.consultar_texto(sql).unwrap()
}

fn uno(salida: &Path, parquet: &str, sql: &str) -> String {
    consulta(salida, parquet, sql)[0][0]
        .clone()
        .unwrap_or_default()
}

fn procesa(entradas: &Entradas) -> (tempfile::TempDir, Ejecucion) {
    let dir = tempfile::tempdir().unwrap();
    let ej = procesar(config(), entradas, dir.path(), "prueba").unwrap();
    (dir, ej)
}

// --- Tarea 2: el molde, con Control de practicantes ---------------------------------------------

#[test]
fn pasos_con_otras() {
    let e = solo("x", vec![PathBuf::from("a.xlsx")]);
    assert_eq!(
        pasos(&e),
        vec![
            Paso::Leer,
            Paso::OtrosReportes,
            Paso::Controles,
            Paso::Guardar
        ]
    );
}

#[test]
fn practicantes_cuadra_con_manifiesto() {
    let m = &manifiesto("limpio")["practicantes"];
    let (dir, ej) = procesa(&solo("practicantes", otro("limpio", "practicantes")));
    let s = dir.path();
    let g = "gold/practicantes.parquet";
    assert_eq!(uno(s, g, "SELECT count(*) FROM t"), m["filas"].to_string());
    for alerta in ["por_cumplir_1", "cumplio_1", "por_cumplir_2"] {
        assert_eq!(
            uno(s, g, &format!("SELECT count(*) FROM t WHERE {alerta}")),
            m[alerta].to_string(),
            "{alerta}"
        );
    }
    assert!(s.join("actual/excel/practicantes.xlsx").exists());
    assert!(ej.etapas.iter().any(|e| e.nombre == "practicantes"));
    let pra = ej
        .problemas
        .iter()
        .find(|p| p.codigo == "PRA-01")
        .expect("aviso PRA-01");
    assert!(pra.mensaje.contains("2 practicantes"), "{}", pra.mensaje);
}

/// Control de practicantes con una fila por cantidad de días de servicio (ingreso 2025-01-01).
fn libro_practicantes(dir: &Path, dias: &[i64]) -> PathBuf {
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    ws.set_name("Practicantes").unwrap();
    let fecha = Format::new().set_num_format("dd/mm/yyyy");
    let cab = [
        "DNI",
        "CONDICION",
        "FECHA ING",
        "F. RENOVACION",
        "SEDE",
        "UNIVERSIDAD",
        "JEFE INMEDIATO",
        "GERENCIA",
    ];
    for (c, t) in cab.iter().enumerate() {
        ws.write_string(3, c as u16, *t).unwrap();
    }
    let ingreso = chrono::NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    for (i, d) in dias.iter().enumerate() {
        let f = 4 + i as u32;
        let ren = ingreso + chrono::Duration::days(*d);
        let a_excel = |x: chrono::NaiveDate| {
            ExcelDateTime::from_ymd(
                x.format("%Y").to_string().parse().unwrap(),
                x.format("%m").to_string().parse().unwrap(),
                x.format("%d").to_string().parse().unwrap(),
            )
            .unwrap()
        };
        ws.write_string(f, 0, format!("7000000{i}")).unwrap();
        ws.write_string(f, 1, "Practicante Profesional ").unwrap();
        ws.write_datetime_with_format(f, 2, a_excel(ingreso), &fecha)
            .unwrap();
        ws.write_datetime_with_format(f, 3, a_excel(ren), &fecha)
            .unwrap();
        for c in 4..8 {
            ws.write_string(f, c, "x").unwrap();
        }
    }
    let ruta = dir.join("BD Practicantes 12.11.2025.xlsx");
    wb.save(&ruta).unwrap();
    ruta
}

#[test]
fn tiempo_servicio_en_texto() {
    let libros = tempfile::tempdir().unwrap();
    let ruta = libro_practicantes(libros.path(), &[31, 365, 400, 340, 700]);
    let (dir, _) = procesa(&solo("practicantes", vec![ruta]));
    let filas = consulta(
        dir.path(),
        "gold/practicantes.parquet",
        "SELECT dias_servicio, tiempo_servicio FROM t ORDER BY dias_servicio",
    );
    let textos: Vec<_> = filas.iter().map(|f| f[1].clone().unwrap()).collect();
    assert_eq!(
        textos,
        [
            "1 mes",
            "1 año (En límite)",
            "1 año",
            "1 año y 1 mes",
            "2 años (Límite Máximo)"
        ]
    );
}

#[test]
fn otra_fuente_con_errores_no_publica_nada() {
    let dir = tempfile::tempdir().unwrap();
    let salida = dir.path();
    let mut todo = Entradas {
        planillas: vec![planilla("limpio", "2025-11")],
        licencias: Some(licencias("limpio")),
        base_personal: Some(base_personal("limpio")),
        ..Default::default()
    };
    todo.otras
        .insert("practicantes".into(), otro("limpio", "practicantes"));
    let act = |e: &Entradas, marca: &str| {
        actualizar(
            config(),
            salida,
            e,
            &[],
            marca,
            &mut |_| {},
            &AtomicBool::new(false),
        )
    };
    act(&todo, "uno").unwrap();
    let antes_json = std::fs::read_to_string(salida.join("actual/ejecucion.json")).unwrap();
    let antes = comun::arbol(salida);

    let caso = comun::fixture("casos").join("CASO-09/Practicantes.xlsx");
    let r = act(&solo("practicantes", vec![caso]), "dos");
    assert!(matches!(r, Err(ErrorProceso::Entradas(_))), "{r:?}");
    assert_eq!(
        std::fs::read_to_string(salida.join("actual/ejecucion.json")).unwrap(),
        antes_json
    );
    assert_eq!(comun::arbol(salida), antes);
}

// --- Tarea 3: Planilla régimen especial ----------------------------------------------------------

#[test]
fn regimen_especial_cuadra_y_sin_montos() {
    let m = &manifiesto("limpio")["planilla_regimen_especial"];
    let (dir, _) = procesa(&solo(
        "planilla_regimen_especial",
        otro("limpio", "planilla_regimen_especial"),
    ));
    let s = dir.path();
    let g = "gold/planilla_regimen_especial.parquet";
    let filas = consulta(
        s,
        g,
        "SELECT periodo, count(*) FROM t GROUP BY periodo ORDER BY periodo",
    );
    let esperado: Vec<_> = m
        .as_object()
        .unwrap()
        .iter()
        .map(|(p, n)| vec![Some(p.clone()), Some(n.to_string())])
        .collect();
    assert_eq!(filas, esperado);
    let columnas = consulta(
        s,
        g,
        "SELECT string_agg(column_name, ',') FROM (DESCRIBE t)",
    )[0][0]
        .clone()
        .unwrap();
    for prohibida in [
        "ingreso",
        "descuento",
        "aportacion",
        "neto",
        "nombres",
        "apellido",
    ] {
        assert!(!columnas.contains(prohibida), "{prohibida} en {columnas}");
    }
    assert!(!columnas.contains("total"), "{columnas}");
    assert!(!columnas.contains("neto"), "{columnas}");
    assert_eq!(
        uno(
            s,
            g,
            "SELECT count(*) FROM t WHERE numero_documento IS NULL"
        ),
        "0"
    );
    let primero = m.as_object().unwrap().keys().next().unwrap().clone();
    let mes = uno(
        s,
        g,
        &format!("SELECT DISTINCT nombre_mes FROM t WHERE periodo = '{primero}'"),
    );
    assert_eq!(mes, "Septiembre");
}

#[test]
fn regimen_especial_y_planilla_juntas_no_cruzan_controles() {
    let mut e = Entradas {
        planillas: vec![planilla("limpio", "2025-11")],
        ..Default::default()
    };
    e.otras.insert(
        "planilla_regimen_especial".into(),
        otro("limpio", "planilla_regimen_especial"),
    );
    let (_, ej) = procesa(&e);
    assert!(
        ej.etapas
            .iter()
            .any(|x| x.nombre == "planilla_regimen_especial")
    );
    // Solo la planilla: el control de la planilla especial (prefijo `planilla_regimen_especial_`) no corre.
    let (_, ej) = procesa(&Entradas {
        planillas: vec![planilla("limpio", "2025-11")],
        ..Default::default()
    });
    assert!(ej.etapas.iter().any(|x| x.nombre == "planilla"));
}

// --- Tarea 4: Altas de personal -------------------------------------------------------------------

#[test]
fn altas_por_periodo_y_tipo() {
    let m = &manifiesto("limpio")["altas_personal"];
    let (dir, _) = procesa(&solo("altas_personal", otro("limpio", "altas_personal")));
    let filas = consulta(
        dir.path(),
        "gold/altas_personal.parquet",
        "SELECT periodo, tipo, count(*) FROM t GROUP BY ALL ORDER BY ALL",
    );
    let mut esperado = Vec::new();
    for (periodo, d) in m.as_object().unwrap() {
        for (tipo, clave) in [("empleado", "empleados"), ("practicante", "practicantes")] {
            let n = d[clave].as_u64().unwrap();
            if n > 0 {
                esperado.push(vec![
                    Some(periodo.clone()),
                    Some(tipo.to_string()),
                    Some(n.to_string()),
                ]);
            }
        }
    }
    assert_eq!(filas, esperado);
    let columnas = uno(
        dir.path(),
        "gold/altas_personal.parquet",
        "SELECT string_agg(column_name, ',') FROM (DESCRIBE t)",
    );
    assert!(
        columnas.starts_with("periodo,anio,mes,nombre_mes,tipo,"),
        "{columnas}"
    );
    assert!(
        !columnas.contains("proyecto") && !columnas.contains("sap"),
        "{columnas}"
    );
    assert!(columnas.contains("universidad") && columnas.contains("modalidad_contrato"));
}

// --- Tarea 5: Exámenes médicos de salida ----------------------------------------------------------

#[test]
fn examenes_cruce_con_base() {
    let m = &manifiesto("limpio")["examenes_salida"];
    let mut e = solo("examenes_salida", otro("limpio", "examenes_salida"));
    e.base_personal = Some(base_personal("limpio"));
    let (dir, _) = procesa(&e);
    let g = "gold/examenes_salida.parquet";
    let s = dir.path();
    assert_eq!(uno(s, g, "SELECT count(*) FROM t"), m["filas"].to_string());
    for estado in ["ok", "sin_codigo", "codigo_no_encontrado"] {
        assert_eq!(
            uno(
                s,
                g,
                &format!("SELECT count(*) FROM t WHERE estado_cruce = '{estado}'")
            ),
            m[estado].to_string(),
            "{estado}"
        );
    }
    assert_eq!(
        uno(
            s,
            g,
            "SELECT count(*) FROM t WHERE cargo = 'STUDENT INTERNAL'"
        ),
        "0"
    );
    assert_eq!(
        uno(
            s,
            g,
            "SELECT count(*) FROM t WHERE estado_cruce = 'ok' AND gerencia IS NULL"
        ),
        "0"
    );
    let mes = uno(s, g, "SELECT nombre_mes FROM t WHERE mes = 11 LIMIT 1");
    assert_eq!(mes, "Noviembre");
}

#[test]
fn examenes_sin_base_avisan() {
    let (dir, ej) = procesa(&solo("examenes_salida", otro("limpio", "examenes_salida")));
    let g = "gold/examenes_salida.parquet";
    assert_eq!(
        uno(
            dir.path(),
            g,
            "SELECT count(DISTINCT estado_cruce) || '-' || min(estado_cruce) FROM t"
        ),
        "1-sin_catalogo"
    );
    assert!(ej.problemas.iter().any(|p| p.codigo == "EXS-01"));
}

/// Revisión final: códigos del catálogo que solo difieren en mayúsculas no duplican filas de exámenes.
#[test]
fn examenes_no_duplica_con_codigos_que_difieren_en_mayusculas() {
    let a = Almacen::nuevo().unwrap();
    a.ejecutar(
        "SET VARIABLE es_cargo_practicante = 'STUDENT INTERNAL';
         CREATE TABLE centros_costo AS SELECT * FROM (VALUES
             ('ab01', 'Planta', 'PRODUCTIVO', 'Operaciones'),
             ('AB01', 'Planta B', 'PRODUCTIVO', 'Operaciones')
         ) t(centro_costo, nombre_centro_costo, categoria_centro_costo, gerencia);
         CREATE TABLE examenes_salida_lectura AS SELECT * FROM (VALUES
             ('X, Y', '70000001', DATE '2025-11-03', 'RENUNCIA', 'OPERARIO', 'AB01', 'DATA', 'a.xlsx', 4)
         ) t(nombre_completo, numero_documento, fecha_salida, motivo_salida, cargo, centro_costo, hoja,
             archivo_origen, fila_origen);",
    )
    .unwrap();
    a.ejecutar_archivo(&config().join("sql/examenes_salida/10_gold.sql"))
        .unwrap();
    let filas = a
        .consultar_texto("SELECT count(*), min(estado_cruce) FROM examenes_salida_gold")
        .unwrap();
    assert_eq!(filas[0], [Some("1".into()), Some("ok".into())]);
}
