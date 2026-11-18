//! Paridad con el legacy (Plan 4, Tarea 7): el motor procesa los mismos datos demo que el legacy procesó
//! en el Plan 2 (`demo/golden/fixtures/<modo>/`, 3 meses) y se comparan los resultados. Las diferencias
//! esperadas son correcciones decididas y quedan explícitas, con su regla.

mod comun;

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use comun::{base_personal, licencias, manifiesto, planilla};
use etl_local_core::almacen::Almacen;
use etl_local_core::proceso::{Entradas, procesar};

const CONFIG: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../config");
const GOLDEN: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../demo/golden/fixtures");

/// Carpeta de los resultados del legacy, o `None` (con mensaje) si no están.
fn golden(modo: &str) -> Option<PathBuf> {
    let ruta = Path::new(GOLDEN).join(modo);
    if ruta.join("resumen.json").is_file() {
        Some(ruta)
    } else {
        eprintln!(
            "Sin resultados del legacy en {}: se omite la paridad.",
            ruta.display()
        );
        None
    }
}

/// Procesa el modo una sola vez por ejecución de tests y devuelve `<salida>/actual`.
fn motor(modo: &'static str) -> PathBuf {
    static LIMPIO: OnceLock<PathBuf> = OnceLock::new();
    static SUCIO: OnceLock<PathBuf> = OnceLock::new();
    let celda = if modo == "limpio" { &LIMPIO } else { &SUCIO };
    celda
        .get_or_init(|| {
            let periodos: Vec<String> = manifiesto(modo)["periodos"]
                .as_object()
                .unwrap()
                .keys()
                .cloned()
                .collect();
            let entradas = Entradas {
                planillas: periodos.iter().map(|p| planilla(modo, p)).collect(),
                licencias: Some(licencias(modo)),
                base_personal: Some(base_personal(modo)),
                otras: Default::default(),
            };
            let salida = tempfile::tempdir().unwrap().keep();
            procesar(Path::new(CONFIG), &entradas, &salida, "paridad").unwrap();
            salida.join("actual")
        })
        .clone()
}

fn ruta_sql(p: &Path) -> String {
    format!("'{}'", p.display().to_string().replace('\\', "/"))
}

/// Almacén con vistas `l_<nombre>` (legacy) y `n_<nombre>` (motor nuevo).
fn comparar(modo: &'static str) -> Option<Almacen> {
    let g = golden(modo)?;
    let n = motor(modo);
    let a = Almacen::nuevo().unwrap();
    let vistas = [
        ("l_planilla", g.join("planilla_gold.parquet")),
        ("l_licencias", g.join("licencias_silver.parquet")),
        (
            "l_empleados",
            g.join("base_personal_empleados_flags.parquet"),
        ),
        (
            "l_practicantes",
            g.join("base_personal_practicantes_gold.parquet"),
        ),
        ("l_centros", g.join("centros_costo.parquet")),
        ("n_planilla", n.join("gold/planilla.parquet")),
        ("n_licencias", n.join("silver/licencias.parquet")),
        (
            "n_empleados",
            n.join("gold/base_personal_empleados.parquet"),
        ),
        (
            "n_practicantes",
            n.join("gold/base_personal_practicantes.parquet"),
        ),
        ("n_centros", n.join("gold/centros_costo.parquet")),
    ];
    for (nombre, ruta) in vistas {
        a.ejecutar(&format!(
            "CREATE VIEW {nombre} AS SELECT * FROM read_parquet({});",
            ruta_sql(&ruta)
        ))
        .unwrap();
    }
    Some(a)
}

fn entero(a: &Almacen, sql: &str) -> i64 {
    a.consultar_texto(sql).unwrap()[0][0]
        .as_deref()
        .unwrap_or("0")
        .parse()
        .unwrap()
}

/// Documento del legacy sin el `.0` de un número guardado como decimal (TR-55; el lector nuevo lo corrige).
const DOC_LEGACY: &str = r#"regexp_replace("DNI/CEX", '\.0$', '')"#;

/// Columnas de días y horas comunes: (legacy, canónico).
const SUMAS: [(&str, &str); 10] = [
    ("DIAS GOCE HABER(D)", "dias_licencia_con_goce"),
    ("DIAS DIST.LIC.S.GH", "dias_licencia_sin_goce"),
    ("DIAS LIC. PATE (P)", "dias_licencia_paternidad"),
    ("DIAS DESC MEDICO (P)", "dias_descanso_medico"),
    ("DIAS SUBSIDIO ENFERMEDAD", "dias_subsidio_enfermedad"),
    ("DIAS SUB. MATERNIDAD", "dias_subsidio_maternidad"),
    ("# HRS EXTRAS SIMP.25%", "horas_extra_25"),
    ("# HRS EXTRAS SIMP.35%", "horas_extra_35"),
    ("#HRS SIMP.50%", "horas_extra_50"),
    ("#HRS EXTRAS DOBLES", "horas_extra_100"),
];

fn planilla_igual(modo: &'static str) {
    let Some(a) = comparar(modo) else { return };
    let personas = format!(
        "SELECT count(*) FROM ((SELECT \"PERIODO\", {DOC_LEGACY} FROM l_planilla \
           EXCEPT SELECT periodo, numero_documento FROM n_planilla) \
         UNION ALL (SELECT periodo, numero_documento FROM n_planilla \
           EXCEPT SELECT \"PERIODO\", {DOC_LEGACY} FROM l_planilla))"
    );
    assert_eq!(entero(&a, &personas), 0, "{modo}: personas por periodo");
    assert_eq!(
        entero(&a, "SELECT count(*) FROM l_planilla"),
        entero(&a, "SELECT count(*) FROM n_planilla"),
        "{modo}: filas"
    );
    for (legacy, canonico) in SUMAS {
        let sql = format!(
            "SELECT count(*) FROM (SELECT \"PERIODO\" AS periodo, sum(\"{legacy}\") AS s FROM l_planilla GROUP BY 1) l \
             FULL JOIN (SELECT periodo, sum({canonico}) AS s FROM n_planilla GROUP BY 1) n USING (periodo) \
             WHERE abs(coalesce(l.s, 0) - coalesce(n.s, 0)) > 1e-6"
        );
        assert_eq!(
            entero(&a, &sql),
            0,
            "{modo}: suma de {canonico} por periodo"
        );
    }
    // Decisión 6 del plan: el motor no repite motivos; se compara contra el legacy sin repetidos.
    for (legacy, canonico) in [
        ("MOTIVO_CON_GOCE", "motivo_licencia_con_goce"),
        ("MOTIVO_SIN_GOCE", "motivo_licencia_sin_goce"),
    ] {
        let sql = format!(
            "SELECT count(*) FROM (SELECT DISTINCT \"PERIODO\" AS periodo, {DOC_LEGACY} AS doc, \
                 array_to_string(list_sort(list_distinct(string_split(\"{legacy}\", ' | '))), ' | ') AS m \
               FROM l_planilla) l \
             JOIN (SELECT DISTINCT periodo, numero_documento AS doc, {canonico} AS m FROM n_planilla) n \
               USING (periodo, doc) \
             WHERE l.m IS DISTINCT FROM n.m"
        );
        assert_eq!(entero(&a, &sql), 0, "{modo}: {canonico}");
    }
}

#[test]
fn planilla_igual_que_legacy_limpio() {
    planilla_igual("limpio");
}

#[test]
fn planilla_igual_que_legacy_sucio() {
    planilla_igual("sucio");
}

#[test]
fn licencias_igual_que_legacy() {
    for modo in ["limpio", "sucio"] {
        let Some(a) = comparar(modo) else { return };
        let sql = format!(
            "SELECT count(*) FROM ((SELECT \"PERIODO\", {DOC_LEGACY}, \"MOTIVO\", \"TIPO_LICENCIA\" FROM l_licencias \
               EXCEPT ALL SELECT periodo, numero_documento, motivo_licencia, tipo_licencia FROM n_licencias) \
             UNION ALL (SELECT periodo, numero_documento, motivo_licencia, tipo_licencia FROM n_licencias \
               EXCEPT ALL SELECT \"PERIODO\", {DOC_LEGACY}, \"MOTIVO\", \"TIPO_LICENCIA\" FROM l_licencias))"
        );
        assert_eq!(entero(&a, &sql), 0, "{modo}");
    }
}

#[test]
fn base_personal_igual_que_legacy() {
    for modo in ["limpio", "sucio"] {
        let Some(a) = comparar(modo) else { return };
        for grupo in ["empleados", "practicantes"] {
            let sql = format!(
                "SELECT count(*) FROM ((SELECT \"NUMERO DE DOC\" FROM l_{grupo} EXCEPT SELECT numero_documento FROM n_{grupo}) \
                 UNION ALL (SELECT numero_documento FROM n_{grupo} EXCEPT SELECT \"NUMERO DE DOC\" FROM l_{grupo}))"
            );
            assert_eq!(entero(&a, &sql), 0, "{modo}: documentos de {grupo}");
            assert_eq!(
                entero(&a, &format!("SELECT count(*) FROM l_{grupo}")),
                entero(&a, &format!("SELECT count(*) FROM n_{grupo}")),
                "{modo}: cantidad de {grupo}"
            );
        }
        // Decisión 7: el motor excluye los centros de costo nulos.
        let sql = "SELECT count(*) FROM ((SELECT \"CC\", \"NOMBRE CC\", \"CATEGORIA CC\", \"GERENCIA\" FROM l_centros \
                     WHERE \"CC\" IS NOT NULL \
                   EXCEPT SELECT centro_costo, nombre_centro_costo, categoria_centro_costo, gerencia FROM n_centros) \
                   UNION ALL (SELECT centro_costo, nombre_centro_costo, categoria_centro_costo, gerencia FROM n_centros \
                   EXCEPT SELECT \"CC\", \"NOMBRE CC\", \"CATEGORIA CC\", \"GERENCIA\" FROM l_centros))";
        assert_eq!(entero(&a, sql), 0, "{modo}: centros de costo");
    }
}

/// Documentos cuya alerta difiere entre legacy y motor.
fn alertas_distintas(a: &Almacen, legacy: &str, canonico: &str) -> i64 {
    entero(
        a,
        &format!(
            "SELECT count(*) FROM l_empleados l JOIN n_empleados n ON n.numero_documento = l.\"NUMERO DE DOC\" \
             WHERE coalesce(l.\"{legacy}\", false) <> coalesce(n.{canonico}, false)"
        ),
    )
}

#[test]
fn alertas_de_contrato_vs_legacy() {
    if let Some(a) = comparar("limpio") {
        assert_eq!(
            alertas_distintas(&a, "alerta_contrato_obra", "alerta_contrato_obra"),
            0
        );
        assert_eq!(
            alertas_distintas(
                &a,
                "alerta_contrato_incremento",
                "alerta_contrato_incremento"
            ),
            0
        );
    }
    if let Some(a) = comparar("sucio") {
        // Corrección BD-DIF-03: con la fecha de ingreso escrita como texto (SUC-03) el legacy la volvía nula y
        // perdía la alerta. El motor la lee (PRE-20 no se dispara) y la marca: exactamente 1 documento.
        assert_eq!(
            alertas_distintas(&a, "alerta_contrato_obra", "alerta_contrato_obra"),
            1
        );
        let sql = "SELECT count(*) FROM l_empleados l JOIN n_empleados n ON n.numero_documento = l.\"NUMERO DE DOC\" \
                   WHERE n.alerta_contrato_obra AND NOT coalesce(l.alerta_contrato_obra, false)";
        assert_eq!(entero(&a, sql), 1);
        assert_eq!(
            alertas_distintas(
                &a,
                "alerta_contrato_incremento",
                "alerta_contrato_incremento"
            ),
            0
        );
    }
}

#[test]
fn tiempo_servicio_vs_legacy() {
    // Con fin de contrato, la antigüedad no depende del día de ejecución: debe ser igual (BD-FLG-04).
    // El resto depende de `fecha_ejecucion` del legacy (resumen.json, dependencias_de_fecha) y no se compara.
    for modo in ["limpio", "sucio"] {
        let Some(a) = comparar(modo) else { return };
        let sql = "SELECT count(*), count(*) FILTER (l.tiempo_servicio_texto IS DISTINCT FROM n.tiempo_servicio_texto) \
                   FROM l_empleados l JOIN n_empleados n ON n.numero_documento = l.\"NUMERO DE DOC\" \
                   WHERE n.fecha_fin_contrato IS NOT NULL AND l.\"Fecha de Termino\" IS NOT NULL \
                     AND l.\"FECH_INGR.\" IS NOT NULL";
        let f = a.consultar_texto(sql).unwrap().remove(0);
        assert!(
            f[0].as_deref().unwrap().parse::<i64>().unwrap() > 0,
            "{modo}: sin filas comparables"
        );
        assert_eq!(f[1].as_deref(), Some("0"), "{modo}");
    }
}

#[test]
fn jubilacion_difiere_por_fecha_de_referencia() {
    // TR-D15 / TR-D3: misma fórmula por año; el legacy usaba el año de `fecha_ejecucion` y el motor el de corte.
    let Some(a) = comparar("limpio") else { return };
    let resumen: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(golden("limpio").unwrap().join("resumen.json")).unwrap(),
    )
    .unwrap();
    let anio: i64 = resumen["fecha_ejecucion"].as_str().unwrap()[..4]
        .parse()
        .unwrap();
    let motor = |edad: i64| {
        entero(
            &a,
            &format!(
                "SELECT count_if(cumple_{edad}_este_anio OR cumple_{edad}_proximo_anio) FROM n_empleados"
            ),
        )
    };
    let legacy = |edad: i64| {
        entero(
            &a,
            &format!(
                "SELECT count_if(\"cumple_{edad}_esteaño\" OR \"cumple_{edad}_proximoaño\") FROM l_empleados"
            ),
        )
    };
    let con_anio = |edad: i64| {
        entero(
            &a,
            &format!(
                "SELECT count(*) FROM n_empleados WHERE year(fecha_nacimiento) IN ({anio} - {edad}, {anio} + 1 - {edad})"
            ),
        )
    };
    assert_eq!((motor(65), motor(70)), (9, 4), "motor = manifiesto");
    assert_ne!(
        (legacy(65), legacy(70)),
        (9, 4),
        "el legacy usaba la fecha de hoy"
    );
    assert_eq!(
        (con_anio(65), con_anio(70)),
        (legacy(65), legacy(70)),
        "única diferencia: la fecha"
    );
}
