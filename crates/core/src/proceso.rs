//! Proceso Silver/Gold (Plan 4): carga lo leído en DuckDB, ejecuta el SQL de `config/sql/` y los controles.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use chrono::NaiveDate;

use crate::almacen::Almacen;
use crate::error::ErrorProceso;
use crate::informe::{Problema, Severidad};
use crate::lectura::{Lectura, Valor};
use crate::parametros::Parametros;
use crate::preflight::FechaNombre;

/// Columnas de `licencias_lectura` cuando no se entrega el control de licencias.
const LICENCIAS_VACIA: &str = "CREATE TABLE licencias_lectura (periodo VARCHAR, numero_documento VARCHAR, \
     apellido_paterno VARCHAR, apellido_materno VARCHAR, nombres VARCHAR, motivo_licencia VARCHAR, \
     tipo_licencia VARCHAR, archivo_origen VARCHAR, fila_origen BIGINT);";

/// Columnas de `centros_costo` (config/sql/base_personal/20_centros_costo.sql) cuando no se cargó la base.
const CENTROS_COSTO_VACIO: &str = "CREATE TABLE centros_costo (centro_costo VARCHAR, \
     nombre_centro_costo VARCHAR, categoria_centro_costo VARCHAR, gerencia VARCHAR);";

fn id(nombre: &str) -> String {
    format!("\"{}\"", nombre.replace('"', "\"\""))
}

/// Carga las planillas en `planilla_lectura` y devuelve los periodos, ordenados.
///
/// TR-D23: dos archivos del mismo periodo son error. Vocabulario §4: asegura `conservar` y suma el resto de
/// columnas dinámicas en `subsidios.resto` (nulo si no hay ninguna).
pub(crate) fn cargar_planillas(
    a: &Almacen,
    lecturas: &[Lectura],
    p: &Parametros,
) -> Result<Vec<String>, ErrorProceso> {
    let mut por_periodo: BTreeMap<String, Vec<&str>> = BTreeMap::new();
    for l in lecturas {
        let Some(FechaNombre::Periodo(periodo)) = &l.fecha_nombre else {
            return Err(ErrorProceso::Datos(format!(
                "No se pudo saber el periodo de «{}».",
                l.archivo
            )));
        };
        por_periodo
            .entry(periodo.clone())
            .or_default()
            .push(&l.archivo);
    }
    for (periodo, archivos) in &por_periodo {
        if archivos.len() > 1 {
            let lista = archivos
                .iter()
                .map(|a| format!("«{a}»"))
                .collect::<Vec<_>>()
                .join(", ");
            return Err(ErrorProceso::Datos(format!(
                "El periodo {periodo} viene en más de un archivo: {lista}. Deje solo uno."
            )));
        }
    }

    let mut tablas = Vec::new();
    for (i, l) in lecturas.iter().enumerate() {
        let Some(FechaNombre::Periodo(periodo)) = &l.fecha_nombre else {
            unreachable!("revisado arriba");
        };
        let nombre = format!("planilla_{i}");
        a.cargar(
            &nombre,
            &l.hojas[0],
            &l.archivo,
            &[("periodo", Valor::Texto(periodo.clone()))],
        )?;
        tablas.push(nombre);
    }
    a.unir("planilla_lectura", &tablas)?;
    for t in &tablas {
        a.ejecutar(&format!("DROP TABLE {};", id(t)))?;
    }
    agrupar_subsidios(a, p)?;
    Ok(por_periodo.into_keys().collect())
}

/// Vocabulario §4 / NOM-SLV-04: columnas dinámicas de subsidios con un esquema fijo.
fn agrupar_subsidios(a: &Almacen, p: &Parametros) -> Result<(), ErrorProceso> {
    let s = &p.subsidios;
    let columnas = a.columnas("planilla_lectura")?;
    for c in &s.conservar {
        if !columnas.contains(c) {
            a.ejecutar(&format!(
                "ALTER TABLE planilla_lectura ADD COLUMN {} BIGINT;",
                id(c)
            ))?;
        }
    }
    // El lector ya puede traer `resto` (p. ej. `DIAS SUB. (*)` o `DIAS SUB. OTROS`): también se suma.
    let resto: Vec<&String> = columnas
        .iter()
        .filter(|c| c.starts_with(&s.prefijo) && !s.conservar.contains(c) && !s.fijas.contains(c))
        .collect();
    let temporal = format!("{}__suma", s.resto);
    a.ejecutar(&format!(
        "ALTER TABLE planilla_lectura ADD COLUMN {} BIGINT;",
        id(&temporal)
    ))?;
    if !resto.is_empty() {
        let presente = resto
            .iter()
            .map(|c| format!("{} IS NOT NULL", id(c)))
            .collect::<Vec<_>>();
        let suma = resto
            .iter()
            .map(|c| format!("COALESCE({}, 0)", id(c)))
            .collect::<Vec<_>>();
        a.ejecutar(&format!(
            "UPDATE planilla_lectura SET {} = CASE WHEN {} THEN {} END;",
            id(&temporal),
            presente.join(" OR "),
            suma.join(" + ")
        ))?;
        for c in resto {
            a.ejecutar(&format!(
                "ALTER TABLE planilla_lectura DROP COLUMN {};",
                id(c)
            ))?;
        }
    }
    a.ejecutar(&format!(
        "ALTER TABLE planilla_lectura RENAME COLUMN {} TO {};",
        id(&temporal),
        id(&s.resto)
    ))?;
    Ok(())
}

/// Carga el control de licencias en `licencias_lectura`, con `tipo_licencia` según la hoja (LIC-SLV-05).
/// Sin archivo, la tabla queda vacía para que la planilla Gold tenga los motivos en nulo.
pub(crate) fn cargar_licencias(
    a: &Almacen,
    lectura: Option<&Lectura>,
    p: &Parametros,
) -> Result<(), ErrorProceso> {
    let Some(l) = lectura else {
        return a.ejecutar(LICENCIAS_VACIA);
    };
    let mut tablas = Vec::new();
    for (i, hoja) in l.hojas.iter().enumerate() {
        let tipo = p.licencias.tipos.get(&hoja.hoja).ok_or_else(|| {
            ErrorProceso::Datos(format!(
                "La hoja «{}» no tiene tipo de licencia en parametros.yaml (licencias.tipos).",
                hoja.hoja
            ))
        })?;
        let nombre = format!("licencias_{i}");
        a.cargar(
            &nombre,
            hoja,
            &l.archivo,
            &[("tipo_licencia", Valor::Texto(tipo.clone()))],
        )?;
        tablas.push(nombre);
    }
    a.unir("licencias_lectura", &tablas)?;
    for t in &tablas {
        a.ejecutar(&format!("DROP TABLE {};", id(t)))?;
    }
    Ok(())
}

/// Carga la base de personal en `base_personal_lectura` con la constante `fecha_corte` (TR-D3, BD-SLV-08)
/// y devuelve esa fecha.
pub(crate) fn cargar_base_personal(
    a: &Almacen,
    lectura: &Lectura,
) -> Result<NaiveDate, ErrorProceso> {
    let Some(FechaNombre::FechaCorte(fecha)) = lectura.fecha_nombre else {
        return Err(ErrorProceso::Datos(format!(
            "No se pudo saber la fecha de corte de «{}».",
            lectura.archivo
        )));
    };
    a.cargar(
        "base_personal_lectura",
        &lectura.hojas[0],
        &lectura.archivo,
        &[("fecha_corte", Valor::Fecha(fecha))],
    )?;
    Ok(fecha)
}

/// Plan 8 (molde): carga las hojas de `lecturas` en `<fuente>_lectura`, unidas por nombre, con las constantes
/// `hoja` y, según el nombre del archivo, `periodo` o `fecha_corte`. Dos archivos del mismo periodo son error.
pub(crate) fn cargar_otra(
    a: &Almacen,
    fuente: &str,
    lecturas: &[Lectura],
) -> Result<(), ErrorProceso> {
    let mut por_periodo: BTreeMap<&str, &str> = BTreeMap::new();
    for l in lecturas {
        if let Some(FechaNombre::Periodo(p)) = &l.fecha_nombre
            && let Some(otro) = por_periodo.insert(p, &l.archivo)
        {
            return Err(ErrorProceso::Datos(format!(
                "El periodo {p} viene en más de un archivo: «{otro}», «{}». Deje solo uno.",
                l.archivo
            )));
        }
    }
    let mut tablas = Vec::new();
    for (i, l) in lecturas.iter().enumerate() {
        for (j, hoja) in l.hojas.iter().enumerate() {
            let mut constantes = vec![("hoja", Valor::Texto(hoja.hoja.clone()))];
            match &l.fecha_nombre {
                Some(FechaNombre::Periodo(p)) => {
                    constantes.push(("periodo", Valor::Texto(p.clone())))
                }
                Some(FechaNombre::FechaCorte(f)) => {
                    constantes.push(("fecha_corte", Valor::Fecha(*f)))
                }
                None => {}
            }
            let nombre = format!("{fuente}__lectura_{i}_{j}");
            a.cargar(&nombre, hoja, &l.archivo, &constantes)?;
            tablas.push(nombre);
        }
    }
    a.unir(&format!("{fuente}_lectura"), &tablas)?;
    for t in &tablas {
        a.ejecutar(&format!("DROP TABLE {};", id(t)))?;
    }
    Ok(())
}

/// Ejecuta los `.sql` de una carpeta en orden alfabético.
pub(crate) fn ejecutar_carpeta(a: &Almacen, carpeta: &Path) -> Result<(), ErrorProceso> {
    for ruta in archivos_sql(carpeta, "")? {
        a.ejecutar_archivo(&ruta)?;
    }
    Ok(())
}

fn archivos_sql(carpeta: &Path, prefijo: &str) -> Result<Vec<PathBuf>, ErrorProceso> {
    let leer = |e: std::io::Error| ErrorProceso::Duckdb {
        donde: carpeta.display().to_string(),
        detalle: e.to_string(),
    };
    let mut rutas = Vec::new();
    for entrada in std::fs::read_dir(carpeta).map_err(leer)? {
        let ruta = entrada.map_err(leer)?.path();
        let nombre = ruta.file_name().unwrap_or_default().to_string_lossy();
        if ruta.extension().is_some_and(|x| x == "sql") && nombre.starts_with(prefijo) {
            rutas.push(ruta);
        }
    }
    rutas.sort();
    Ok(rutas)
}

/// Prefijos `<fuente>_` de las carpetas hermanas de `controles/` (una por fuente con SQL propio).
fn prefijos_de_fuentes(carpeta: &Path) -> Vec<String> {
    let Some(sql) = carpeta.parent() else {
        return Vec::new();
    };
    std::fs::read_dir(sql)
        .map(|d| {
            d.filter_map(Result::ok)
                .filter(|e| e.path().is_dir())
                .map(|e| format!("{}_", e.file_name().to_string_lossy()))
                .collect()
        })
        .unwrap_or_default()
}

/// Ejecuta los controles de `carpeta` cuyos nombres empiezan por alguno de `prefijos` (TR-D5).
/// Cada control devuelve filas `(codigo, severidad, archivo, hoja, celda, mensaje)`.
///
/// Plan 8: un control pertenece a la fuente de prefijo más largo, así `planilla_` no ejecuta los de
/// `planilla_regimen_especial_`.
pub(crate) fn controles(
    a: &Almacen,
    carpeta: &Path,
    prefijos: &[&str],
) -> Result<Vec<Problema>, ErrorProceso> {
    let fuentes = prefijos_de_fuentes(carpeta);
    let mut problemas = Vec::new();
    for prefijo in prefijos {
        let de_otra = |ruta: &PathBuf| {
            let nombre = ruta.file_name().unwrap_or_default().to_string_lossy();
            fuentes
                .iter()
                .any(|f| f.len() > prefijo.len() && f.starts_with(prefijo) && nombre.starts_with(f))
        };
        for ruta in archivos_sql(carpeta, prefijo)?
            .into_iter()
            .filter(|r| !de_otra(r))
        {
            let sql = std::fs::read_to_string(&ruta).map_err(|e| ErrorProceso::Duckdb {
                donde: ruta.display().to_string(),
                detalle: e.to_string(),
            })?;
            let filas = a.consultar_texto(&sql).map_err(|e| match e {
                ErrorProceso::Duckdb { detalle, .. } => ErrorProceso::Duckdb {
                    donde: ruta.display().to_string(),
                    detalle,
                },
                otro => otro,
            })?;
            for f in filas {
                let [codigo, severidad, archivo, hoja, celda, mensaje] =
                    <[Option<String>; 6]>::try_from(f).map_err(|_| ErrorProceso::Duckdb {
                        donde: ruta.display().to_string(),
                        detalle: "un control debe devolver 6 columnas".into(),
                    })?;
                problemas.push(Problema {
                    codigo: codigo.unwrap_or_default(),
                    severidad: if severidad.as_deref() == Some("error") {
                        Severidad::Error
                    } else {
                        Severidad::Aviso
                    },
                    archivo: archivo.unwrap_or_default(),
                    hoja,
                    celda,
                    celda_encontrada: None,
                    mensaje: mensaje.unwrap_or_default(),
                    detalle: None,
                });
            }
        }
    }
    Ok(problemas)
}

/// Lee archivos con su contrato; si alguno tiene errores, devuelve todos los informes.
pub(crate) fn leer_todos(
    contrato: &crate::contrato::Contrato,
    rutas: &[PathBuf],
) -> Result<Vec<Lectura>, Vec<crate::informe::Informe>> {
    let mut lecturas = Vec::new();
    let mut con_errores = Vec::new();
    for ruta in rutas {
        match crate::lectura::leer_archivo(contrato, ruta) {
            Ok(l) => lecturas.push(l),
            Err(informe) => con_errores.push(informe),
        }
    }
    if con_errores.is_empty() {
        Ok(lecturas)
    } else {
        Err(con_errores)
    }
}

// --- Proceso completo (Tarea 6) ---------------------------------------------------------------

/// Archivos de una ejecución. Cualquier combinación sirve, pero al menos uno.
#[derive(Debug, Clone, Default)]
pub struct Entradas {
    pub planillas: Vec<PathBuf>,
    pub licencias: Option<PathBuf>,
    pub base_personal: Option<PathBuf>,
    /// Plan 8: fuentes del molde (contrato + `config/sql/<fuente>/`), por nombre de fuente.
    pub otras: BTreeMap<String, Vec<PathBuf>>,
}

impl Entradas {
    /// Fuentes del molde que traen al menos un archivo.
    pub fn otras_con_archivos(&self) -> impl Iterator<Item = (&String, &Vec<PathBuf>)> {
        self.otras.iter().filter(|(_, v)| !v.is_empty())
    }
}

/// Estado de una etapa (TR-D6).
#[derive(
    schemars::JsonSchema, serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq,
)]
#[serde(rename_all = "snake_case")]
pub enum EstadoEtapa {
    Ok,
    Aviso,
    Fallo,
    Omitida,
}

#[derive(schemars::JsonSchema, serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Etapa {
    pub nombre: String,
    pub estado: EstadoEtapa,
    pub filas: Option<u64>,
}

#[derive(schemars::JsonSchema, serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct Fuente {
    pub fuente: String,
    pub archivo: String,
    pub fecha_nombre: Option<FechaNombre>,
}

/// Resumen de una ejecución; se guarda como `ejecucion.json` junto a los Parquet (TR-D16).
#[derive(schemars::JsonSchema, serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Ejecucion {
    pub marca: String,
    pub version_motor: String,
    pub fecha_corte: Option<NaiveDate>,
    pub periodos: Vec<String>,
    pub fuentes: Vec<Fuente>,
    pub etapas: Vec<Etapa>,
    /// (ruta relativa del Parquet, filas)
    pub salidas: Vec<(String, u64)>,
    /// Avisos de lectura y de los controles.
    pub problemas: Vec<Problema>,
}

const MARCADOR: &str = ".etl-local";

fn salida_err(ruta: &Path) -> impl FnOnce(std::io::Error) -> ErrorProceso + '_ {
    move |fuente| ErrorProceso::Salida {
        ruta: ruta.to_path_buf(),
        fuente,
    }
}

/// Crea la carpeta de salida con su marcador, o comprueba que ya es nuestra (lección de los Planes 1–2).
/// Pública para que la app la use al crear la carpeta desde «Abrir carpeta».
pub fn preparar_salida(salida: &Path) -> Result<(), ErrorProceso> {
    if salida.join(MARCADOR).is_file() {
        return Ok(());
    }
    if salida.exists() {
        let vacia = std::fs::read_dir(salida)
            .map_err(salida_err(salida))?
            .next()
            .is_none();
        if !vacia {
            return Err(ErrorProceso::CarpetaAjena(salida.to_path_buf()));
        }
    }
    std::fs::create_dir_all(salida).map_err(salida_err(salida))?;
    std::fs::write(salida.join(MARCADOR), "Carpeta de salida de etl-local.\n")
        .map_err(salida_err(salida))
}

fn borrar_si_existe(ruta: &Path) -> Result<(), ErrorProceso> {
    if ruta.exists() {
        std::fs::remove_dir_all(ruta).map_err(salida_err(ruta))?;
    }
    Ok(())
}

fn copiar_carpeta(origen: &Path, destino: &Path) -> Result<(), ErrorProceso> {
    std::fs::create_dir_all(destino).map_err(salida_err(destino))?;
    for entrada in std::fs::read_dir(origen).map_err(salida_err(origen))? {
        let ruta = entrada.map_err(salida_err(origen))?.path();
        let hacia = destino.join(ruta.file_name().unwrap_or_default());
        if ruta.is_dir() {
            copiar_carpeta(&ruta, &hacia)?;
        } else {
            std::fs::copy(&ruta, &hacia).map_err(salida_err(&hacia))?;
        }
    }
    Ok(())
}

/// TR-D7: `.en-curso-<marca>` → `historico/<marca>`; luego reemplaza `actual/` sin dejarlo a medias.
fn promover(salida: &Path, en_curso: &Path, marca: &str) -> Result<(), ErrorProceso> {
    let historico = salida.join("historico");
    std::fs::create_dir_all(&historico).map_err(salida_err(&historico))?;
    let destino = historico.join(marca);
    std::fs::rename(en_curso, &destino).map_err(salida_err(&destino))?;

    let actual = salida.join("actual");
    let nuevo = salida.join(".actual-nuevo");
    let viejo = salida.join(".actual-viejo");
    borrar_si_existe(&nuevo)?;
    borrar_si_existe(&viejo)?;
    copiar_carpeta(&destino, &nuevo)?;
    if actual.exists()
        && let Err(e) = std::fs::rename(&actual, &viejo)
    {
        let _ = std::fs::remove_dir_all(&nuevo);
        return Err(ErrorProceso::Publicar {
            ruta: actual.clone(),
            marca: marca.to_string(),
            fuente: e,
        });
    }
    if let Err(e) = std::fs::rename(&nuevo, &actual) {
        if viejo.exists() {
            let _ = std::fs::rename(&viejo, &actual);
        }
        let _ = std::fs::remove_dir_all(&nuevo);
        return Err(ErrorProceso::Publicar {
            ruta: actual.clone(),
            marca: marca.to_string(),
            fuente: e,
        });
    }
    // Si no se puede borrar la copia vieja, la próxima ejecución lo reintenta (`borrar_si_existe`).
    let _ = std::fs::remove_dir_all(&viejo);
    Ok(())
}

fn pares(v: &[(&str, &str)]) -> Vec<(String, String)> {
    v.iter()
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .collect()
}

fn estado(avisos: usize) -> EstadoEtapa {
    if avisos > 0 {
        EstadoEtapa::Aviso
    } else {
        EstadoEtapa::Ok
    }
}

fn omitida(nombre: &str) -> Etapa {
    Etapa {
        nombre: nombre.into(),
        estado: EstadoEtapa::Omitida,
        filas: None,
    }
}

/// Ejecuta el proceso completo: lectura → Silver → Gold → controles → escritura atómica.
///
/// `config` es la carpeta con `fuentes/`, `sql/` y `parametros.yaml`. `marca` identifica la ejecución
/// (el CLI usa la fecha y hora; el núcleo no lee el reloj, AGENTS.md §4.5).
pub fn procesar(
    config: &Path,
    entradas: &Entradas,
    salida: &Path,
    marca: &str,
) -> Result<Ejecucion, ErrorProceso> {
    procesar_con(
        config,
        entradas,
        salida,
        marca,
        &mut |_| {},
        &AtomicBool::new(false),
    )
}

/// Paso del proceso que la app muestra en lenguaje de oficina (Plan 6, pantalla «Procesando»).
#[derive(
    schemars::JsonSchema, serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq,
)]
#[serde(rename_all = "snake_case")]
pub enum Paso {
    Leer,
    Licencias,
    Planilla,
    BasePersonal,
    /// Plan 8: las fuentes del molde (un solo paso para todas).
    OtrosReportes,
    Controles,
    Guardar,
}

/// Aviso de avance: `paso` empieza; `indice` es su posición en `pasos(entradas)`.
#[derive(schemars::JsonSchema, serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct Avance {
    pub paso: Paso,
    pub indice: usize,
    pub total: usize,
    /// Todos los pasos de esta ejecución, en orden (la app arma la lista con el primer aviso).
    pub pasos: Vec<Paso>,
}

/// Pasos que recorrerá `procesar` con estas entradas, en orden.
pub fn pasos(entradas: &Entradas) -> Vec<Paso> {
    let hay_planilla = !entradas.planillas.is_empty();
    let mut v = vec![Paso::Leer];
    if hay_planilla || entradas.licencias.is_some() {
        v.push(Paso::Licencias);
    }
    if hay_planilla {
        v.push(Paso::Planilla);
    }
    if entradas.base_personal.is_some() {
        v.push(Paso::BasePersonal);
    }
    if entradas.otras_con_archivos().next().is_some() {
        v.push(Paso::OtrosReportes);
    }
    v.extend([Paso::Controles, Paso::Guardar]);
    v
}

/// Como `procesar`, pero avisa el comienzo de cada paso y se detiene (sin publicar nada) si `cancelar`
/// se activa antes de reemplazar `actual/`.
pub fn procesar_con(
    config: &Path,
    entradas: &Entradas,
    salida: &Path,
    marca: &str,
    avance: &mut dyn FnMut(Avance),
    cancelar: &AtomicBool,
) -> Result<Ejecucion, ErrorProceso> {
    let lista = pasos(entradas);
    let mut empezar = |p: Paso| -> Result<(), ErrorProceso> {
        if cancelar.load(Ordering::Relaxed) {
            return Err(ErrorProceso::Cancelado);
        }
        let indice = lista
            .iter()
            .position(|x| *x == p)
            .expect("el paso está en pasos(entradas)");
        avance(Avance {
            paso: p,
            indice,
            total: lista.len(),
            pasos: lista.clone(),
        });
        Ok(())
    };
    if entradas.planillas.is_empty()
        && entradas.licencias.is_none()
        && entradas.base_personal.is_none()
        && entradas.otras_con_archivos().next().is_none()
    {
        return Err(ErrorProceso::Datos("No hay archivos para procesar.".into()));
    }
    if marca.is_empty()
        || !marca
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(ErrorProceso::Datos(format!(
            "La marca «{marca}» solo puede tener letras, números, guiones y guiones bajos."
        )));
    }
    preparar_salida(salida)?;
    if salida.join("historico").join(marca).exists() {
        return Err(ErrorProceso::Datos(format!(
            "Ya existe una ejecución con la marca «{marca}» en {}.",
            salida.join("historico").display()
        )));
    }

    let mut contratos = crate::contrato::cargar_contratos(&config.join("fuentes"))?;
    let p = crate::parametros::cargar_parametros(&config.join("parametros.yaml"))?;
    let mut contrato = |fuente: &str| {
        contratos.remove(fuente).ok_or_else(|| {
            ErrorProceso::Datos(format!(
                "Falta el contrato «{fuente}» en {}.",
                config.join("fuentes").display()
            ))
        })
    };
    let (c_planilla, c_licencias, c_base) = (
        contrato("planilla")?,
        contrato("licencias")?,
        contrato("base_personal")?,
    );
    let c_otras = entradas
        .otras_con_archivos()
        .map(|(f, rutas)| Ok((f.clone(), contrato(f)?, rutas.clone())))
        .collect::<Result<Vec<_>, ErrorProceso>>()?;

    // Lectura de todo; si algún archivo tiene errores, no se procesa nada.
    let mut con_errores = Vec::new();
    let mut leer = |c: &crate::contrato::Contrato, rutas: &[PathBuf]| match leer_todos(c, rutas) {
        Ok(l) => l,
        Err(mut informes) => {
            con_errores.append(&mut informes);
            Vec::new()
        }
    };
    empezar(Paso::Leer)?;
    let planillas = leer(&c_planilla, &entradas.planillas);
    let licencias = leer(&c_licencias, entradas.licencias.as_slice()).pop();
    let base = leer(&c_base, entradas.base_personal.as_slice()).pop();
    let otras: Vec<(String, Vec<Lectura>)> = c_otras
        .iter()
        .map(|(f, c, rutas)| (f.clone(), leer(c, rutas)))
        .collect();
    if !con_errores.is_empty() {
        return Err(ErrorProceso::Entradas(con_errores));
    }

    let mut fuentes = Vec::new();
    let mut problemas = Vec::new();
    let leidas_otras = otras.iter().flat_map(|(_, ls)| ls);
    for l in planillas
        .iter()
        .chain(&licencias)
        .chain(&base)
        .chain(leidas_otras)
    {
        fuentes.push(Fuente {
            fuente: l.fuente.clone(),
            archivo: l.archivo.clone(),
            fecha_nombre: l.fecha_nombre.clone(),
        });
        problemas.extend(l.informe.problemas.iter().cloned());
    }
    let avisos_de = |ls: &[&Lectura]| ls.iter().map(|l| l.informe.avisos().count()).sum::<usize>();

    let a = Almacen::nuevo()?;
    let fecha_corte = base
        .as_ref()
        .map(|b| cargar_base_personal(&a, b))
        .transpose()?;
    a.fijar_variables(&p, fecha_corte)?;
    let sql = config.join("sql");
    let mut etapas = Vec::new();
    let mut exportar: Vec<(String, String)> = Vec::new();
    let mut prefijos: Vec<String> = Vec::new();
    let mut periodos = Vec::new();

    let hay_planilla = !planillas.is_empty();
    if hay_planilla || licencias.is_some() {
        empezar(Paso::Licencias)?;
        cargar_licencias(&a, licencias.as_ref(), &p)?;
        ejecutar_carpeta(&a, &sql.join("licencias"))?;
    }
    if hay_planilla {
        empezar(Paso::Planilla)?;
        periodos = cargar_planillas(&a, &planillas, &p)?;
        ejecutar_carpeta(&a, &sql.join("planilla"))?;
        let refs: Vec<&Lectura> = planillas.iter().collect();
        etapas.push(Etapa {
            nombre: "planilla".into(),
            estado: estado(avisos_de(&refs)),
            filas: Some(a.contar("planilla_gold")?),
        });
        exportar.extend(pares(&[
            ("planilla_silver", "silver/planilla.parquet"),
            ("planilla_gold", "gold/planilla.parquet"),
        ]));
        prefijos.push("planilla_".into());
    } else {
        etapas.push(omitida("planilla"));
    }
    match &licencias {
        Some(l) => {
            etapas.push(Etapa {
                nombre: "licencias".into(),
                estado: estado(avisos_de(&[l])),
                filas: Some(a.contar("licencias_silver")?),
            });
            exportar.extend(pares(&[("licencias_silver", "silver/licencias.parquet")]));
            if hay_planilla {
                prefijos.push("licencias_".into());
            }
        }
        None => etapas.push(omitida("licencias")),
    }
    match &base {
        Some(b) => {
            empezar(Paso::BasePersonal)?;
            ejecutar_carpeta(&a, &sql.join("base_personal"))?;
            let filas = a.contar("base_personal_empleados_gold")?
                + a.contar("base_personal_practicantes_gold")?;
            etapas.push(Etapa {
                nombre: "base_personal".into(),
                estado: estado(avisos_de(&[b])),
                filas: Some(filas),
            });
            exportar.extend(pares(&[
                ("base_personal_silver", "silver/base_personal.parquet"),
                (
                    "base_personal_empleados_gold",
                    "gold/base_personal_empleados.parquet",
                ),
                (
                    "base_personal_practicantes_gold",
                    "gold/base_personal_practicantes.parquet",
                ),
                ("centros_costo", "gold/centros_costo.parquet"),
            ]));
            prefijos.push("base_personal_".into());
        }
        None => etapas.push(omitida("base_personal")),
    }
    // Plan 8 (molde): cada fuente de `otras` deja `<fuente>_gold` con su SQL.
    if !otras.is_empty() {
        empezar(Paso::OtrosReportes)?;
        // Los reportes del molde pueden cruzar con el catálogo de centros de costo de la base de personal;
        // sin base en esta ejecución, el catálogo existe vacío.
        if !a.existe("centros_costo")? {
            a.ejecutar(CENTROS_COSTO_VACIO)?;
        }
    }
    for (fuente, lecturas) in &otras {
        cargar_otra(&a, fuente, lecturas)?;
        ejecutar_carpeta(&a, &sql.join(fuente))?;
        let gold = format!("{fuente}_gold");
        let refs: Vec<&Lectura> = lecturas.iter().collect();
        etapas.push(Etapa {
            nombre: fuente.clone(),
            estado: estado(avisos_de(&refs)),
            filas: Some(a.contar(&gold)?),
        });
        exportar.push((gold, format!("gold/{fuente}.parquet")));
        prefijos.push(format!("{fuente}_"));
    }
    empezar(Paso::Controles)?;
    let prefijos: Vec<&str> = prefijos.iter().map(String::as_str).collect();
    let de_controles = controles(&a, &sql.join("controles"), &prefijos)?;
    etapas.push(Etapa {
        nombre: "controles".into(),
        estado: if de_controles.iter().any(|p| p.severidad == Severidad::Error) {
            EstadoEtapa::Fallo
        } else {
            estado(de_controles.len())
        },
        filas: Some(de_controles.len() as u64),
    });
    let fallo = de_controles.iter().any(|p| p.severidad == Severidad::Error);
    problemas.extend(de_controles);
    if fallo {
        return Err(ErrorProceso::Datos(
            "Un control encontró errores; no se publicó nada. Revise los problemas.".into(),
        ));
    }

    // Escritura atómica (TR-D7).
    empezar(Paso::Guardar)?;
    let en_curso = salida.join(format!(".en-curso-{marca}"));
    borrar_si_existe(&en_curso)?;
    let mut salidas = Vec::new();
    let resultado = (|| -> Result<Ejecucion, ErrorProceso> {
        for (tabla, ruta) in &exportar {
            let filas = a.exportar_parquet(tabla, &en_curso.join(ruta))?;
            salidas.push((ruta.to_string(), filas));
        }
        // Plan 6: Excel de revisión de cada tabla Gold (00-objetivo.md: «Excel opcional para revisión»).
        for (tabla, ruta) in &exportar {
            if let Some(nombre) = ruta
                .strip_prefix("gold/")
                .and_then(|r| r.strip_suffix(".parquet"))
            {
                let destino = format!("excel/{nombre}.xlsx");
                let filas = a.exportar_excel(tabla, &en_curso.join(&destino))?;
                salidas.push((destino, filas));
            }
        }
        let ejecucion = Ejecucion {
            marca: marca.to_string(),
            version_motor: env!("CARGO_PKG_VERSION").to_string(),
            fecha_corte,
            periodos: periodos.clone(),
            fuentes: fuentes.clone(),
            etapas: etapas.clone(),
            salidas: salidas.clone(),
            problemas: problemas.clone(),
        };
        let json = serde_json::to_string_pretty(&ejecucion).expect("Ejecucion es serializable");
        std::fs::write(en_curso.join("ejecucion.json"), json).map_err(salida_err(&en_curso))?;
        if cancelar.load(Ordering::Relaxed) {
            return Err(ErrorProceso::Cancelado);
        }
        promover(salida, &en_curso, marca)?;
        Ok(ejecucion)
    })();
    if resultado.is_err() {
        let _ = std::fs::remove_dir_all(&en_curso);
    }
    resultado
}

/// Funciones para los tests de integración: ejecutan solo una parte del proceso.
#[doc(hidden)]
pub mod prueba {
    use super::*;
    use crate::contrato::cargar_contratos;
    use crate::parametros::cargar_parametros;

    /// Lectura + carga + SQL de planilla y licencias + controles de esa parte (Tarea 4).
    pub fn planilla_gold(
        config: &Path,
        planillas: &[PathBuf],
        licencias: Option<&Path>,
    ) -> Result<(Almacen, Vec<Problema>), ErrorProceso> {
        let mut contratos = cargar_contratos(&config.join("fuentes"))?;
        let p = cargar_parametros(&config.join("parametros.yaml"))?;
        let planilla = contratos.remove("planilla").expect("contrato planilla");
        let contrato_lic = contratos.remove("licencias").expect("contrato licencias");
        let lecturas = leer_todos(&planilla, planillas).map_err(ErrorProceso::Entradas)?;
        let lic = match licencias {
            Some(r) => Some(
                leer_todos(&contrato_lic, &[r.to_path_buf()])
                    .map_err(ErrorProceso::Entradas)?
                    .remove(0),
            ),
            None => None,
        };
        let a = Almacen::nuevo()?;
        a.fijar_variables(&p, None)?;
        cargar_planillas(&a, &lecturas, &p)?;
        cargar_licencias(&a, lic.as_ref(), &p)?;
        let sql = config.join("sql");
        ejecutar_carpeta(&a, &sql.join("licencias"))?;
        ejecutar_carpeta(&a, &sql.join("planilla"))?;
        let problemas = controles(&a, &sql.join("controles"), &["planilla_", "licencias_"])?;
        Ok((a, problemas))
    }

    /// Lectura + carga + SQL de la base de personal + sus controles (Tarea 5).
    pub fn base_personal(
        config: &Path,
        archivo: &Path,
    ) -> Result<(Almacen, Vec<Problema>), ErrorProceso> {
        let mut contratos = cargar_contratos(&config.join("fuentes"))?;
        let p = cargar_parametros(&config.join("parametros.yaml"))?;
        let contrato = contratos
            .remove("base_personal")
            .expect("contrato base_personal");
        let lectura = leer_todos(&contrato, &[archivo.to_path_buf()])
            .map_err(ErrorProceso::Entradas)?
            .remove(0);
        let a = Almacen::nuevo()?;
        let fecha = cargar_base_personal(&a, &lectura)?;
        a.fijar_variables(&p, Some(fecha))?;
        let sql = config.join("sql");
        ejecutar_carpeta(&a, &sql.join("base_personal"))?;
        let problemas = controles(&a, &sql.join("controles"), &["base_personal_"])?;
        Ok((a, problemas))
    }
}
