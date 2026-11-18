use etl_local_core::contrato::{
    ColumnasExtra, Contrato, ErrorContrato, FinDatos, TipoDato, cargar_contratos,
};

const MINIMO: &str = r#"
version: 1
fuente: planilla
titulo: Planilla mensual
archivo:
  extensiones: [xlsx]
  patron_nombre: 'Planilla (?P<periodo>\d{4}-\d{2})\.xlsx'
  fecha_en_nombre: { grupo: periodo, formato: AAAA-MM }
hojas:
  - nombre: Planilla
    cabecera: A6
    columnas:
      - nombre: numero_documento
        origen: ['DNI/CEX']
        tipo: documento
        requerida: true
        clave: true
      - nombre: condicion
        origen: ['Condición']
        tipo: texto
"#;

fn con(original: &str, reemplazo: &str) -> String {
    assert!(MINIMO.contains(original), "{original}");
    MINIMO.replace(original, reemplazo)
}

fn detalles<T: std::fmt::Debug>(r: Result<T, ErrorContrato>) -> Vec<String> {
    match r {
        Err(ErrorContrato::Invalido { detalles, .. }) => detalles,
        otro => panic!("se esperaba Invalido, llegó {otro:?}"),
    }
}

#[test]
fn contrato_minimo_carga() {
    let c = Contrato::desde_yaml(MINIMO, "x.yaml").unwrap();
    let hoja = &c.hojas[0];
    assert_eq!(hoja.nombre.lista(), vec!["Planilla"]);
    assert_eq!(hoja.cabecera.to_string(), "A6");
    assert_eq!(hoja.fin_datos, FinDatos::FinHoja);
    assert_eq!(hoja.columnas_extra, ColumnasExtra::Ignorar);
    let condicion = &hoja.columnas[1];
    assert!(condicion.nula && !condicion.requerida && !condicion.clave);
    assert_eq!(condicion.tipo, TipoDato::Texto);
    assert_eq!(hoja.clave().unwrap().nombre, "numero_documento");
}

#[test]
fn errata_en_clave_nombra_la_clave() {
    let texto = con("requerida: true", "requerido: true");
    match Contrato::desde_yaml(&texto, "x.yaml") {
        Err(e @ ErrorContrato::Yaml { .. }) => {
            let m = e.to_string();
            assert!(m.contains("requerido") && m.contains("x.yaml"), "{m}");
        }
        otro => panic!("{otro:?}"),
    }
}

#[test]
fn version_desconocida() {
    let d = detalles(Contrato::desde_yaml(
        &con("version: 1", "version: 2"),
        "x.yaml",
    ));
    assert!(d.iter().any(|x| x.contains("version")), "{d:?}");
}

#[test]
fn nombre_no_canonico() {
    let d = detalles(Contrato::desde_yaml(
        &con("nombre: condicion", "nombre: Numero Documento"),
        "x.yaml",
    ));
    assert!(d.iter().any(|x| x.contains("Numero Documento")), "{d:?}");
}

#[test]
fn origen_repetido_tras_normalizar() {
    let texto = con("origen: ['DNI/CEX']", "origen: ['CONDICION ']");
    let d = detalles(Contrato::desde_yaml(&texto, "x.yaml"));
    assert!(d.iter().any(|x| x.contains("CONDICION")), "{d:?}");
}

#[test]
fn fecha_en_nombre_sin_grupo() {
    let texto = con(
        r"patron_nombre: 'Planilla (?P<periodo>\d{4}-\d{2})\.xlsx'",
        "patron_nombre: 'Planilla.*'",
    );
    let d = detalles(Contrato::desde_yaml(&texto, "x.yaml"));
    assert!(d.iter().any(|x| x.contains("periodo")), "{d:?}");
}

#[test]
fn regex_invalida() {
    let texto = con(
        r"patron_nombre: 'Planilla (?P<periodo>\d{4}-\d{2})\.xlsx'",
        "patron_nombre: '(['",
    );
    assert!(!detalles(Contrato::desde_yaml(&texto, "x.yaml")).is_empty());
}

#[test]
fn acumula_varios_detalles() {
    let texto = con("version: 1", "version: 9").replace("fuente: planilla", "fuente: Planilla");
    assert!(detalles(Contrato::desde_yaml(&texto, "x.yaml")).len() >= 2);
}

#[test]
fn reglas_por_tipo_y_clave_unica() {
    let texto = con(
        "        tipo: texto\n",
        "        tipo: entero\n        valores: [A]\n        clave: true\n",
    );
    let d = detalles(Contrato::desde_yaml(&texto, "x.yaml"));
    assert!(d.iter().any(|x| x.contains("valores")), "{d:?}");
    assert!(d.iter().any(|x| x.contains("clave")), "{d:?}");
}

#[test]
fn carpeta_con_fuente_que_no_coincide() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("otra.yaml"), MINIMO).unwrap();
    let d = detalles(cargar_contratos(dir.path()));
    assert!(d.iter().any(|x| x.contains("otra")), "{d:?}");
}

#[test]
fn carpeta_valida() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("planilla.yaml"), MINIMO).unwrap();
    std::fs::write(dir.path().join("LEAME.md"), "no es contrato").unwrap();
    let todos = cargar_contratos(dir.path()).unwrap();
    assert_eq!(todos.keys().collect::<Vec<_>>(), vec!["planilla"]);
}

#[test]
fn nombre_de_hoja_vacio() {
    for vacio in ["nombre: []", "nombre: '  '", "nombre: [Planilla, '']"] {
        let d = detalles(Contrato::desde_yaml(
            &con("nombre: Planilla", vacio),
            "x.yaml",
        ));
        assert!(
            d.iter().any(|x| x.contains("nombre de hoja")),
            "{vacio}: {d:?}"
        );
    }
}

#[test]
fn origen_repetido_en_misma_columna() {
    let d = detalles(Contrato::desde_yaml(
        &con("origen: ['DNI/CEX']", "origen: ['DNI/CEX', 'dni/cex']"),
        "x.yaml",
    ));
    assert!(d.iter().any(|x| x.contains("repite")), "{d:?}");
    assert!(!d.iter().any(|x| x.contains("se confunde")), "{d:?}");
}

#[test]
fn filas_resumen_vacio_es_invalido() {
    let texto = con(
        "    cabecera: A6
",
        "    cabecera: A6
    filas_resumen: ['']
",
    );
    let d = detalles(Contrato::desde_yaml(&texto, "x.yaml"));
    assert!(d.iter().any(|x| x.contains("filas_resumen")), "{d:?}");
}
