mod comun;

use std::path::{Path, PathBuf};

use comun::{base_personal, contrato, excels, fixture, licencias};
use etl_local_core::contrato::Contrato;
use etl_local_core::informe::{Informe, Severidad};
use etl_local_core::lectura::leer_archivo;
use etl_local_core::preflight::validar_archivo;
use rust_xlsxwriter::Workbook;

fn sin_errores(informe: &Informe) {
    let errores: Vec<_> = informe.errores().collect();
    assert!(errores.is_empty(), "{errores:#?}");
}

#[test]
fn limpio_y_sucio_sin_errores() {
    for modo in ["limpio", "sucio"] {
        let planilla = contrato("planilla");
        for archivo in excels(&fixture(modo).join("planilla")) {
            sin_errores(&validar_archivo(&planilla, &archivo));
        }
        sin_errores(&validar_archivo(
            &contrato("base_personal"),
            &base_personal(modo),
        ));
        sin_errores(&validar_archivo(&contrato("licencias"), &licencias(modo)));
    }
}

/// (caso, fuente, código, hoja, celda, texto que debe estar en el mensaje)
type CasoEsperado = (
    &'static str,
    &'static str,
    &'static str,
    Option<&'static str>,
    Option<&'static str>,
    &'static str,
);

#[test]
fn casos_reportan_su_error() {
    let tabla: Vec<CasoEsperado> = vec![
        (
            "CASO-01",
            "planilla",
            "PRE-10",
            Some("Planilla"),
            Some("A6"),
            "A7",
        ),
        (
            "CASO-02",
            "planilla",
            "PRE-09",
            Some("Planilla"),
            None,
            "Planilla",
        ),
        (
            "CASO-03",
            "planilla",
            "PRE-11",
            Some("Planilla"),
            Some("fila 6"),
            "DNI/CEX",
        ),
        ("CASO-04", "planilla", "PRE-07", None, None, "nombre"),
        ("CASO-05", "base_personal", "PRE-07", None, None, "nombre"),
        (
            "CASO-06",
            "licencias",
            "PRE-09",
            Some("LICENCIA SIN GOCE"),
            None,
            "LICENCIA SIN GOCE",
        ),
        // Plan 8.
        (
            "CASO-07",
            "planilla_regimen_especial",
            "PRE-11",
            Some("Planilla"),
            Some("fila 6"),
            "DNI/CEX",
        ),
        (
            "CASO-08",
            "altas_personal",
            "PRE-09",
            Some("PRACTICANTES"),
            None,
            "PRACTICANTES",
        ),
        ("CASO-09", "practicantes", "PRE-07", None, None, "nombre"),
        (
            "CASO-10",
            "examenes_salida",
            "PRE-11",
            Some("DATA"),
            Some("fila 3"),
            "FECHA DE CESE",
        ),
    ];
    for (caso, fuente, codigo, hoja, celda, en_mensaje) in tabla {
        let carpeta = fixture("casos").join(caso);
        let archivo = excels(&carpeta).remove(0);
        let informe = validar_archivo(&contrato(fuente), &archivo);
        let errores: Vec<_> = informe.errores().collect();
        assert_eq!(errores.len(), 1, "{caso}: {errores:#?}");
        let e = errores[0];
        assert_eq!(e.codigo, codigo, "{caso}");
        assert_eq!(e.hoja.as_deref(), hoja, "{caso}");
        assert_eq!(e.celda.as_deref(), celda, "{caso}");
        assert!(e.mensaje.contains(en_mensaje), "{caso}: {}", e.mensaje);

        let esperado: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(carpeta.join("esperado.json")).unwrap())
                .unwrap();
        let opcion = |v: &serde_json::Value| v.as_str().filter(|s| !s.is_empty()).map(String::from);
        assert_eq!(
            e.hoja,
            opcion(&esperado["hoja"]),
            "{caso} hoja vs esperado.json"
        );
        assert_eq!(
            e.celda,
            opcion(&esperado["celda"]),
            "{caso} celda vs esperado.json"
        );
    }
}

/// Escribe un libro con una hoja y la cabecera completa de la primera hoja del contrato.
#[test]
fn caso_01_informa_celda_encontrada() {
    let archivo = fixture("casos").join("CASO-01/Planilla 2025-11 Empleados.xlsx");
    let informe = validar_archivo(&contrato("planilla"), &archivo);
    let e = informe.errores().next().expect("CASO-01 tiene un error");
    assert_eq!(e.codigo, "PRE-10");
    assert_eq!(e.celda.as_deref(), Some("A6"));
    assert_eq!(e.celda_encontrada.as_deref(), Some("A7"));
}

fn libro_con_cabecera(dir: &Path, nombre: &str, c: &Contrato, extra: &[&str]) -> PathBuf {
    let hoja = &c.hojas[0];
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    ws.set_name(hoja.nombre.lista()[0]).unwrap();
    let fila = hoja.cabecera.fila - 1;
    let textos = hoja
        .columnas
        .iter()
        .map(|c| c.origen[0].as_str())
        .chain(extra.iter().copied());
    for (col, texto) in ((hoja.cabecera.columna - 1) as u16..).zip(textos) {
        ws.write_string(fila, col, texto).unwrap();
    }
    let ruta = dir.join(nombre);
    wb.save(&ruta).unwrap();
    ruta
}

fn unico_error(informe: &Informe) -> (String, String) {
    let errores: Vec<_> = informe.errores().collect();
    assert_eq!(errores.len(), 1, "{errores:#?}");
    assert_eq!(errores[0].severidad, Severidad::Error);
    (errores[0].codigo.clone(), errores[0].mensaje.clone())
}

#[test]
fn fecha_imposible_en_nombre() {
    let dir = tempfile::tempdir().unwrap();
    let c = contrato("base_personal");
    let ruta = libro_con_cabecera(dir.path(), "BD. 31.02.2025..xlsx", &c, &[]);
    let (codigo, mensaje) = unico_error(&validar_archivo(&c, &ruta));
    assert_eq!(codigo, "PRE-07");
    assert!(mensaje.contains("31.02.2025"), "{mensaje}");
}

#[test]
fn extension_no_permitida() {
    let dir = tempfile::tempdir().unwrap();
    let ruta = dir.path().join("Planilla 2025-11 Empleados.csv");
    std::fs::write(&ruta, "a,b").unwrap();
    let informe = validar_archivo(&contrato("planilla"), &ruta);
    assert_eq!(informe.problemas.len(), 1);
    assert_eq!(unico_error(&informe).0, "PRE-06");
}

#[test]
fn archivo_inexistente() {
    let dir = tempfile::tempdir().unwrap();
    let ruta = dir.path().join("Planilla 2025-11 Empleados.xlsx");
    assert_eq!(
        unico_error(&validar_archivo(&contrato("planilla"), &ruta)).0,
        "PRE-05"
    );
}

#[test]
fn archivo_no_excel() {
    let dir = tempfile::tempdir().unwrap();
    let ruta = dir.path().join("Planilla 2025-11 Empleados.xlsx");
    std::fs::write(&ruta, "esto no es un libro de Excel").unwrap();
    let (codigo, mensaje) = unico_error(&validar_archivo(&contrato("planilla"), &ruta));
    assert_eq!(codigo, "PRE-08");
    assert!(mensaje.contains("abrir"), "{mensaje}");
}

#[test]
fn cabecera_duplicada() {
    let dir = tempfile::tempdir().unwrap();
    let c = contrato("planilla");
    let ruta = libro_con_cabecera(
        dir.path(),
        "Planilla 2025-11 Empleados.xlsx",
        &c,
        &["dni/cex "],
    );
    let informe = validar_archivo(&c, &ruta);
    let errores: Vec<_> = informe.errores().collect();
    assert_eq!(errores.len(), 1, "{errores:#?}");
    assert_eq!(errores[0].codigo, "PRE-19");
    // DNI/CEX es la sexta columna del contrato: F6.
    assert_eq!(errores[0].celda.as_deref(), Some("F6"));
}

#[test]
fn carpeta_no_es_archivo() {
    let dir = tempfile::tempdir().unwrap();
    let (codigo, mensaje) = unico_error(&validar_archivo(&contrato("planilla"), dir.path()));
    assert_eq!(codigo, "PRE-05");
    assert!(mensaje.contains("carpeta"), "{mensaje}");
}

fn mensaje_pre08(nombre: &str, contenido: &[u8]) -> (String, Option<String>) {
    let dir = tempfile::tempdir().unwrap();
    let ruta = dir.path().join(nombre);
    std::fs::write(&ruta, contenido).unwrap();
    let informe = validar_archivo(&contrato("planilla"), &ruta);
    let errores: Vec<_> = informe.errores().filter(|e| e.codigo == "PRE-08").collect();
    assert_eq!(errores.len(), 1, "{informe:#?}");
    (errores[0].mensaje.clone(), errores[0].detalle.clone())
}

#[test]
fn archivo_temporal_de_excel() {
    let (mensaje, detalle) = mensaje_pre08("~$Planilla 2025-11 Empleados.xlsx", b"\x00\x01basura");
    assert!(mensaje.contains("temporal"), "{mensaje}");
    assert!(
        !mensaje.contains("Zip") && !mensaje.contains("rror"),
        "{mensaje}"
    );
    assert!(detalle.is_some());
}

#[test]
fn pagina_web_con_extension_de_excel() {
    let (mensaje, _) = mensaje_pre08(
        "Planilla 2025-11 Empleados.xls",
        b"<html><table><tr><td>1</td></tr></table></html>",
    );
    assert!(mensaje.contains("página web"), "{mensaje}");
}

#[test]
fn extension_que_no_coincide() {
    let dir = tempfile::tempdir().unwrap();
    let c = contrato("planilla");
    let real = libro_con_cabecera(dir.path(), "real.xlsx", &c, &[]);
    let (mensaje, _) = mensaje_pre08(
        "Planilla 2025-11 Empleados.xls",
        &std::fs::read(real).unwrap(),
    );
    assert!(mensaje.contains("no corresponde"), "{mensaje}");
}

#[test]
fn solo_la_primera_cabecera_vacia() {
    let dir = tempfile::tempdir().unwrap();
    let c = contrato("planilla");
    // Cabecera completa salvo A6 (codigo_empleado no es obligatoria).
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    ws.set_name("Planilla").unwrap();
    for (j, col) in c.hojas[0].columnas.iter().enumerate().skip(1) {
        ws.write_string(5, j as u16, col.origen[0].as_str())
            .unwrap();
    }
    let ruta = dir.path().join("Planilla 2025-11 Empleados.xlsx");
    wb.save(&ruta).unwrap();
    let (codigo, mensaje) = unico_error(&validar_archivo(&c, &ruta));
    assert_eq!(codigo, "PRE-10");
    assert!(
        mensaje.contains("A6") && mensaje.contains("vacía"),
        "{mensaje}"
    );
    assert!(!mensaje.contains("B6"), "{mensaje}");
}

#[test]
fn hoja_solo_con_titulo() {
    let dir = tempfile::tempdir().unwrap();
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    ws.set_name("Planilla").unwrap();
    ws.write_string(0, 0, "EMPRESA DEMO S.A.C. - PLANILLA")
        .unwrap();
    let ruta = dir.path().join("Planilla 2025-11 Empleados.xlsx");
    wb.save(&ruta).unwrap();
    let c = contrato("planilla");
    let informe = validar_archivo(&c, &ruta);
    let codigos: Vec<_> = informe.errores().map(|e| e.codigo.as_str()).collect();
    assert_eq!(codigos, vec!["PRE-10", "PRE-11"], "{informe:#?}");
    assert!(leer_archivo(&c, &ruta).is_err());
}

#[test]
fn cabeceras_con_salto_de_linea_y_espacio_duro() {
    let dir = tempfile::tempdir().unwrap();
    let c = contrato("planilla");
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    ws.set_name("Planilla").unwrap();
    for (j, col) in c.hojas[0].columnas.iter().enumerate() {
        let texto = match col.origen[0].as_str() {
            "FECHA DE INGRESO" => "FECHA DE\nINGRESO".to_string(),
            "APELLIDO PATERNO" => "APELLIDO\u{a0}PATERNO".to_string(),
            otro => otro.to_string(),
        };
        let dato = match col.nombre.as_str() {
            "numero_documento" => "99000001",
            "apellido_paterno" | "nombres" => "X",
            "fecha_entrada" => "15/03/2025",
            _ => "",
        };
        ws.write_string(5, j as u16, &texto).unwrap();
        if !dato.is_empty() {
            ws.write_string(6, j as u16, dato).unwrap();
        }
    }
    let ruta = dir.path().join("Planilla 2025-11 Empleados.xlsx");
    wb.save(&ruta).unwrap();
    let l = leer_archivo(&c, &ruta).unwrap_or_else(|i| panic!("{i:#?}"));
    let t = &l.hojas[0];
    assert_eq!(t.filas(), 1);
    assert_eq!(
        t.columna("fecha_entrada").unwrap().valores[0],
        etl_local_core::lectura::Valor::Fecha(
            chrono::NaiveDate::from_ymd_opt(2025, 3, 15).unwrap()
        )
    );
    assert_eq!(
        t.columna("apellido_paterno").unwrap().valores[0],
        etl_local_core::lectura::Valor::Texto("X".into())
    );
}

#[test]
fn planilla_y_regimen_especial_no_se_confunden() {
    use etl_local_core::preflight::fecha_en_nombre;
    let estandar = contrato("planilla");
    let especial = contrato("planilla_regimen_especial");
    assert!(fecha_en_nombre(&especial, "Planilla 2025-11 Empleados.xlsx").is_none());
    assert!(fecha_en_nombre(&estandar, "Planilla Regimen Especial 2025-11.xlsx").is_none());
    assert!(fecha_en_nombre(&especial, "Planilla Regimen Especial 2025-11.xlsx").is_some());
}

#[test]
fn dominios_del_plan_8_sin_errores() {
    for modo in ["limpio", "sucio"] {
        for fuente in [
            "planilla_regimen_especial",
            "altas_personal",
            "practicantes",
            "examenes_salida",
        ] {
            let c = contrato(fuente);
            for archivo in excels(&fixture(modo).join(fuente)) {
                sin_errores(&validar_archivo(&c, &archivo));
            }
        }
    }
}
