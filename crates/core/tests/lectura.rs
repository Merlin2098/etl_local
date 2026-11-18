mod comun;

use std::path::{Path, PathBuf};

use chrono::NaiveDate;
use comun::{base_personal, contrato, licencias, manifiesto, planilla};
use etl_local_core::contrato::Contrato;
use etl_local_core::lectura::{Lectura, TablaHoja, Valor, leer_archivo};
use etl_local_core::preflight::FechaNombre;
use rust_xlsxwriter::Workbook;

fn leer(fuente: &str, ruta: &Path) -> Lectura {
    leer_archivo(&contrato(fuente), ruta).unwrap_or_else(|i| panic!("{ruta:?}: {i:#?}"))
}

fn suma_enteros(t: &TablaHoja, columna: &str) -> i64 {
    t.columna(columna)
        .unwrap_or_else(|| panic!("falta {columna}"))
        .valores
        .iter()
        .map(|v| match v {
            Valor::Entero(n) => *n,
            Valor::Nulo => 0,
            otro => panic!("{columna}: {otro:?}"),
        })
        .sum()
}

fn suma_decimales(t: &TablaHoja, columna: &str) -> f64 {
    t.columna(columna)
        .unwrap()
        .valores
        .iter()
        .map(|v| match v {
            Valor::Decimal(n) => *n,
            Valor::Entero(n) => *n as f64,
            Valor::Nulo => 0.0,
            otro => panic!("{columna}: {otro:?}"),
        })
        .sum()
}

#[test]
fn limpio_cuadra_con_manifiesto() {
    let m = manifiesto("limpio");
    for (periodo, datos) in m["periodos"].as_object().unwrap() {
        let l = leer("planilla", &planilla("limpio", periodo));
        assert_eq!(l.fecha_nombre, Some(FechaNombre::Periodo(periodo.clone())));
        let t = &l.hojas[0];
        let avisos: Vec<_> = l.informe.avisos().collect();
        assert!(avisos.is_empty(), "{periodo}: {avisos:#?}");
        for dias in [
            "dias_licencia_con_goce",
            "dias_licencia_sin_goce",
            "dias_descanso_medico",
        ] {
            assert_eq!(
                suma_enteros(t, dias),
                datos["dias_ausencia"][dias].as_i64().unwrap(),
                "{periodo} {dias}"
            );
        }
        let maternidad = datos["dias_ausencia"]["dias_subsidio_maternidad"]
            .as_i64()
            .unwrap();
        if maternidad == 0 {
            assert!(t.columna("dias_subsidio_maternidad").is_none(), "{periodo}");
        } else {
            assert_eq!(
                suma_enteros(t, "dias_subsidio_maternidad"),
                maternidad,
                "{periodo}"
            );
        }
        let horas = datos["horas_extra"]["horas_extra_25"].as_f64().unwrap();
        assert!(
            (suma_decimales(t, "horas_extra_25") - horas).abs() < 1e-6,
            "{periodo}"
        );
    }
}

#[test]
fn base_personal_cuadra() {
    let m = manifiesto("limpio");
    let b = &m["base_personal"];
    let l = leer("base_personal", &base_personal("limpio"));
    let total = b["empleados"].as_u64().unwrap() + b["practicantes"].as_u64().unwrap();
    assert_eq!(l.hojas[0].filas() as u64, total);
    let corte = NaiveDate::parse_from_str(b["fecha_corte"].as_str().unwrap(), "%Y-%m-%d").unwrap();
    assert_eq!(l.fecha_nombre, Some(FechaNombre::FechaCorte(corte)));
}

#[test]
fn licencias_dos_hojas() {
    let l = leer("licencias", &licencias("limpio"));
    assert_eq!(l.hojas.len(), 2);
    let patron = regex::Regex::new(r"^\d{4}-\d{2}$").unwrap();
    for t in &l.hojas {
        assert!(t.filas() > 0, "{}", t.hoja);
        for v in &t.columna("periodo").unwrap().valores {
            assert!(matches!(v, Valor::Texto(s) if patron.is_match(s)), "{v:?}");
        }
    }
}

fn periodos(modo: &str) -> Vec<String> {
    manifiesto(modo)["periodos"]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect()
}

#[test]
fn sucio_mismos_conteos() {
    for periodo in periodos("limpio") {
        let limpio = leer("planilla", &planilla("limpio", &periodo));
        let sucio = leer("planilla", &planilla("sucio", &periodo));
        assert_eq!(sucio.hojas[0].filas(), limpio.hojas[0].filas(), "{periodo}");
    }
    let limpio = leer("base_personal", &base_personal("limpio"));
    let sucio = leer("base_personal", &base_personal("sucio"));
    assert_eq!(sucio.hojas[0].filas(), limpio.hojas[0].filas());
}

fn documentos(t: &TablaHoja) -> Vec<Valor> {
    t.columna("numero_documento").unwrap().valores.clone()
}

#[test]
fn sucio_documentos_limpios() {
    let digitos = regex::Regex::new(r"^\d+$").unwrap();
    let mut rutas: Vec<(&str, PathBuf)> = periodos("sucio")
        .iter()
        .map(|p| ("planilla", planilla("sucio", p)))
        .collect();
    rutas.push(("licencias", licencias("sucio")));
    for (fuente, ruta) in rutas {
        for t in &leer(fuente, &ruta).hojas {
            for v in documentos(t) {
                assert!(
                    matches!(&v, Valor::Texto(s) if digitos.is_match(s)),
                    "{ruta:?}: {v:?}"
                );
            }
        }
    }
    // Mismos documentos que en limpio, en el mismo orden.
    let p = periodos("sucio").pop().unwrap();
    assert_eq!(
        documentos(&leer("planilla", &planilla("sucio", &p)).hojas[0]),
        documentos(&leer("planilla", &planilla("limpio", &p)).hojas[0])
    );
}

#[test]
fn sucio_fechas_texto_se_leen() {
    let sucio = leer("base_personal", &base_personal("sucio"));
    let avisos: Vec<_> = sucio
        .informe
        .avisos()
        .filter(|a| a.codigo == "PRE-20")
        .collect();
    assert!(avisos.is_empty(), "{avisos:#?}");
    let limpio = leer("base_personal", &base_personal("limpio"));
    assert_eq!(
        sucio.hojas[0].columna("fecha_entrada").unwrap().valores,
        limpio.hojas[0].columna("fecha_entrada").unwrap().valores
    );
}

#[test]
fn sucio_error_excel_avisa() {
    let avisos: Vec<_> = periodos("sucio")
        .iter()
        .flat_map(|p| leer("planilla", &planilla("sucio", p)).informe.problemas)
        .filter(|a| a.codigo == "PRE-22")
        .collect();
    assert!(!avisos.is_empty());
    assert!(
        avisos
            .iter()
            .all(|a| a.celda.as_deref().is_some_and(|c| !c.is_empty()))
    );
}

// --- Libros mínimos escritos en el test --------------------------------------------------

const CONTRATO_MINIMO: &str = r#"
version: 1
fuente: prueba
titulo: Prueba
archivo: { extensiones: [xlsx] }
hojas:
  - nombre: Datos
    cabecera: A1
    FIN_DATOS
    columnas:
      - { nombre: numero_documento, origen: [DNI], tipo: documento, clave: true }
      - { nombre: nombres, origen: [NOMBRES], tipo: texto, nula: false }
      - { nombre: fecha, origen: [FECHA], tipo: fecha, formatos: [excel, DD/MM/AAAA] }
      - { nombre: dias, origen: [DIAS], tipo: entero, minimo: 0, maximo: 31 }
"#;

fn contrato_minimo(fin_datos: &str) -> Contrato {
    Contrato::desde_yaml(
        &CONTRATO_MINIMO.replace("FIN_DATOS", fin_datos),
        "prueba.yaml",
    )
    .unwrap()
}

enum Celda {
    T(&'static str),
    N(f64),
    V,
}

/// Escribe `Datos` con cabecera DNI, NOMBRES, FECHA, DIAS en la fila 1 y las filas dadas debajo.
fn libro(dir: &Path, filas: &[&[Celda]]) -> PathBuf {
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    ws.set_name("Datos").unwrap();
    for (j, t) in ["DNI", "NOMBRES", "FECHA", "DIAS"].iter().enumerate() {
        ws.write_string(0, j as u16, *t).unwrap();
    }
    for (i, fila) in filas.iter().enumerate() {
        for (j, celda) in fila.iter().enumerate() {
            let (r, c) = (i as u32 + 1, j as u16);
            match celda {
                Celda::T(t) => {
                    ws.write_string(r, c, *t).unwrap();
                }
                Celda::N(n) => {
                    ws.write_number(r, c, *n).unwrap();
                }
                Celda::V => {}
            }
        }
    }
    let ruta = dir.join("prueba.xlsx");
    wb.save(&ruta).unwrap();
    ruta
}

#[test]
fn hoja_sin_datos() {
    let dir = tempfile::tempdir().unwrap();
    let l = leer_archivo(&contrato_minimo(""), &libro(dir.path(), &[])).unwrap();
    assert_eq!(l.hojas[0].filas(), 0);
}

#[test]
fn fecha_invalida_avisa() {
    use Celda::*;
    let dir = tempfile::tempdir().unwrap();
    let ruta = libro(
        dir.path(),
        &[
            &[T("1"), T("Ana"), T("31/02/2025")],
            &[T("2"), T("Luis"), T("15/03/2025")],
        ],
    );
    let l = leer_archivo(&contrato_minimo(""), &ruta).unwrap();
    let fechas = &l.hojas[0].columna("fecha").unwrap().valores;
    assert_eq!(fechas[0], Valor::Nulo);
    assert_eq!(
        fechas[1],
        Valor::Fecha(NaiveDate::from_ymd_opt(2025, 3, 15).unwrap())
    );
    let avisos: Vec<_> = l.informe.avisos().collect();
    assert_eq!(avisos.len(), 1, "{avisos:#?}");
    assert_eq!(avisos[0].codigo, "PRE-20");
    assert_eq!(avisos[0].celda.as_deref(), Some("C2"));
}

#[test]
fn nulo_prohibido_es_error() {
    use Celda::*;
    let dir = tempfile::tempdir().unwrap();
    let ruta = libro(dir.path(), &[&[T("1"), T("Ana")], &[T("2"), V]]);
    let informe = leer_archivo(&contrato_minimo(""), &ruta).unwrap_err();
    let errores: Vec<_> = informe.errores().collect();
    assert_eq!(errores.len(), 1, "{errores:#?}");
    assert_eq!(errores[0].codigo, "PRE-21");
    assert_eq!(errores[0].celda.as_deref(), Some("B3"));
}

#[test]
fn nulo_prohibido_se_agrupa() {
    use Celda::*;
    let dir = tempfile::tempdir().unwrap();
    let ruta = libro(dir.path(), &[&[T("1"), V], &[T("2"), V], &[T("3"), V]]);
    let informe = leer_archivo(&contrato_minimo(""), &ruta).unwrap_err();
    let errores: Vec<_> = informe.errores().collect();
    assert_eq!(errores.len(), 1, "{errores:#?}");
    assert_eq!(errores[0].codigo, "PRE-21");
    assert_eq!(errores[0].celda.as_deref(), Some("B2"));
    assert!(
        errores[0].mensaje.contains("3 filas"),
        "{}",
        errores[0].mensaje
    );
    assert!(errores[0].mensaje.contains("B4"), "{}", errores[0].mensaje);
}

#[test]
fn fila_vacia_corta() {
    use Celda::*;
    let dir = tempfile::tempdir().unwrap();
    let ruta = libro(
        dir.path(),
        &[&[T("1"), T("Ana")], &[], &[T("2"), T("Luis")]],
    );
    let corta = leer_archivo(&contrato_minimo("fin_datos: fila_vacia"), &ruta).unwrap();
    assert_eq!(corta.hojas[0].filas(), 1);
    let sigue = leer_archivo(&contrato_minimo(""), &ruta).unwrap();
    assert_eq!(sigue.hojas[0].filas(), 2);
    assert_eq!(sigue.hojas[0].filas_origen, vec![2, 4]);
    assert_eq!(sigue.informe.problemas, vec![], "una fila vacía no avisa");
}

#[test]
fn documento_como_numero() {
    use Celda::*;
    let dir = tempfile::tempdir().unwrap();
    let ruta = libro(
        dir.path(),
        &[&[N(12345678.0), T("Ana")], &[T(" 12345678.0 "), T("Luis")]],
    );
    let l = leer_archivo(&contrato_minimo(""), &ruta).unwrap();
    assert_eq!(
        documentos(&l.hojas[0]),
        vec![
            Valor::Texto("12345678".into()),
            Valor::Texto("12345678".into())
        ]
    );
}

#[test]
fn clave_nula_avisa_y_descarta() {
    use Celda::*;
    let dir = tempfile::tempdir().unwrap();
    let ruta = libro(
        dir.path(),
        &[
            &[T("#N/A"), T("Ana")],
            &[T("nan"), T("Luis")],
            &[T("3"), T("Eva")],
        ],
    );
    let l = leer_archivo(&contrato_minimo(""), &ruta).unwrap();
    assert_eq!(l.hojas[0].filas_origen, vec![4]);
    let avisos: Vec<_> = l.informe.avisos().collect();
    assert_eq!(avisos.len(), 2, "{avisos:#?}");
    assert!(avisos.iter().all(|a| a.codigo == "PRE-24"));
    assert_eq!(
        avisos
            .iter()
            .map(|a| a.celda.as_deref().unwrap())
            .collect::<Vec<_>>(),
        vec!["A2", "A3"]
    );
    assert!(avisos[0].mensaje.contains("#N/A"), "{}", avisos[0].mensaje);
}

#[test]
fn fila_con_datos_sin_clave_avisa() {
    use Celda::*;
    let dir = tempfile::tempdir().unwrap();
    let ruta = libro(dir.path(), &[&[V, T("Ana")], &[], &[T("2"), T("Luis")]]);
    let l = leer_archivo(&contrato_minimo(""), &ruta).unwrap();
    assert_eq!(l.hojas[0].filas_origen, vec![4]);
    let avisos: Vec<_> = l.informe.avisos().collect();
    assert_eq!(avisos.len(), 1, "{avisos:#?}");
    assert_eq!(avisos[0].codigo, "PRE-24");
    assert_eq!(avisos[0].celda.as_deref(), Some("A2"));
    assert!(avisos[0].mensaje.contains("DNI"), "{}", avisos[0].mensaje);
}

#[test]
fn rango_dice_limites() {
    use Celda::*;
    let dir = tempfile::tempdir().unwrap();
    let ruta = libro(dir.path(), &[&[T("1"), T("Ana"), V, N(40.0)]]);
    let l = leer_archivo(&contrato_minimo(""), &ruta).unwrap();
    let avisos: Vec<_> = l.informe.avisos().collect();
    assert_eq!(avisos.len(), 1, "{avisos:#?}");
    assert_eq!(avisos[0].codigo, "PRE-23");
    assert!(
        avisos[0].mensaje.contains("entre 0 y 31"),
        "{}",
        avisos[0].mensaje
    );
}

#[test]
fn filas_de_totales_se_descartan_sin_aviso() {
    use Celda::*;
    let dir = tempfile::tempdir().unwrap();
    let ruta = libro(
        dir.path(),
        &[
            &[T("1"), T("Ana")],
            &[T("+ TOTAL INGRESOS")],
            &[T("* Total ingresos")],
        ],
    );
    // NOM-BRZ-10: la primera celda es la clave (DNI) en este libro; un texto de totales ahí se descarta.
    let l = leer_archivo(&contrato_minimo("filas_resumen: [total ingresos]"), &ruta).unwrap();
    assert_eq!(l.hojas[0].filas_origen, vec![2]);
    assert_eq!(l.informe.problemas, vec![]);
}

#[test]
fn numero_en_primera_celda_no_es_total() {
    use Celda::*;
    let dir = tempfile::tempdir().unwrap();
    let ruta = libro(dir.path(), &[&[T("1"), T("Ana")], &[V, T("5")]]);
    let l = leer_archivo(&contrato_minimo("filas_resumen: [total ingresos]"), &ruta).unwrap();
    let codigos: Vec<_> = l.informe.avisos().map(|a| a.codigo.as_str()).collect();
    assert_eq!(codigos, vec!["PRE-24"]);
}
