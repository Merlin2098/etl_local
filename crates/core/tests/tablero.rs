//! Plan 5: indicadores del tablero (indicadores-tablero.md) contra el manifiesto del demo.
mod comun;

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use comun::{base_personal, licencias, manifiesto, planilla};
use etl_local_core::error::ErrorTablero;
use etl_local_core::proceso::{Entradas, procesar};
use etl_local_core::tablero::respuesta::{Comparacion, Indicador, MejorSi, RespuestaIndicadores};
use etl_local_core::tablero::{Filtros, Tablero};

const CONFIG: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../config");

fn periodos() -> Vec<String> {
    manifiesto("limpio")["periodos"]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect()
}

fn entradas(modo: &str) -> Entradas {
    Entradas {
        planillas: periodos().iter().map(|p| planilla(modo, p)).collect(),
        licencias: Some(licencias(modo)),
        base_personal: Some(base_personal(modo)),
        otras: Default::default(),
    }
}

/// Procesa un modo una sola vez por binario de test y devuelve la carpeta de salida.
fn salida(modo: &str) -> &'static Path {
    static LIMPIO: OnceLock<tempfile::TempDir> = OnceLock::new();
    static SUCIO: OnceLock<tempfile::TempDir> = OnceLock::new();
    let celda = if modo == "limpio" { &LIMPIO } else { &SUCIO };
    celda
        .get_or_init(|| {
            let dir = tempfile::tempdir().unwrap();
            procesar(Path::new(CONFIG), &entradas(modo), dir.path(), "m1").unwrap();
            dir
        })
        .path()
}

fn tablero(modo: &str) -> Tablero {
    Tablero::nuevo(Path::new(CONFIG), salida(modo))
}

fn del_periodo(p: &str) -> Filtros {
    Filtros {
        periodo: Some(p.into()),
        ..Filtros::default()
    }
}

fn ind<'a>(r: &'a RespuestaIndicadores, id: &str) -> &'a Indicador {
    r.indicadores
        .iter()
        .find(|i| i.id == id)
        .unwrap_or_else(|| panic!("falta {id}"))
}

fn parte(i: &Indicador, categoria: &str) -> f64 {
    i.desglose
        .iter()
        .find(|p| p.categoria == categoria)
        .unwrap_or_else(|| panic!("{}: falta la parte {categoria}", i.id))
        .valor
}

fn cerca(a: f64, b: f64, tol: f64) {
    assert!((a - b).abs() <= tol, "{a} ≠ {b}");
}

#[test]
fn ind_01_a_03_cuadran_con_manifiesto() {
    let m = manifiesto("limpio");
    for p in periodos() {
        let r = tablero("limpio").indicadores(&del_periodo(&p)).unwrap();
        let esperado = &m["periodos"][&p];
        assert_eq!(
            ind(&r, "IND-01").valor,
            esperado["personas_planilla"].as_f64(),
            "{p}"
        );
        assert_eq!(
            ind(&r, "IND-02").valor,
            esperado["entradas"].as_f64(),
            "{p}"
        );
        assert_eq!(ind(&r, "IND-03").valor, esperado["salidas"].as_f64(), "{p}");
        let suma: f64 = ind(&r, "IND-03").desglose.iter().map(|d| d.valor).sum();
        assert_eq!(Some(suma), ind(&r, "IND-03").valor, "{p}");
    }
}

#[test]
fn ind_04_rotacion() {
    let r = tablero("limpio")
        .indicadores(&del_periodo("2025-10"))
        .unwrap();
    cerca(
        ind(&r, "IND-04").valor.unwrap(),
        9.0 / ((1224.0 + 1226.0) / 2.0),
        1e-12,
    );
}

#[test]
fn ind_05_ausentismo() {
    let m = manifiesto("limpio");
    for (p, dias) in periodos().iter().zip([30.0, 31.0, 30.0]) {
        let r = tablero("limpio").indicadores(&del_periodo(p)).unwrap();
        let i = ind(&r, "IND-05");
        for (categoria, valor) in m["periodos"][p]["dias_ausencia"].as_object().unwrap() {
            assert_eq!(
                parte(i, categoria),
                valor.as_f64().unwrap(),
                "{p} {categoria}"
            );
        }
        assert_eq!(parte(i, "dias_subsidio_otros"), 0.0, "{p}");
        assert_eq!(i.desglose.len(), 7, "{p}");
        let suma: f64 = i.desglose.iter().map(|d| d.valor).sum();
        let personas = ind(&r, "IND-01").valor.unwrap();
        cerca(i.valor.unwrap(), suma / (personas * dias), 1e-12);
    }
}

#[test]
fn ind_06_horas_extra() {
    let m = manifiesto("limpio");
    for p in periodos() {
        let r = tablero("limpio").indicadores(&del_periodo(&p)).unwrap();
        let i = ind(&r, "IND-06");
        for (categoria, valor) in m["periodos"][&p]["horas_extra"].as_object().unwrap() {
            cerca(parte(i, categoria), valor.as_f64().unwrap(), 1e-6);
        }
        let suma: f64 = i.desglose.iter().map(|d| d.valor).sum();
        cerca(
            i.valor.unwrap(),
            suma / ind(&r, "IND-01").valor.unwrap(),
            1e-12,
        );
    }
}

#[test]
fn primer_mes_sin_anterior() {
    // Revisión 1: el primer mes no tiene con qué compararse.
    let r = tablero("limpio")
        .indicadores(&del_periodo("2025-09"))
        .unwrap();
    assert_eq!(r.periodo_anterior, None);
    let rotacion = ind(&r, "IND-04");
    assert_eq!(rotacion.valor, None);
    assert!(
        rotacion
            .nota
            .as_deref()
            .unwrap_or("")
            .contains("mes anterior"),
        "{:?}",
        rotacion.nota
    );
    for i in &r.indicadores[..6] {
        assert_eq!(i.anterior, None, "{}", i.id);
        assert_eq!(i.variacion, None, "{}", i.id);
        assert_eq!(i.comparacion, None, "{}", i.id);
    }
    let json = serde_json::to_string(&r).unwrap();
    assert!(!json.contains("NaN") && !json.contains("inf"), "{json}");
}

#[test]
fn comparacion_segun_sentido() {
    let r = tablero("limpio")
        .indicadores(&del_periodo("2025-11"))
        .unwrap();
    assert_eq!(r.periodo_anterior.as_deref(), Some("2025-10"));
    for i in &r.indicadores[..6] {
        let (valor, anterior) = (i.valor.unwrap(), i.anterior.unwrap());
        cerca(i.variacion.unwrap(), valor - anterior, 1e-12);
        let esperada = match i.mejor_si {
            MejorSi::Neutro => None,
            _ if valor == anterior => Some(Comparacion::Igual),
            MejorSi::Baja if valor < anterior => Some(Comparacion::Mejor),
            MejorSi::Sube if valor > anterior => Some(Comparacion::Mejor),
            _ => Some(Comparacion::Peor),
        };
        assert_eq!(i.comparacion, esperada, "{}", i.id);
    }
    assert_eq!(ind(&r, "IND-01").mejor_si, MejorSi::Neutro);
    assert_eq!(ind(&r, "IND-04").mejor_si, MejorSi::Baja);
}

#[test]
fn periodo_por_defecto_es_el_ultimo() {
    let r = tablero("limpio").indicadores(&Filtros::default()).unwrap();
    assert_eq!(r.periodo.as_deref(), Some("2025-11"));
    assert_eq!(r.filtros.periodo.as_deref(), Some("2025-11"));
    assert_eq!(r.indicadores.len(), 8);
}

#[test]
fn origen_cita_reglas_y_archivos() {
    let r = tablero("limpio").indicadores(&Filtros::default()).unwrap();
    let o = &ind(&r, "IND-05").origen;
    assert_eq!(o.fuente, "planilla");
    assert_eq!(o.archivos.len(), 3, "{:?}", o.archivos);
    assert!(o.reglas.iter().any(|r| r == "IND-05"), "{:?}", o.reglas);
    assert_eq!(o.periodo.as_deref(), Some("2025-11"));
}

#[test]
fn sucio_igual_a_limpio() {
    for p in periodos() {
        let a = tablero("limpio").indicadores(&del_periodo(&p)).unwrap();
        let b = tablero("sucio").indicadores(&del_periodo(&p)).unwrap();
        for (x, y) in a.indicadores[..6].iter().zip(&b.indicadores[..6]) {
            assert_eq!(x.valor, y.valor, "{p} {}", x.id);
            let partes = |i: &Indicador| {
                i.desglose
                    .iter()
                    .map(|d| (d.categoria.clone(), d.valor))
                    .collect::<Vec<_>>()
            };
            assert_eq!(partes(x), partes(y), "{p} {}", x.id);
        }
    }
}

/// Copia `config/` a un temporal para editar un archivo.
fn copiar_config() -> tempfile::TempDir {
    fn copiar(origen: &Path, destino: &Path) {
        std::fs::create_dir_all(destino).unwrap();
        for e in std::fs::read_dir(origen).unwrap() {
            let e = e.unwrap();
            let d: PathBuf = destino.join(e.file_name());
            if e.file_type().unwrap().is_dir() {
                copiar(&e.path(), &d);
            } else {
                std::fs::copy(e.path(), d).unwrap();
            }
        }
    }
    let dir = tempfile::tempdir().unwrap();
    copiar(Path::new(CONFIG), dir.path());
    dir
}

#[test]
fn meta_desde_parametros() {
    let config = copiar_config();
    let ruta = config.path().join("parametros.yaml");
    let texto = std::fs::read_to_string(&ruta)
        .unwrap()
        .replace("metas: {}", "metas: {IND-04: 0.02}");
    std::fs::write(&ruta, texto).unwrap();
    let r = Tablero::nuevo(config.path(), salida("limpio"))
        .indicadores(&Filtros::default())
        .unwrap();
    assert_eq!(ind(&r, "IND-04").meta, Some(0.02));
    assert!(
        r.indicadores
            .iter()
            .filter(|i| i.id != "IND-04")
            .all(|i| i.meta.is_none())
    );
}

#[test]
fn catalogo_invalido() {
    let config = copiar_config();
    let ruta = config.path().join("indicadores.yaml");
    let texto = std::fs::read_to_string(&ruta).unwrap();
    let corte = texto.find("  - id: IND-08").expect("IND-08 en el catálogo");
    std::fs::write(&ruta, &texto[..corte]).unwrap();
    match Tablero::nuevo(config.path(), salida("limpio")).indicadores(&Filtros::default()) {
        Err(ErrorTablero::Contrato(e)) => assert!(e.to_string().contains("IND-08"), "{e}"),
        otro => panic!("{otro:?}"),
    }
}

// ---- Tarea 3: IND-07/08, filtros de dimensión y fuentes ausentes ----

fn con(gerencia: Option<&str>, sede: Option<&str>) -> Filtros {
    Filtros {
        periodo: Some("2025-11".into()),
        gerencia: gerencia.map(str::to_string),
        sede: sede.map(str::to_string),
        tipo_trabajador: None,
    }
}

/// Valores distintos de una columna del Parquet Gold de la base (para armar filtros en los tests).
fn de_la_base(sql: &str) -> Vec<Vec<Option<String>>> {
    let a = etl_local_core::almacen::Almacen::nuevo().unwrap();
    a.vista_parquet(
        "base",
        &salida("limpio").join("actual/gold/base_personal_empleados.parquet"),
    )
    .unwrap();
    a.consultar_texto(sql).unwrap()
}

fn gerencias() -> Vec<String> {
    de_la_base("SELECT DISTINCT gerencia FROM base WHERE gerencia IS NOT NULL ORDER BY 1")
        .into_iter()
        .filter_map(|mut f| f.remove(0))
        .collect()
}

#[test]
fn ind_07_y_08_cuadran_con_manifiesto() {
    let m = manifiesto("limpio");
    let r = tablero("limpio").indicadores(&Filtros::default()).unwrap();
    let contratos = ind(&r, "IND-07");
    assert_eq!(
        contratos.valor,
        m["base_personal"]["casos_contrato_plazo"].as_f64()
    );
    assert_eq!(parte(contratos, "alerta_contrato_obra"), 11.0);
    assert_eq!(parte(contratos, "alerta_contrato_incremento"), 7.0);
    let jubilacion = ind(&r, "IND-08");
    assert_eq!(
        jubilacion.valor,
        m["base_personal"]["casos_jubilacion"].as_f64()
    );
    assert_eq!(
        parte(jubilacion, "cumple_65_este_anio") + parte(jubilacion, "cumple_65_proximo_anio"),
        9.0
    );
    assert_eq!(
        parte(jubilacion, "cumple_70_este_anio") + parte(jubilacion, "cumple_70_proximo_anio"),
        4.0
    );
    let corte = chrono::NaiveDate::from_ymd_opt(2025, 11, 12);
    assert_eq!(contratos.origen.fecha_corte, corte);
    assert_eq!(r.fecha_corte, corte);
    // No responden al periodo (indicadores-tablero §2).
    let antes = tablero("limpio")
        .indicadores(&del_periodo("2025-09"))
        .unwrap();
    assert_eq!(ind(&antes, "IND-07").valor, contratos.valor);
}

#[test]
fn filtro_por_gerencia_reparte_el_total() {
    // IND-C1: quien no está en la base no aparece al filtrar por gerencia.
    let total = tablero("limpio").indicadores(&con(None, None)).unwrap();
    let (mut personas, mut contratos) = (0.0, 0.0);
    for g in gerencias() {
        let r = tablero("limpio").indicadores(&con(Some(&g), None)).unwrap();
        personas += ind(&r, "IND-01").valor.unwrap();
        contratos += ind(&r, "IND-07").valor.unwrap();
    }
    let sin_base = manifiesto("limpio")["periodos"]["2025-11"]["personas_sin_base"]
        .as_f64()
        .unwrap();
    assert_eq!(personas + sin_base, ind(&total, "IND-01").valor.unwrap());
    assert_eq!(contratos, ind(&total, "IND-07").valor.unwrap());
}

#[test]
fn filtro_inexistente_es_invalido() {
    // Revisión 2: un valor mal escrito o malicioso nunca da 200 con ceros ni un error de SQL.
    let opciones = gerencias();
    for valor in ["operaciones", "x' OR '1'='1"] {
        match tablero("limpio").indicadores(&con(Some(valor), None)) {
            Err(ErrorTablero::FiltroInvalido {
                filtro,
                opciones: o,
                ..
            }) => {
                assert_eq!(filtro, "gerencia");
                assert_eq!(o, opciones);
            }
            otro => panic!("{valor}: {otro:?}"),
        }
    }
    for p in ["2030-01", "2025-13"] {
        match tablero("limpio").indicadores(&del_periodo(p)) {
            Err(ErrorTablero::FiltroInvalido { filtro, .. }) => assert_eq!(filtro, "periodo"),
            otro => panic!("{p}: {otro:?}"),
        }
    }
}

#[test]
fn combinacion_sin_personas() {
    // Revisión 3: una combinación de filtros sin nadie da conteos en 0 y tasas en null, nunca NaN.
    // (En el demo todo par gerencia-sede tiene personas; se busca una terna con el tipo de trabajador.)
    let vacia = de_la_base(
        "SELECT g.gerencia, s.sede, t.tipo_trabajador          FROM (SELECT DISTINCT gerencia FROM base WHERE gerencia IS NOT NULL) AS g          CROSS JOIN (SELECT DISTINCT sede FROM base WHERE sede IS NOT NULL) AS s          CROSS JOIN (SELECT DISTINCT tipo_trabajador FROM base WHERE tipo_trabajador IS NOT NULL) AS t          WHERE NOT EXISTS (SELECT 1 FROM base AS b WHERE b.gerencia = g.gerencia AND b.sede = s.sede                            AND b.tipo_trabajador = t.tipo_trabajador)          ORDER BY 1, 2, 3 LIMIT 1",
    );
    let fila = vacia
        .first()
        .expect("el fixture no tiene una combinación vacía: elegir otro caso");
    let filtros = Filtros {
        tipo_trabajador: fila[2].clone(),
        ..con(fila[0].as_deref(), fila[1].as_deref())
    };
    let r = tablero("limpio").indicadores(&filtros).unwrap();
    for id in ["IND-01", "IND-02", "IND-03", "IND-07", "IND-08"] {
        assert_eq!(ind(&r, id).valor, Some(0.0), "{id}");
    }
    for id in ["IND-04", "IND-05", "IND-06"] {
        assert_eq!(ind(&r, id).valor, None, "{id}");
    }
    let json = serde_json::to_string(&r).unwrap();
    assert!(!json.contains("NaN"), "{json}");
}

#[test]
fn solo_planillas() {
    // Revisión 4: sin base de personal, IND-07/08 quedan en null con nota.
    let dir = tempfile::tempdir().unwrap();
    let entradas = Entradas {
        planillas: periodos().iter().map(|p| planilla("limpio", p)).collect(),
        ..Entradas::default()
    };
    procesar(Path::new(CONFIG), &entradas, dir.path(), "m1").unwrap();
    let r = Tablero::nuevo(Path::new(CONFIG), dir.path())
        .indicadores(&Filtros::default())
        .unwrap();
    assert!(ind(&r, "IND-01").valor.is_some());
    for id in ["IND-07", "IND-08"] {
        let i = ind(&r, id);
        assert_eq!(i.valor, None, "{id}");
        assert!(
            i.nota.as_deref().unwrap_or("").contains("base de personal"),
            "{id}: {:?}",
            i.nota
        );
    }
}

#[test]
fn sin_resultados_es_sin_datos() {
    let dir = tempfile::tempdir().unwrap();
    match Tablero::nuevo(Path::new(CONFIG), dir.path()).indicadores(&Filtros::default()) {
        Err(ErrorTablero::SinDatos(_)) => {}
        otro => panic!("{otro:?}"),
    }
}

// ---- Tarea 4: estado, opciones de filtro, series, checks y detalle ----

use etl_local_core::tablero::Detalle;

#[test]
fn estado_datos_usados() {
    let e = tablero("limpio").estado().unwrap();
    assert_eq!(e.fecha_corte, chrono::NaiveDate::from_ymd_opt(2025, 11, 12));
    assert_eq!(e.periodos, periodos());
    assert!(!e.periodos_mezclados);
    assert_eq!(e.aviso_periodos, None);
    assert_eq!(e.avisos, 0);
    assert_eq!(e.marca, "m1");

    // IND-C2 (TR-D16): base de personal de otro mes que la última planilla.
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("actual")).unwrap();
    let texto = std::fs::read_to_string(salida("limpio").join("actual/ejecucion.json"))
        .unwrap()
        .replace("\"2025-11-12\"", "\"2025-12-05\"");
    std::fs::write(dir.path().join("actual/ejecucion.json"), texto).unwrap();
    let e = Tablero::nuevo(Path::new(CONFIG), dir.path())
        .estado()
        .unwrap();
    assert!(e.periodos_mezclados);
    let aviso = e.aviso_periodos.unwrap();
    assert!(
        aviso.contains("2025-11") && aviso.contains("2025-12-05"),
        "{aviso}"
    );
}

#[test]
fn opciones_de_filtro() {
    let o = tablero("limpio").filtros().unwrap();
    assert_eq!(o.periodos, periodos());
    assert_eq!(o.gerencias, gerencias());
    let columna = |c: &str| -> Vec<String> {
        de_la_base(&format!(
            "SELECT DISTINCT {c} FROM base WHERE {c} IS NOT NULL ORDER BY 1"
        ))
        .into_iter()
        .filter_map(|mut f| f.remove(0))
        .collect()
    };
    assert_eq!(o.sedes, columna("sede"));
    assert_eq!(o.tipos_trabajador, columna("tipo_trabajador"));
}

#[test]
fn series_cuadran_con_indicadores() {
    let s = tablero("limpio").series(&Filtros::default()).unwrap();
    let ids: Vec<_> = s.series.iter().map(|x| x.id.as_str()).collect();
    assert_eq!(
        ids,
        ["IND-01", "IND-02", "IND-03", "IND-04", "IND-05", "IND-06"]
    );
    for p in periodos() {
        let r = tablero("limpio").indicadores(&del_periodo(&p)).unwrap();
        for serie in &s.series {
            let punto = serie.puntos.iter().find(|x| x.periodo == p).unwrap();
            assert_eq!(punto.valor, ind(&r, &serie.id).valor, "{p} {}", serie.id);
        }
    }
    let rotacion = s.series.iter().find(|x| x.id == "IND-04").unwrap();
    assert_eq!(rotacion.puntos[0].valor, None);
}

#[test]
fn ind_c1_y_c3() {
    let m = manifiesto("limpio");
    for p in periodos() {
        let c = tablero("limpio").checks(&del_periodo(&p)).unwrap();
        let c1 = c.checks.iter().find(|x| x.id == "IND-C1").unwrap();
        assert_eq!(
            c1.valor,
            m["periodos"][&p]["personas_sin_base"].as_f64(),
            "{p}"
        );
        assert!(c1.mensaje.contains("gerencia o sede"), "{}", c1.mensaje);
        for x in &c.checks {
            assert!(!x.mensaje.contains("  "), "espacios de más: {}", x.mensaje);
        }
    }
    let c3 = |p: &str| {
        tablero("limpio")
            .checks(&del_periodo(p))
            .unwrap()
            .checks
            .into_iter()
            .find(|x| x.id == "IND-C3")
            .unwrap()
    };
    assert_eq!(c3("2025-10").valor, Some((1224.0 - 1226.0) - (16.0 - 9.0)));
    let primero = c3("2025-09");
    assert_eq!(primero.valor, None);
    assert!(
        primero.mensaje.contains("mes anterior"),
        "{}",
        primero.mensaje
    );
}

#[test]
fn detalle_alertas_coincide_con_ind_07_08() {
    let d = tablero("limpio")
        .detalle(Detalle::Alertas, &Filtros::default())
        .unwrap();
    let esperadas =
        de_la_base("SELECT count(*) FROM base WHERE alerta_contrato_plazo OR alerta_jubilacion")[0]
            [0]
        .clone()
        .unwrap();
    assert_eq!(d.filas.len().to_string(), esperadas);
    assert!(d.filas.iter().all(|f| f.contains_key("nombre_completo")));
}

#[test]
fn detalle_sin_base_del_periodo() {
    let d = tablero("limpio")
        .detalle(Detalle::SinBase, &del_periodo("2025-11"))
        .unwrap();
    assert_eq!(d.filas.len(), 8);
    assert_eq!(d.periodo.as_deref(), Some("2025-11"));
    assert!(d.filas.iter().all(|f| f.contains_key("numero_documento")));
}

#[test]
fn solo_detalle_trae_nombres() {
    // TR-D18: estado, filtros, indicadores, series y checks nunca llevan nombres ni documentos.
    let t = tablero("limpio");
    let f = Filtros::default();
    let textos = [
        serde_json::to_string(&t.estado().unwrap()).unwrap(),
        serde_json::to_string(&t.filtros().unwrap()).unwrap(),
        serde_json::to_string(&t.indicadores(&f).unwrap()).unwrap(),
        serde_json::to_string(&t.series(&f).unwrap()).unwrap(),
        serde_json::to_string(&t.checks(&f).unwrap()).unwrap(),
    ];
    let personas = de_la_base("SELECT numero_documento, nombre_completo FROM base LIMIT 50");
    assert_eq!(personas.len(), 50);
    for fila in personas {
        for dato in fila.into_iter().flatten() {
            for texto in &textos {
                assert!(
                    !texto.contains(&dato),
                    "«{dato}» aparece fuera de /detalle/"
                );
            }
        }
    }
}

// ---- Revisión final ----

#[test]
fn ejecucion_ilegible_no_es_sin_datos() {
    // Un error de lectura que no es «no existe» no se informa como «procese primero» (AGENTS.md §4.6).
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("actual/ejecucion.json")).unwrap();
    match Tablero::nuevo(Path::new(CONFIG), dir.path()).estado() {
        Err(ErrorTablero::SinDatos(_)) | Ok(_) => panic!("debía ser un error de lectura"),
        Err(_) => {}
    }
}

#[test]
fn parquet_faltante_de_fuente_procesada_es_error() {
    // ejecucion.json dice que hubo base de personal pero su Parquet no está (actual/ a medio reemplazar):
    // error explícito, nunca «No hay base de personal procesada».
    let dir = tempfile::tempdir().unwrap();
    let origen = salida("limpio").join("actual");
    let destino = dir.path().join("actual");
    std::fs::create_dir_all(destino.join("gold")).unwrap();
    std::fs::copy(
        origen.join("ejecucion.json"),
        destino.join("ejecucion.json"),
    )
    .unwrap();
    std::fs::copy(
        origen.join("gold/planilla.parquet"),
        destino.join("gold/planilla.parquet"),
    )
    .unwrap();
    match Tablero::nuevo(Path::new(CONFIG), dir.path()).indicadores(&Filtros::default()) {
        // Plan 7: es «actualizando» (la API responde 503 y el tablero reintenta).
        Err(e @ ErrorTablero::Actualizando { .. }) => {
            assert!(e.to_string().contains("base_personal_empleados"), "{e}")
        }
        otro => panic!("{otro:?}"),
    }
}

#[test]
fn validar_config_nombra_el_archivo() {
    let vacia = tempfile::tempdir().unwrap();
    let error = Tablero::nuevo(vacia.path(), salida("limpio"))
        .validar_config()
        .unwrap_err()
        .to_string();
    assert!(error.contains("indicadores.yaml"), "{error}");
    Tablero::nuevo(Path::new(CONFIG), salida("limpio"))
        .validar_config()
        .unwrap();
}

// Plan 7, Tarea 1: desglose por gerencia de IND-01, IND-07 e IND-08.
#[test]
fn por_gerencia_de_ind01_mas_sin_base_da_el_total() {
    let t = tablero("limpio");
    let r = t.indicadores(&Filtros::default()).unwrap();
    let c1 = t
        .checks(&Filtros::default())
        .unwrap()
        .checks
        .into_iter()
        .find(|c| c.id == "IND-C1")
        .unwrap()
        .valor
        .unwrap();
    let ind01 = ind(&r, "IND-01");
    let suma: f64 = ind01.por_gerencia.iter().map(|p| p.valor).sum();
    assert!(!ind01.por_gerencia.is_empty());
    assert_eq!(suma + c1, ind01.valor.unwrap());
    assert!(
        ind01
            .por_gerencia
            .windows(2)
            .all(|w| w[0].valor >= w[1].valor)
    );
}

#[test]
fn por_gerencia_de_alertas_suma_el_total() {
    let r = tablero("limpio").indicadores(&Filtros::default()).unwrap();
    for id in ["IND-07", "IND-08"] {
        let i = ind(&r, id);
        assert_eq!(
            i.por_gerencia.iter().map(|p| p.valor).sum::<f64>(),
            i.valor.unwrap(),
            "{id}"
        );
    }
    assert!(ind(&r, "IND-04").por_gerencia.is_empty());
}

#[test]
fn por_gerencia_respeta_el_filtro_de_gerencia() {
    let t = tablero("limpio");
    let g = t.filtros().unwrap().gerencias[0].clone();
    let f = Filtros {
        gerencia: Some(g.clone()),
        ..Default::default()
    };
    let r = t.indicadores(&f).unwrap();
    let i = ind(&r, "IND-01");
    assert_eq!(i.por_gerencia.len(), 1);
    assert_eq!(i.por_gerencia[0].categoria, g);
    assert_eq!(i.por_gerencia[0].valor, i.valor.unwrap());
}
