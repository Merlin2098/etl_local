//! Tablero (Plan 5, ADR-0004): indicadores IND-01…08 y checks IND-C1…C3 sobre `actual/`.
//!
//! Las fórmulas viven en `config/sql/indicadores/`. Aquí solo se abren los Parquet, se fijan los filtros,
//! se ejecuta el SQL y se arman las respuestas con los metadatos de `config/indicadores.yaml`.
//! Funciones síncronas y de solo lectura: nunca escriben en la carpeta de salida.

pub mod esquema;
pub mod respuesta;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::almacen::Almacen;
use crate::error::{ErrorContrato, ErrorTablero};
use crate::parametros::cargar_parametros;
use crate::proceso::{Ejecucion, ejecutar_carpeta};
use respuesta::{
    Check, Comparacion, Estado, Indicador, MejorSi, OpcionesFiltro, Origen, Parte, Punto,
    RespuestaChecks, RespuestaDetalle, RespuestaIndicadores, RespuestaSeries, Serie,
};

/// Detalle nominal disponible (TR-D18).
#[derive(schemars::JsonSchema, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Detalle {
    /// Personas con alerta de contrato o de jubilación (IND-07, IND-08).
    Alertas,
    /// Personas de la planilla sin registro en la base de personal (IND-C1).
    SinBase,
}

/// Filtros globales del tablero. Un campo ausente significa "todos" (el periodo: el último).
#[derive(schemars::JsonSchema, Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Filtros {
    pub periodo: Option<String>,
    pub gerencia: Option<String>,
    pub sede: Option<String>,
    pub tipo_trabajador: Option<String>,
}

/// Ficha de `config/indicadores.yaml` (Plan 5, decisión 1).
#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
struct Ficha {
    id: String,
    nombre: String,
    pregunta: String,
    unidad: String,
    mejor_si: MejorSi,
    fuente: String,
    reglas: Vec<String>,
    #[serde(default)]
    etiquetas: BTreeMap<String, String>,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
struct Catalogo {
    version: u32,
    indicadores: Vec<Ficha>,
}

const IDS: [&str; 8] = [
    "IND-01", "IND-02", "IND-03", "IND-04", "IND-05", "IND-06", "IND-07", "IND-08",
];

fn cargar_catalogo(ruta: &Path) -> Result<Vec<Ficha>, ErrorContrato> {
    let origen = ruta.display().to_string();
    let texto = std::fs::read_to_string(ruta).map_err(|fuente| ErrorContrato::Lectura {
        ruta: ruta.to_path_buf(),
        fuente,
    })?;
    let c: Catalogo = serde_saphyr::from_str(&texto).map_err(|e| ErrorContrato::Yaml {
        origen: origen.clone(),
        mensaje: e.to_string(),
    })?;
    let mut detalles = Vec::new();
    if c.version != 1 {
        detalles.push(format!(
            "version {} no soportada (este motor entiende la 1)",
            c.version
        ));
    }
    let ids: Vec<&str> = c.indicadores.iter().map(|f| f.id.as_str()).collect();
    if ids != IDS {
        detalles.push(format!(
            "los indicadores deben ser {} en ese orden (hay: {})",
            IDS.join(", "),
            ids.join(", ")
        ));
    }
    for f in &c.indicadores {
        if !["personas", "fraccion", "horas"].contains(&f.unidad.as_str()) {
            detalles.push(format!(
                "{}: unidad «{}» no válida (personas, fraccion u horas)",
                f.id, f.unidad
            ));
        }
        if !["planilla", "base_personal"].contains(&f.fuente.as_str()) {
            detalles.push(format!(
                "{}: fuente «{}» no válida (planilla o base_personal)",
                f.id, f.fuente
            ));
        }
        if f.nombre.trim().is_empty() || f.pregunta.trim().is_empty() {
            detalles.push(format!(
                "{}: nombre y pregunta no pueden estar vacíos",
                f.id
            ));
        }
    }
    if detalles.is_empty() {
        Ok(c.indicadores)
    } else {
        Err(ErrorContrato::Invalido { origen, detalles })
    }
}

/// Acceso al tablero de una carpeta de salida de `etl-local procesar`.
#[derive(Debug, Clone)]
pub struct Tablero {
    config: PathBuf,
    salida: PathBuf,
}

/// Base abierta para un pedido: vistas sobre `actual/`, filtros fijados y SQL ejecutado.
struct Abierto {
    almacen: Almacen,
    ejecucion: Ejecucion,
    filtros: Filtros,
    hay_planilla: bool,
    hay_base: bool,
}

/// Tablas Gold que lee el tablero (`actual/gold/<tabla>.parquet`).
const PLANILLA: &str = "planilla";
const BASE: &str = "base_personal_empleados";

impl Tablero {
    /// `config`: carpeta con `indicadores.yaml`, `parametros.yaml` y `sql/indicadores/`.
    /// `salida`: carpeta de salida de `procesar` (la que contiene `actual/`).
    pub fn nuevo(config: &Path, salida: &Path) -> Tablero {
        Tablero {
            config: config.to_path_buf(),
            salida: salida.to_path_buf(),
        }
    }

    /// Comprueba `indicadores.yaml` y `parametros.yaml` antes de levantar el servidor.
    pub fn validar_config(&self) -> Result<(), ErrorTablero> {
        cargar_catalogo(&self.config.join("indicadores.yaml"))?;
        cargar_parametros(&self.config.join("parametros.yaml"))?;
        Ok(())
    }

    fn actual(&self) -> PathBuf {
        self.salida.join("actual")
    }

    fn sql(&self) -> PathBuf {
        self.config.join("sql").join("indicadores")
    }

    fn leer_ejecucion(&self) -> Result<Ejecucion, ErrorTablero> {
        let ruta = self.actual().join("ejecucion.json");
        // Solo «no existe» es «todavía no hay resultados»; otro error de lectura se informa tal cual.
        let texto = std::fs::read_to_string(&ruta).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                ErrorTablero::SinDatos(self.actual())
            } else {
                ErrorTablero::Proceso(crate::error::ErrorProceso::Datos(format!(
                    "No se pudo leer {}: {e}",
                    ruta.display()
                )))
            }
        })?;
        serde_json::from_str(&texto).map_err(|e| {
            ErrorTablero::Proceso(crate::error::ErrorProceso::Datos(format!(
                "{} no es válido: {e}",
                ruta.display()
            )))
        })
    }

    /// Valores distintos y no nulos de una columna (constante del programa) de una tabla.
    fn distintos(a: &Almacen, tabla: &str, columna: &str) -> Result<Vec<String>, ErrorTablero> {
        let sql = format!(
            "SELECT DISTINCT {columna} FROM {tabla} WHERE {columna} IS NOT NULL ORDER BY 1"
        );
        Ok(a.consultar_texto(&sql)?
            .into_iter()
            .filter_map(|mut f| f.remove(0))
            .collect())
    }

    fn abrir(&self, f: &Filtros) -> Result<Abierto, ErrorTablero> {
        let ejecucion = self.leer_ejecucion()?;
        let almacen = Almacen::nuevo()?;
        // Qué fuentes hay lo dice ejecucion.json, no la presencia de archivos: si una fuente procesada no
        // tiene su Parquet, actual/ se está reemplazando y se informa, en vez de mostrarla como ausente.
        // Los Parquet se copian a memoria enseguida: una sola lectura, consistente, y sin archivos abiertos.
        let hay = |tabla: &str, fuente: &str| -> Result<bool, ErrorTablero> {
            if !ejecucion.fuentes.iter().any(|x| x.fuente == fuente) {
                almacen
                    .ejecutar_archivo(&self.sql().join("vacias").join(format!("{tabla}.sql")))?;
                return Ok(false);
            }
            let ruta = self.actual().join("gold").join(format!("{tabla}.parquet"));
            if !ruta.is_file() {
                return Err(ErrorTablero::Actualizando {
                    ruta,
                    fuente: fuente.to_string(),
                });
            }
            almacen.tabla_parquet(tabla, &ruta)?;
            Ok(true)
        };
        let hay_planilla = hay(PLANILLA, "planilla")?;
        let hay_base = hay(BASE, "base_personal")?;

        // Plan 5, decisión 3: periodo por defecto = el último de la planilla.
        let periodos = Self::distintos(&almacen, PLANILLA, "periodo")?;
        let periodo = match &f.periodo {
            Some(p) if periodos.contains(p) => Some(p.clone()),
            Some(p) => {
                return Err(ErrorTablero::FiltroInvalido {
                    filtro: "periodo".into(),
                    valor: p.clone(),
                    opciones: periodos,
                });
            }
            None => periodos.last().cloned(),
        };
        // Plan 5, decisión 4: un valor que no está en la base (comparación exacta) es un filtro inválido.
        for (nombre, valor) in [
            ("gerencia", &f.gerencia),
            ("sede", &f.sede),
            ("tipo_trabajador", &f.tipo_trabajador),
        ] {
            if let Some(v) = valor {
                let opciones = Self::distintos(&almacen, BASE, nombre)?;
                if !opciones.contains(v) {
                    return Err(ErrorTablero::FiltroInvalido {
                        filtro: nombre.into(),
                        valor: v.clone(),
                        opciones,
                    });
                }
            }
        }
        let filtros = Filtros {
            periodo,
            ..f.clone()
        };
        almacen.fijar_filtros(&[
            ("periodo", filtros.periodo.as_deref()),
            ("gerencia", filtros.gerencia.as_deref()),
            ("sede", filtros.sede.as_deref()),
            ("tipo_trabajador", filtros.tipo_trabajador.as_deref()),
        ])?;
        ejecutar_carpeta(&almacen, &self.sql())?;
        Ok(Abierto {
            almacen,
            ejecucion,
            filtros,
            hay_planilla,
            hay_base,
        })
    }

    /// IND-01…08 del periodo filtrado, con mes anterior, meta, desglose y origen.
    pub fn indicadores(&self, f: &Filtros) -> Result<RespuestaIndicadores, ErrorTablero> {
        let fichas = cargar_catalogo(&self.config.join("indicadores.yaml"))?;
        let metas = cargar_parametros(&self.config.join("parametros.yaml"))?.metas;
        let ab = self.abrir(f)?;
        let a = &ab.almacen;

        let periodo_anterior = a
            .consultar_texto(
                "SELECT t.periodo_anterior FROM ind_periodos AS t \
                 JOIN ind_periodos AS a ON a.periodo = t.periodo_anterior \
                 WHERE t.periodo = getvariable('f_periodo')",
            )?
            .into_iter()
            .find_map(|mut fila| fila.remove(0));
        let valores = |sql: &str| -> Result<BTreeMap<String, Option<f64>>, ErrorTablero> {
            Ok(a.consultar_json(sql)?
                .into_iter()
                .map(|fila| {
                    let id = fila["id"].as_str().unwrap_or_default().to_string();
                    (id, fila["valor"].as_f64())
                })
                .collect())
        };
        let actuales = valores(
            "SELECT id, valor FROM ind_valor \
             WHERE periodo = getvariable('f_periodo') OR periodo IS NULL",
        )?;
        let anteriores = valores(
            "SELECT v.id, v.valor FROM ind_valor AS v \
             JOIN ind_periodos AS t ON v.periodo = t.periodo_anterior \
             WHERE t.periodo = getvariable('f_periodo')",
        )?;
        let mut desgloses: BTreeMap<String, Vec<(String, f64)>> = BTreeMap::new();
        for fila in a.consultar_json(
            "SELECT id, categoria, valor FROM ind_desglose \
             WHERE periodo = getvariable('f_periodo') OR periodo IS NULL \
             ORDER BY id, orden, valor DESC, categoria",
        )? {
            desgloses
                .entry(fila["id"].as_str().unwrap_or_default().to_string())
                .or_default()
                .push((
                    fila["categoria"].as_str().unwrap_or_default().to_string(),
                    fila["valor"].as_f64().unwrap_or(0.0),
                ));
        }
        let mut por_gerencia: BTreeMap<String, Vec<Parte>> = BTreeMap::new();
        for fila in a.consultar_json(
            "SELECT id, gerencia, valor FROM ind_por_gerencia ORDER BY id, valor DESC, gerencia",
        )? {
            let gerencia = fila["gerencia"].as_str().unwrap_or_default().to_string();
            por_gerencia
                .entry(fila["id"].as_str().unwrap_or_default().to_string())
                .or_default()
                .push(Parte {
                    categoria: gerencia.clone(),
                    etiqueta: gerencia,
                    valor: fila["valor"].as_f64().unwrap_or(0.0),
                });
        }

        let archivos = |fuente: &str| -> Vec<String> {
            ab.ejecucion
                .fuentes
                .iter()
                .filter(|x| x.fuente == fuente)
                .map(|x| x.archivo.clone())
                .collect()
        };
        let indicadores = fichas
            .into_iter()
            .map(|ficha| {
                let de_planilla = ficha.fuente == "planilla";
                let falta = if de_planilla && !ab.hay_planilla {
                    Some("No hay planilla procesada.")
                } else if !de_planilla && !ab.hay_base {
                    Some("No hay base de personal procesada.")
                } else {
                    None
                };
                let valor = if falta.is_some() {
                    None
                } else {
                    actuales.get(&ficha.id).copied().flatten()
                };
                let anterior = if falta.is_some() {
                    None
                } else {
                    anteriores.get(&ficha.id).copied().flatten()
                };
                let variacion = valor.zip(anterior).map(|(v, a)| v - a);
                let comparacion = match (ficha.mejor_si, valor, anterior) {
                    (MejorSi::Neutro, _, _) => None,
                    (_, Some(v), Some(a)) if v == a => Some(Comparacion::Igual),
                    (MejorSi::Baja, Some(v), Some(a)) => Some(if v < a {
                        Comparacion::Mejor
                    } else {
                        Comparacion::Peor
                    }),
                    (MejorSi::Sube, Some(v), Some(a)) => Some(if v > a {
                        Comparacion::Mejor
                    } else {
                        Comparacion::Peor
                    }),
                    _ => None,
                };
                let nota = falta.map(str::to_string).or_else(|| {
                    (ficha.id == "IND-04" && valor.is_none() && periodo_anterior.is_none()).then(
                        || {
                            "Sin mes anterior: la rotación necesita el mes anterior en los datos."
                                .to_string()
                        },
                    )
                });
                let desglose = if falta.is_some() {
                    Vec::new()
                } else {
                    desgloses
                        .get(&ficha.id)
                        .map(|partes| {
                            partes
                                .iter()
                                .map(|(categoria, valor)| Parte {
                                    etiqueta: ficha
                                        .etiquetas
                                        .get(categoria)
                                        .cloned()
                                        .unwrap_or_else(|| categoria.clone()),
                                    categoria: categoria.clone(),
                                    valor: *valor,
                                })
                                .collect()
                        })
                        .unwrap_or_default()
                };
                let por_gerencia = if falta.is_some() {
                    Vec::new()
                } else {
                    por_gerencia.get(&ficha.id).cloned().unwrap_or_default()
                };
                Indicador {
                    por_gerencia,
                    meta: metas.get(&ficha.id).copied(),
                    origen: Origen {
                        archivos: archivos(&ficha.fuente),
                        fecha_corte: if de_planilla {
                            None
                        } else {
                            ab.ejecucion.fecha_corte
                        },
                        periodo: if de_planilla {
                            ab.filtros.periodo.clone()
                        } else {
                            None
                        },
                        fuente: ficha.fuente,
                        reglas: ficha.reglas,
                    },
                    id: ficha.id,
                    nombre: ficha.nombre,
                    pregunta: ficha.pregunta,
                    unidad: ficha.unidad,
                    valor,
                    anterior,
                    variacion,
                    mejor_si: ficha.mejor_si,
                    comparacion,
                    desglose,
                    nota,
                }
            })
            .collect();
        Ok(RespuestaIndicadores {
            periodo: ab.filtros.periodo.clone(),
            periodo_anterior,
            fecha_corte: ab.ejecucion.fecha_corte,
            filtros: ab.filtros.clone(),
            indicadores,
        })
    }

    /// "Datos usados" (TR-D16) e IND-C2 (Plan 5, decisión 7). Solo lee `ejecucion.json`.
    pub fn estado(&self) -> Result<Estado, ErrorTablero> {
        let e = self.leer_ejecucion()?;
        let ultimo = e.periodos.iter().max().cloned();
        let aviso_periodos = match (e.fecha_corte, &ultimo) {
            (Some(corte), Some(periodo)) if corte.format("%Y-%m").to_string() != *periodo => {
                Some(format!(
                    "La base de personal tiene fecha de corte {} y la última planilla es de {periodo}.",
                    corte.format("%Y-%m-%d")
                ))
            }
            _ => None,
        };
        Ok(Estado {
            marca: e.marca,
            version_motor: e.version_motor,
            fecha_corte: e.fecha_corte,
            periodos: e.periodos,
            fuentes: e.fuentes,
            avisos: e.problemas.len(),
            periodos_mezclados: aviso_periodos.is_some(),
            aviso_periodos,
        })
    }

    /// Valores posibles de cada filtro, ordenados y sin nulos.
    pub fn filtros(&self) -> Result<OpcionesFiltro, ErrorTablero> {
        let ab = self.abrir(&Filtros::default())?;
        let a = &ab.almacen;
        Ok(OpcionesFiltro {
            periodos: Self::distintos(a, PLANILLA, "periodo")?,
            gerencias: Self::distintos(a, BASE, "gerencia")?,
            sedes: Self::distintos(a, BASE, "sede")?,
            tipos_trabajador: Self::distintos(a, BASE, "tipo_trabajador")?,
        })
    }

    /// Serie mensual de IND-01…06 en todos los periodos (ignora el filtro de periodo).
    pub fn series(&self, f: &Filtros) -> Result<RespuestaSeries, ErrorTablero> {
        let fichas = cargar_catalogo(&self.config.join("indicadores.yaml"))?;
        let ab = self.abrir(&Filtros {
            periodo: None,
            ..f.clone()
        })?;
        let filas = ab.almacen.consultar_json(
            "SELECT id, periodo, valor FROM ind_valor WHERE periodo IS NOT NULL ORDER BY id, periodo",
        )?;
        let series = fichas
            .into_iter()
            .filter(|ficha| ficha.fuente == "planilla")
            .map(|ficha| Serie {
                puntos: filas
                    .iter()
                    .filter(|fila| fila["id"] == ficha.id.as_str())
                    .map(|fila| Punto {
                        periodo: fila["periodo"].as_str().unwrap_or_default().to_string(),
                        valor: fila["valor"].as_f64(),
                    })
                    .collect(),
                id: ficha.id,
                nombre: ficha.nombre,
                unidad: ficha.unidad,
            })
            .collect();
        Ok(RespuestaSeries {
            filtros: Filtros {
                periodo: None,
                ..ab.filtros
            },
            series,
        })
    }

    /// IND-C1 e IND-C3 del periodo filtrado, con su explicación.
    pub fn checks(&self, f: &Filtros) -> Result<RespuestaChecks, ErrorTablero> {
        let ab = self.abrir(f)?;
        let filas = ab.almacen.consultar_json(
            "SELECT id, valor, cambio_personas, entradas_menos_salidas FROM ind_check              WHERE periodo = getvariable('f_periodo') ORDER BY id",
        )?;
        let numero = |v: &serde_json::Value| v.as_f64().map_or(String::new(), |n| format!("{n}"));
        let checks = filas
            .iter()
            .map(|fila| {
                let id = fila["id"].as_str().unwrap_or_default().to_string();
                let valor = fila["valor"].as_f64();
                let mensaje = match (id.as_str(), valor) {
                    ("IND-C1", _) if !ab.hay_base => "No hay base de personal procesada.".to_string(),
                    ("IND-C1", _) => format!(
                        "{} personas no aparecen al filtrar por gerencia o sede",
                        numero(&fila["valor"])
                    ),
                    (_, None) => "Sin mes anterior: no hay con qué comparar.".to_string(),
                    _ => format!(
                        "La planilla cambió en {} personas; entradas menos salidas da {}. La diferencia ({}) se explica por salidas a fin de mes que todavía cobran.",
                        numero(&fila["cambio_personas"]),
                        numero(&fila["entradas_menos_salidas"]),
                        numero(&fila["valor"])
                    ),
                };
                let valor = if id == "IND-C1" && !ab.hay_base { None } else { valor };
                Check { id, valor, mensaje }
            })
            .collect();
        Ok(RespuestaChecks {
            periodo: ab.filtros.periodo,
            checks,
        })
    }

    /// Detalle nominal (TR-D18). Solo para la pestaña de RR. HH.
    pub fn detalle(&self, tipo: Detalle, f: &Filtros) -> Result<RespuestaDetalle, ErrorTablero> {
        let ab = self.abrir(f)?;
        let vista = match tipo {
            Detalle::Alertas => "detalle_alertas",
            Detalle::SinBase => "detalle_sin_base",
        };
        let filas = ab
            .almacen
            .consultar_json(&format!("SELECT * FROM {vista}"))?;
        Ok(RespuestaDetalle {
            tipo,
            periodo: match tipo {
                Detalle::Alertas => None,
                Detalle::SinBase => ab.filtros.periodo,
            },
            filas,
        })
    }
}
