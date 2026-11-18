mod comun;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use comun::{contrato, licencias, manifiesto, planilla};
use etl_local_core::almacen::Almacen;
use etl_local_core::error::ErrorProceso;
use etl_local_core::informe::Problema;
use etl_local_core::lectura::{Valor, leer_archivo};
use etl_local_core::proceso::prueba;
use rust_xlsxwriter::Workbook;

const CONFIG: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../config");

fn periodos() -> Vec<String> {
    manifiesto("limpio")["periodos"]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect()
}

fn planillas_limpio() -> Vec<PathBuf> {
    periodos().iter().map(|p| planilla("limpio", p)).collect()
}

fn gold_limpio() -> (Almacen, Vec<Problema>) {
    prueba::planilla_gold(
        Path::new(CONFIG),
        &planillas_limpio(),
        Some(&licencias("limpio")),
    )
    .unwrap_or_else(|e| panic!("{e:?}"))
}

fn uno(a: &Almacen, sql: &str) -> Vec<Option<String>> {
    a.consultar_texto(sql).unwrap().remove(0)
}

fn numero(v: &Option<String>) -> f64 {
    v.as_deref().map_or(0.0, |s| s.parse().unwrap())
}

#[test]
fn gold_cuadra_con_manifiesto() {
    let m = manifiesto("limpio");
    let (a, _) = gold_limpio();
    for (periodo, datos) in m["periodos"].as_object().unwrap() {
        let f = uno(
            &a,
            &format!(
                "SELECT count(*), sum(dias_licencia_con_goce), sum(dias_descanso_medico), \
                 sum(dias_subsidio_maternidad), sum(horas_extra_25), count(dias_subsidio_otros) \
                 FROM planilla_gold WHERE periodo = '{periodo}'"
            ),
        );
        let d = &datos["dias_ausencia"];
        assert_eq!(
            numero(&f[0]),
            datos["personas_planilla"].as_f64().unwrap(),
            "{periodo}"
        );
        assert_eq!(
            numero(&f[1]),
            d["dias_licencia_con_goce"].as_f64().unwrap(),
            "{periodo}"
        );
        assert_eq!(
            numero(&f[2]),
            d["dias_descanso_medico"].as_f64().unwrap(),
            "{periodo}"
        );
        assert_eq!(
            numero(&f[3]),
            d["dias_subsidio_maternidad"].as_f64().unwrap(),
            "{periodo}"
        );
        let horas = datos["horas_extra"]["horas_extra_25"].as_f64().unwrap();
        assert!((numero(&f[4]) - horas).abs() < 1e-6, "{periodo}");
        assert_eq!(
            numero(&f[5]),
            0.0,
            "{periodo}: el demo no trae otras columnas dinámicas"
        );
    }
}

#[test]
fn columnas_del_gold() {
    let (a, _) = gold_limpio();
    let mut esperadas = vec!["periodo", "anio", "mes", "nombre_mes"];
    let planilla = contrato("planilla");
    esperadas.extend(planilla.hojas[0].columnas.iter().map(|c| c.nombre.as_str()));
    esperadas.extend([
        "dias_subsidio_maternidad",
        "dias_subsidio_otros",
        "motivo_licencia_con_goce",
        "motivo_licencia_sin_goce",
    ]);
    assert_eq!(a.columnas("planilla_gold").unwrap(), esperadas);
}

#[test]
fn meses_con_columnas_distintas() {
    // El demo agrega `DIAS SUB. MATERNIDAD` desde el tercer mes: 2025-09 no la trae (revisión 3).
    let (a, _) = gold_limpio();
    let f = uno(
        &a,
        "SELECT count(dias_subsidio_maternidad) FILTER (periodo = '2025-09'), \
                count(dias_subsidio_maternidad) FILTER (periodo = '2025-11') FROM planilla_gold",
    );
    assert_eq!(f[0].as_deref(), Some("0"));
    assert_ne!(f[1].as_deref(), Some("0"));
}

#[test]
fn nombre_mes_en_espanol() {
    let (a, _) = gold_limpio();
    let f = uno(
        &a,
        "SELECT DISTINCT anio, mes, nombre_mes FROM planilla_gold WHERE periodo = '2025-11'",
    );
    assert_eq!(
        f,
        vec![
            Some("2025".into()),
            Some("11".into()),
            Some("Noviembre".into())
        ]
    );
}

fn pares(
    t: &etl_local_core::lectura::TablaHoja,
    periodo: Option<&str>,
) -> BTreeSet<(String, String)> {
    let docs = &t.columna("numero_documento").unwrap().valores;
    let periodos = t.columna("periodo").map(|c| c.valores.clone());
    (0..t.filas())
        .filter_map(|i| {
            let Valor::Texto(doc) = &docs[i] else {
                return None;
            };
            let per = match (&periodos, periodo) {
                (Some(v), _) => match &v[i] {
                    Valor::Texto(p) => p.clone(),
                    _ => return None,
                },
                (None, Some(p)) => p.to_string(),
                (None, None) => return None,
            };
            Some((per, doc.clone()))
        })
        .collect()
}

#[test]
fn motivos_de_licencia() {
    // NOM-ENR-01/02: un motivo por (periodo, documento) presente en la planilla.
    let (a, _) = gold_limpio();
    let lic = leer_archivo(&contrato("licencias"), &licencias("limpio")).unwrap();
    let mut en_planilla = BTreeSet::new();
    for p in periodos() {
        let l = leer_archivo(&contrato("planilla"), &planilla("limpio", &p)).unwrap();
        en_planilla.extend(pares(&l.hojas[0], Some(&p)));
    }
    let con_goce = pares(&lic.hojas[0], None);
    let esperado = con_goce.intersection(&en_planilla).count();
    let f = uno(
        &a,
        "SELECT count(*) FROM planilla_gold WHERE motivo_licencia_con_goce IS NOT NULL",
    );
    assert_eq!(numero(&f[0]) as usize, esperado);
    assert!(esperado > 0);

    let filas: usize = lic.hojas.iter().map(|h| h.filas()).sum();
    assert_eq!(a.contar("licencias_silver").unwrap() as usize, filas);
    let tipos = a
        .consultar_texto("SELECT DISTINCT tipo_licencia FROM licencias_silver ORDER BY 1")
        .unwrap();
    assert_eq!(
        tipos,
        vec![vec![Some("CON_GOCE".into())], vec![Some("SIN_GOCE".into())]]
    );
}

#[test]
fn sin_base_de_personal_no_se_pierden_filas() {
    // Revisión 4: el Gold de planilla no cruza con la base; las 8 personas sin base siguen ahí.
    let m = manifiesto("limpio");
    let ultimo = periodos().pop().unwrap();
    let (a, _) = gold_limpio();
    let f = uno(
        &a,
        &format!("SELECT count(*) FROM planilla_gold WHERE periodo = '{ultimo}'"),
    );
    assert_eq!(
        numero(&f[0]),
        m["periodos"][&ultimo]["personas_planilla"]
            .as_f64()
            .unwrap()
    );
}

#[test]
fn periodo_repetido_es_error() {
    // TR-D23: dos archivos del mismo periodo detienen el proceso.
    let dir = tempfile::tempdir().unwrap();
    let original = planilla("limpio", "2025-11");
    let a = dir.path().join("Planilla 2025-11 Empleados.xlsx");
    let b = dir.path().join("Copia Planilla 2025-11 Empleados.xlsx");
    std::fs::copy(&original, &a).unwrap();
    std::fs::copy(&original, &b).unwrap();
    match prueba::planilla_gold(Path::new(CONFIG), &[a, b], None) {
        Err(ErrorProceso::Datos(texto)) => {
            assert!(texto.contains("2025-11"), "{texto}");
            assert!(
                texto.contains("Copia Planilla 2025-11 Empleados.xlsx"),
                "{texto}"
            );
            assert!(
                texto.matches("Planilla 2025-11 Empleados.xlsx").count() >= 2,
                "{texto}"
            );
        }
        otro => panic!("{:?}", otro.map(|_| ())),
    }
}

#[test]
fn persona_repetida_avisa() {
    // TR-D23 / NOM-GLD-15: se conserva y se avisa con las filas.
    let dir = tempfile::tempdir().unwrap();
    let c = contrato("planilla");
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    ws.set_name("Planilla").unwrap();
    for (j, col) in c.hojas[0].columnas.iter().enumerate() {
        ws.write_string(5, j as u16, col.origen[0].as_str())
            .unwrap();
        let dato = match col.nombre.as_str() {
            "numero_documento" => "99000001",
            "apellido_paterno" | "nombres" => "X",
            _ => "",
        };
        if !dato.is_empty() {
            ws.write_string(6, j as u16, dato).unwrap();
            ws.write_string(7, j as u16, dato).unwrap();
        }
    }
    let ruta = dir.path().join("Planilla 2025-11 Empleados.xlsx");
    wb.save(&ruta).unwrap();
    let (a, problemas) = prueba::planilla_gold(Path::new(CONFIG), &[ruta], None).unwrap();
    assert_eq!(a.contar("planilla_gold").unwrap(), 2);
    let avisos: Vec<_> = problemas
        .iter()
        .filter(|p| p.codigo == "NOM-GLD-15")
        .collect();
    assert_eq!(avisos.len(), 1, "{problemas:#?}");
    assert!(
        avisos[0].mensaje.contains("7") && avisos[0].mensaje.contains("8"),
        "{}",
        avisos[0].mensaje
    );
    assert!(
        avisos[0].mensaje.contains("99000001"),
        "{}",
        avisos[0].mensaje
    );
}

#[test]
fn subsidio_sin_motivo_se_suma_en_otros() {
    // Revisión final: `DIAS SUB. (*)` llega del lector como `dias_subsidio_otros`; debe sumarse con el resto
    // (vocabulario §4) y no hacer caer el proceso.
    let dir = tempfile::tempdir().unwrap();
    let c = contrato("planilla");
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    ws.set_name("Planilla").unwrap();
    let mut j = 0u16;
    for col in &c.hojas[0].columnas {
        ws.write_string(5, j, col.origen[0].as_str()).unwrap();
        let dato = match col.nombre.as_str() {
            "numero_documento" => "99000001",
            "apellido_paterno" | "nombres" => "X",
            _ => "",
        };
        if !dato.is_empty() {
            ws.write_string(6, j, dato).unwrap();
        }
        j += 1;
    }
    for (cabecera, dias) in [("DIAS SUB. (*)", 2.0), ("DIAS SUB. ACCIDENTE", 3.0)] {
        ws.write_string(5, j, cabecera).unwrap();
        ws.write_number(6, j, dias).unwrap();
        j += 1;
    }
    let ruta = dir.path().join("Planilla 2025-11 Empleados.xlsx");
    wb.save(&ruta).unwrap();
    let (a, _) =
        prueba::planilla_gold(Path::new(CONFIG), &[ruta], None).unwrap_or_else(|e| panic!("{e}"));
    let f = uno(&a, "SELECT dias_subsidio_otros FROM planilla_gold");
    assert_eq!(f[0].as_deref(), Some("5"));
}
