mod comun;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use comun::{base_personal, contrato, manifiesto};
use etl_local_core::almacen::Almacen;
use etl_local_core::informe::Problema;
use etl_local_core::proceso::prueba;
use rust_xlsxwriter::Workbook;

const CONFIG: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../config");

fn gold(ruta: &Path) -> (Almacen, Vec<Problema>) {
    prueba::base_personal(Path::new(CONFIG), ruta).unwrap_or_else(|e| panic!("{e:?}"))
}

fn uno(a: &Almacen, sql: &str) -> Vec<Option<String>> {
    a.consultar_texto(sql).unwrap().remove(0)
}

fn n(v: &Option<String>) -> u64 {
    v.as_deref().unwrap().parse().unwrap()
}

/// (empleados, practicantes, obra, incremento, plazo, 65, 70, jubilacion)
fn conteos(a: &Almacen) -> [u64; 8] {
    let e = uno(
        a,
        "SELECT count(*), count_if(alerta_contrato_obra), count_if(alerta_contrato_incremento), \
         count_if(alerta_contrato_plazo), count_if(cumple_65_este_anio OR cumple_65_proximo_anio), \
         count_if(cumple_70_este_anio OR cumple_70_proximo_anio), count_if(alerta_jubilacion) \
         FROM base_personal_empleados_gold",
    );
    let p = uno(a, "SELECT count(*) FROM base_personal_practicantes_gold");
    [
        n(&e[0]),
        n(&p[0]),
        n(&e[1]),
        n(&e[2]),
        n(&e[3]),
        n(&e[4]),
        n(&e[5]),
        n(&e[6]),
    ]
}

#[test]
fn split_empleados_practicantes() {
    // BD-GLD-06
    let b = &manifiesto("limpio")["base_personal"];
    let c = conteos(&gold(&base_personal("limpio")).0);
    assert_eq!(c[0], b["empleados"].as_u64().unwrap());
    assert_eq!(c[1], b["practicantes"].as_u64().unwrap());
}

#[test]
fn alertas_de_contrato() {
    // BD-FLG-08…10: 11 por obra y 7 por incremento (parámetros del generador, demo/README.md).
    let b = &manifiesto("limpio")["base_personal"];
    let c = conteos(&gold(&base_personal("limpio")).0);
    assert_eq!((c[2], c[3]), (11, 7));
    assert_eq!(c[4], b["casos_contrato_plazo"].as_u64().unwrap());
}

#[test]
fn alertas_de_jubilacion() {
    // BD-FLG-05…07, TR-D15, TR-D3: por año de la fecha de corte.
    let b = &manifiesto("limpio")["base_personal"];
    let c = conteos(&gold(&base_personal("limpio")).0);
    assert_eq!((c[5], c[6]), (9, 4));
    assert_eq!(c[7], b["casos_jubilacion"].as_u64().unwrap());
}

#[test]
fn sucio_igual_que_limpio() {
    // Con fechas escritas como texto (SUC-03) el legacy perdía una alerta de obra (BD-DIF-03); aquí no.
    let limpio = conteos(&gold(&base_personal("limpio")).0);
    let sucio = conteos(&gold(&base_personal("sucio")).0);
    assert_eq!(sucio, limpio);
}

#[test]
fn tiempo_servicio_texto_formato() {
    // BD-FLG-04
    let (a, _) = gold(&base_personal("limpio"));
    let malos = uno(
        &a,
        "SELECT count(*) FROM base_personal_empleados_gold WHERE fecha_entrada IS NOT NULL \
         AND NOT regexp_full_match(tiempo_servicio_texto, '\\d+ años, \\d{1,2} meses')",
    );
    assert_eq!(n(&malos[0]), 0);
}

#[test]
fn centros_costo_unicos() {
    // BD-CC-01…03
    let (a, _) = gold(&base_personal("limpio"));
    let f = uno(
        &a,
        "SELECT count(*), count(DISTINCT centro_costo), count(*) FILTER (centro_costo IS NULL), \
         (SELECT count(DISTINCT centro_costo) FROM base_personal_silver) FROM centros_costo",
    );
    assert_eq!(n(&f[0]), n(&f[1]));
    assert_eq!(n(&f[2]), 0);
    assert_eq!(n(&f[0]), n(&f[3]));
}

// --- Libro mínimo de base de personal ----------------------------------------------------

/// Escribe `BD. 12.11.2025..xlsx` con la cabecera del contrato en A10 y una fila por mapa.
fn libro(dir: &Path, filas: &[BTreeMap<&str, &str>]) -> PathBuf {
    let c = contrato("base_personal");
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    ws.set_name("Personal").unwrap();
    for (j, col) in c.hojas[0].columnas.iter().enumerate() {
        ws.write_string(9, j as u16, col.origen[0].as_str())
            .unwrap();
        for (i, fila) in filas.iter().enumerate() {
            if let Some(v) = fila.get(col.nombre.as_str()) {
                ws.write_string(10 + i as u32, j as u16, *v).unwrap();
            }
        }
    }
    let ruta = dir.join("BD. 12.11.2025..xlsx");
    wb.save(&ruta).unwrap();
    ruta
}

fn persona(
    doc: &'static str,
    modalidad: Option<&'static str>,
    entrada: &'static str,
) -> BTreeMap<&'static str, &'static str> {
    let mut m = BTreeMap::from([
        ("numero_documento", doc),
        ("nombre_completo", "PERSONA DEMO"),
        ("fecha_entrada", entrada),
        ("fecha_nacimiento", "01/01/1990"),
    ]);
    if let Some(mo) = modalidad {
        m.insert("modalidad_contrato", mo);
    }
    m
}

#[test]
fn tiempo_servicio_de_un_caso_conocido() {
    // 2023-03-01 → 2025-11-12: 987 días / 365,25 = 2,70 años → "2 años, 8 meses".
    let dir = tempfile::tempdir().unwrap();
    let ruta = libro(
        dir.path(),
        &[persona(
            "99000001",
            Some("PLAZO INDETERMINADO"),
            "01/03/2023",
        )],
    );
    let (a, _) = gold(&ruta);
    let f = uno(
        &a,
        "SELECT tiempo_servicio_texto FROM base_personal_empleados_gold",
    );
    assert_eq!(f[0].as_deref(), Some("2 años, 8 meses"));
}

#[test]
fn sin_modalidad_es_empleado_con_aviso() {
    // TR-D24, corrige BD-GLD-07.
    let dir = tempfile::tempdir().unwrap();
    let ruta = libro(dir.path(), &[persona("99000001", None, "01/03/2023")]);
    let (a, problemas) = gold(&ruta);
    assert_eq!(a.contar("base_personal_empleados_gold").unwrap(), 1);
    let avisos: Vec<_> = problemas
        .iter()
        .filter(|p| p.codigo == "BD-GLD-07")
        .collect();
    assert_eq!(avisos.len(), 1, "{problemas:#?}");
    assert!(
        avisos[0].mensaje.contains("empleados"),
        "{}",
        avisos[0].mensaje
    );
}

#[test]
fn modalidad_con_espacios_y_minusculas() {
    // TR-D24: comparación normalizada. 5 años de antigüedad ≥ 4,5.
    let dir = tempfile::tempdir().unwrap();
    let ruta = libro(
        dir.path(),
        &[persona(
            "99000001",
            Some(" contrato por obra o  servicio especifico "),
            "01/01/2020",
        )],
    );
    let (a, _) = gold(&ruta);
    let f = uno(
        &a,
        "SELECT alerta_contrato_obra, alerta_contrato_plazo FROM base_personal_empleados_gold",
    );
    assert_eq!(f, vec![Some("true".into()), Some("true".into())]);
}

#[test]
fn practicante_con_modalidad_normalizada() {
    let dir = tempfile::tempdir().unwrap();
    let ruta = libro(
        dir.path(),
        &[persona(
            "99000001",
            Some("término de convenio"),
            "01/01/2025",
        )],
    );
    let (a, _) = gold(&ruta);
    assert_eq!(a.contar("base_personal_practicantes_gold").unwrap(), 1);
    assert_eq!(a.contar("base_personal_empleados_gold").unwrap(), 0);
}

#[test]
fn documento_repetido_avisa_y_queda_el_primero() {
    // BD-SLV-07 / TR-D4.
    let dir = tempfile::tempdir().unwrap();
    let ruta = libro(
        dir.path(),
        &[
            persona("99000001", Some("PLAZO INDETERMINADO"), "01/03/2023"),
            persona("99000001", Some("PLAZO INDETERMINADO"), "01/03/2020"),
        ],
    );
    let (a, problemas) = gold(&ruta);
    let f = uno(
        &a,
        "SELECT count(*), min(fecha_entrada) FROM base_personal_empleados_gold",
    );
    assert_eq!(f, vec![Some("1".into()), Some("2023-03-01".into())]);
    let avisos: Vec<_> = problemas
        .iter()
        .filter(|p| p.codigo == "BD-SLV-07")
        .collect();
    assert_eq!(avisos.len(), 1, "{problemas:#?}");
    assert!(
        avisos[0].mensaje.contains("99000001"),
        "{}",
        avisos[0].mensaje
    );
}
