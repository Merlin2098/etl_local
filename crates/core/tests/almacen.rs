use std::path::Path;

use chrono::NaiveDate;
use etl_local_core::almacen::Almacen;
use etl_local_core::contrato::TipoDato;
use etl_local_core::error::ErrorProceso;
use etl_local_core::lectura::{ColumnaLeida, TablaHoja, Valor};
use etl_local_core::parametros::cargar_parametros;

const PARAMETROS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../config/parametros.yaml");

fn columna(nombre: &str, tipo: TipoDato, valores: Vec<Valor>) -> ColumnaLeida {
    ColumnaLeida {
        nombre: nombre.into(),
        tipo,
        valores,
    }
}

fn tabla_de_tipos() -> TablaHoja {
    let fecha = NaiveDate::from_ymd_opt(2025, 11, 12).unwrap();
    TablaHoja {
        hoja: "Datos".into(),
        filas_origen: vec![2, 3],
        columnas: vec![
            columna(
                "doc",
                TipoDato::Documento,
                vec![Valor::Texto("99000001".into()), Valor::Nulo],
            ),
            columna(
                "dias",
                TipoDato::Entero,
                vec![Valor::Entero(3), Valor::Nulo],
            ),
            columna(
                "horas",
                TipoDato::Decimal,
                vec![Valor::Decimal(2.5), Valor::Nulo],
            ),
            columna(
                "fecha",
                TipoDato::Fecha,
                vec![Valor::Fecha(fecha), Valor::Nulo],
            ),
            columna(
                "activo",
                TipoDato::Booleano,
                vec![Valor::Booleano(true), Valor::Nulo],
            ),
            columna(
                "periodo",
                TipoDato::Periodo,
                vec![Valor::Texto("2025-11".into()), Valor::Nulo],
            ),
        ],
    }
}

#[test]
fn carga_tipos_y_exporta_parquet() {
    let a = Almacen::nuevo().unwrap();
    a.cargar(
        "t",
        &tabla_de_tipos(),
        "x.xlsx",
        &[("fuente", Valor::Texto("prueba".into()))],
    )
    .unwrap();
    assert_eq!(
        a.columnas("t").unwrap(),
        vec![
            "doc",
            "dias",
            "horas",
            "fecha",
            "activo",
            "periodo",
            "fuente",
            "archivo_origen",
            "fila_origen"
        ]
    );
    let dir = tempfile::tempdir().unwrap();
    let ruta = dir.path().join("t.parquet");
    assert_eq!(a.exportar_parquet("t", &ruta).unwrap(), 2);

    let sql = format!(
        "SELECT *, typeof(fecha) AS tipo_fecha FROM read_parquet('{}') ORDER BY fila_origen",
        ruta.display().to_string().replace('\\', "/")
    );
    let filas = a.consultar_texto(&sql).unwrap();
    let texto = |v: &[&str]| v.iter().map(|s| Some(s.to_string())).collect::<Vec<_>>();
    assert_eq!(
        filas[0],
        texto(&[
            "99000001",
            "3",
            "2.5",
            "2025-11-12",
            "true",
            "2025-11",
            "prueba",
            "x.xlsx",
            "2",
            "DATE"
        ])
    );
    assert_eq!(filas[1][..6], vec![None; 6]);
    assert_eq!(filas[1][8].as_deref(), Some("3"));
}

#[test]
fn unir_por_nombre_con_columnas_distintas() {
    let a = Almacen::nuevo().unwrap();
    a.ejecutar(
        "CREATE TABLE t1 AS SELECT 1 AS a, 'x' AS b; CREATE TABLE t2 AS SELECT 2 AS a, 'y' AS c;",
    )
    .unwrap();
    a.unir("u", &["t1".into(), "t2".into()]).unwrap();
    assert_eq!(a.columnas("u").unwrap(), vec!["a", "b", "c"]);
    assert_eq!(a.contar("u").unwrap(), 2);
    let filas = a.consultar_texto("SELECT b, c FROM u ORDER BY a").unwrap();
    assert_eq!(
        filas,
        vec![vec![Some("x".into()), None], vec![None, Some("y".into())]]
    );
}

#[test]
fn variables_y_normalizar() {
    let a = Almacen::nuevo().unwrap();
    let p = cargar_parametros(Path::new(PARAMETROS)).unwrap();
    a.fijar_variables(&p, NaiveDate::from_ymd_opt(2025, 11, 12))
        .unwrap();
    let filas = a
        .consultar_texto(
            "SELECT getvariable('fecha_corte'), getvariable('edad_jubilacion'), \
             getvariable('modalidad_obra'), normalizar(' contrato  por obra ó ')",
        )
        .unwrap();
    assert_eq!(
        filas[0],
        vec![
            Some("2025-11-12".into()),
            Some("65".into()),
            Some("CONTRATO POR OBRA O SERVICIO ESPECIFICO".into()),
            Some("CONTRATO POR OBRA O".into()),
        ]
    );
    // Sin base de personal no hay fecha de corte.
    a.fijar_variables(&p, None).unwrap();
    assert_eq!(
        a.consultar_texto("SELECT getvariable('fecha_corte')")
            .unwrap()[0],
        vec![None]
    );
}

#[test]
fn error_sql_nombra_el_archivo() {
    let dir = tempfile::tempdir().unwrap();
    let ruta = dir.path().join("10_roto.sql");
    std::fs::write(&ruta, "SELEC 1;").unwrap();
    match Almacen::nuevo().unwrap().ejecutar_archivo(&ruta) {
        Err(ErrorProceso::Duckdb { donde, .. }) => {
            assert!(donde.contains("10_roto.sql"), "{donde}")
        }
        otro => panic!("{otro:?}"),
    }
}

#[test]
fn parametros_cargan_y_validan() {
    let p = cargar_parametros(Path::new(PARAMETROS)).unwrap();
    assert_eq!(p.jubilacion.edad, 65);
    assert_eq!(p.contrato_obra.anios_alerta, 4.5);
    assert_eq!(p.subsidios.resto, "dias_subsidio_otros");

    let dir = tempfile::tempdir().unwrap();
    let malo = dir.path().join("parametros.yaml");
    let texto = std::fs::read_to_string(PARAMETROS)
        .unwrap()
        .replace("edad_extendida", "edad_extendia");
    std::fs::write(&malo, texto).unwrap();
    let error = cargar_parametros(&malo).unwrap_err().to_string();
    assert!(error.contains("edad_extendia"), "{error}");

    let texto = std::fs::read_to_string(PARAMETROS)
        .unwrap()
        .replace("anios_alerta: 4.5", "anios_alerta: -1");
    std::fs::write(&malo, texto).unwrap();
    assert!(cargar_parametros(&malo).is_err());
}

#[test]
fn tipos_de_licencia_validos() {
    // Revisión final: el SQL de la planilla Gold espera CON_GOCE y SIN_GOCE; otro valor dejaría los motivos vacíos.
    let dir = tempfile::tempdir().unwrap();
    let malo = dir.path().join("parametros.yaml");
    let texto = std::fs::read_to_string(PARAMETROS)
        .unwrap()
        .replace(": SIN_GOCE", ": SIN_GOCE_HABER");
    std::fs::write(&malo, texto).unwrap();
    let error = cargar_parametros(&malo).unwrap_err().to_string();
    assert!(error.contains("SIN_GOCE_HABER"), "{error}");
}

#[test]
fn fijar_filtros_con_comillas() {
    let a = Almacen::nuevo().unwrap();
    a.fijar_filtros(&[("gerencia", Some("x' OR '1'='1")), ("sede", None)])
        .unwrap();
    assert_eq!(
        a.consultar_texto("SELECT gerencia, sede FROM filtros")
            .unwrap(),
        vec![vec![Some("x' OR '1'='1".to_string()), None]]
    );
    // Se puede volver a fijar (reemplaza la tabla).
    a.fijar_filtros(&[("gerencia", None)]).unwrap();
    assert_eq!(
        a.consultar_texto("SELECT gerencia FROM filtros").unwrap(),
        vec![vec![None]]
    );
}

#[test]
fn fijar_filtros_rechaza_nombre_raro() {
    let a = Almacen::nuevo().unwrap();
    assert!(a.fijar_filtros(&[("a;DROP", Some("x"))]).is_err());
}

#[test]
fn consultar_json_tipos() {
    let a = Almacen::nuevo().unwrap();
    let filas = a
        .consultar_json(
            "SELECT 1::BIGINT a, 2.5::DOUBLE b, true c, DATE '2025-11-12' d, 'x' e, NULL f, \
             'nan'::DOUBLE g, 3::INTEGER h, sum(x)::HUGEINT i, 1.25::DECIMAL(5,2) j              FROM (SELECT 4 AS x)",
        )
        .unwrap();
    assert_eq!(
        serde_json::Value::Object(filas[0].clone()),
        serde_json::json!({"a":1,"b":2.5,"c":true,"d":"2025-11-12","e":"x","f":null,"g":null,"h":3,"i":4,"j":1.25})
    );
}

#[test]
fn vista_parquet_lee() {
    let a = Almacen::nuevo().unwrap();
    a.cargar("t", &tabla_de_tipos(), "x.xlsx", &[]).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let ruta = dir.path().join("t.parquet");
    a.exportar_parquet("t", &ruta).unwrap();
    let b = Almacen::nuevo().unwrap();
    b.vista_parquet("v", &ruta).unwrap();
    assert_eq!(b.contar("v").unwrap(), 2);
}

#[test]
fn metas_validas() {
    let p = cargar_parametros(Path::new(PARAMETROS)).unwrap();
    assert!(p.metas.is_empty());

    let dir = tempfile::tempdir().unwrap();
    let ruta = dir.path().join("parametros.yaml");
    let base = std::fs::read_to_string(PARAMETROS)
        .unwrap()
        .replace("metas: {}", "");
    std::fs::write(&ruta, format!("{base}\nmetas: {{IND-04: 0.02}}\n")).unwrap();
    assert_eq!(cargar_parametros(&ruta).unwrap().metas["IND-04"], 0.02);

    for malo in ["metas: {IND-09: 1}", "metas: {IND-04: -1}"] {
        std::fs::write(&ruta, format!("{base}\n{malo}\n")).unwrap();
        let error = cargar_parametros(&ruta).unwrap_err().to_string();
        let clave = if malo.contains("09") {
            "IND-09"
        } else {
            "IND-04"
        };
        assert!(error.contains(clave), "{error}");
    }
}

#[test]
fn vista_parquet_con_corchetes_en_la_ruta() {
    // Revisión final: «[», «*» y «?» en la carpeta no se leen como comodines de read_parquet.
    let a = Almacen::nuevo().unwrap();
    a.cargar("t", &tabla_de_tipos(), "x.xlsx", &[]).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let ruta = dir.path().join("Reportes [2025]").join("t.parquet");
    a.exportar_parquet("t", &ruta).unwrap();
    let b = Almacen::nuevo().unwrap();
    b.vista_parquet("v", &ruta).unwrap();
    assert_eq!(b.contar("v").unwrap(), 2);
}

#[test]
fn excel_con_fecha_anterior_a_1900_la_escribe_como_texto() {
    // Revisión final del Plan 6: una fecha mal tipeada (12/05/0202) no debe tumbar todo el proceso.
    let a = Almacen::nuevo().unwrap();
    a.ejecutar(
        "CREATE TABLE t_gold AS SELECT * FROM (VALUES (DATE '0202-05-12'), (DATE '2025-11-12')) v(fecha);",
    )
    .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let ruta = dir.path().join("t.xlsx");
    assert_eq!(a.exportar_excel("t_gold", &ruta).unwrap(), 2);
    use calamine::{Reader, open_workbook_auto};
    let mut libro = open_workbook_auto(&ruta).unwrap();
    let hoja = libro.worksheet_range("t").unwrap();
    assert_eq!(hoja.get_value((1, 0)).unwrap().to_string(), "0202-05-12");
}
