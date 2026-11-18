//! Plan 6, Tarea 1: revisión de un archivo para la UI (fecha del nombre, personas y recorte de hoja).

mod comun;

use std::path::PathBuf;

use chrono::NaiveDate;
use comun::{contrato, fixture, manifiesto, planilla};
use etl_local_core::celda::Celda;
use etl_local_core::informe::{Problema, Severidad};
use etl_local_core::preflight::{FechaNombre, fecha_en_nombre};
use etl_local_core::revision::{recorte, revisar_archivo};

fn caso_01() -> PathBuf {
    fixture("casos").join("CASO-01/Planilla 2025-11 Empleados.xlsx")
}

fn celda(texto: &str) -> Celda {
    texto.parse().unwrap()
}

#[test]
fn fecha_del_nombre() {
    assert_eq!(
        fecha_en_nombre(&contrato("planilla"), "Planilla 2025-09 Empleados.xlsx"),
        Some(FechaNombre::Periodo("2025-09".into()))
    );
    assert_eq!(
        fecha_en_nombre(&contrato("base_personal"), "BD. 12.11.2025..xlsx"),
        Some(FechaNombre::FechaCorte(
            NaiveDate::from_ymd_opt(2025, 11, 12).unwrap()
        ))
    );
    assert_eq!(
        fecha_en_nombre(&contrato("planilla"), "planilla sin mes.xlsx"),
        None
    );
}

#[test]
fn revisar_planilla_limpia() {
    let r = revisar_archivo(&contrato("planilla"), &planilla("limpio", "2025-09"));
    assert!(!r.informe.tiene_errores(), "{:#?}", r.informe);
    assert_eq!(r.archivo, "Planilla 2025-09 Empleados.xlsx");
    assert_eq!(r.fecha_nombre, Some(FechaNombre::Periodo("2025-09".into())));
    let esperado = manifiesto("limpio")["periodos"]["2025-09"]["personas_planilla"]
        .as_u64()
        .unwrap();
    assert_eq!(r.personas, Some(esperado));
}

#[test]
fn revisar_con_error_no_cuenta_personas() {
    let r = revisar_archivo(&contrato("planilla"), &caso_01());
    assert!(r.informe.tiene_errores());
    assert_eq!(r.personas, None);
    assert_eq!(r.fecha_nombre, Some(FechaNombre::Periodo("2025-11".into())));
}

#[test]
fn recorte_de_cabecera_desplazada() {
    let r = recorte(&caso_01(), "Planilla", celda("A5"), celda("D8")).unwrap();
    assert_eq!(r.hoja, "Planilla");
    assert_eq!((r.fila_inicio, r.columna_inicio), (5, 1));
    assert_eq!(r.celdas.len(), 4);
    assert!(r.celdas.iter().all(|f| f.len() == 4));
    assert_eq!(r.celdas[1][0], "", "A6 debería estar vacía");
    assert!(!r.celdas[2][0].is_empty(), "A7 debería tener la cabecera");
}

#[test]
fn recorte_limita_el_tamano() {
    let r = recorte(&caso_01(), "Planilla", celda("A1"), celda("Z99")).unwrap();
    assert_eq!(r.celdas.len(), 12);
    assert!(r.celdas.iter().all(|f| f.len() == 8));
}

#[test]
fn recorte_trunca_textos_largos() {
    let r = recorte(&caso_01(), "Planilla", celda("A1"), celda("Z99")).unwrap();
    assert!(r.celdas.iter().flatten().all(|t| t.chars().count() <= 40));
}

#[test]
fn recorte_hoja_inexistente() {
    assert!(recorte(&caso_01(), "No existe", celda("A1"), celda("B2")).is_err());
}

#[test]
fn problema_sin_encontrada_no_serializa_el_campo() {
    let p = Problema {
        codigo: "PRE-09".into(),
        severidad: Severidad::Error,
        archivo: "a.xlsx".into(),
        hoja: None,
        celda: None,
        celda_encontrada: None,
        mensaje: "m".into(),
        detalle: None,
    };
    let json = serde_json::to_string(&p).unwrap();
    assert!(!json.contains("celda_encontrada"), "{json}");
    let ida_vuelta: Problema = serde_json::from_str(&json).unwrap();
    assert_eq!(ida_vuelta, p);
}
