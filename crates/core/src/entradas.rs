//! Entradas guardadas por fuente (`<salida>/entradas/<fuente>/`, Plan 6): permiten actualizar un reporte a la
//! vez reprocesando todo, aunque el usuario mueva o renombre sus Excel.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use crate::contrato::{Contrato, FormatoFechaNombre, cargar_contratos};
use crate::error::ErrorProceso;
use crate::preflight::{FechaNombre, fecha_en_nombre};
use crate::proceso::{Avance, Ejecucion, Entradas, procesar_con};
use crate::reportes::archivos_de;

const PLANILLA: &str = "planilla";
const LICENCIAS: &str = "licencias";
const BASE_PERSONAL: &str = "base_personal";

fn carpeta(salida: &Path, fuente: &str) -> PathBuf {
    salida.join("entradas").join(fuente)
}

/// Lo que quedó guardado de las ejecuciones anteriores. Sin `entradas/`, todo vacío.
pub fn guardadas(_config: &Path, salida: &Path) -> Result<Entradas, ErrorProceso> {
    let rutas = |fuente: &str| {
        let c = carpeta(salida, fuente);
        archivos_de(&c)
            .into_iter()
            .map(|n| c.join(n))
            .collect::<Vec<_>>()
    };
    // Plan 8: cada otra carpeta de `entradas/` es una fuente del molde.
    let mut otras = BTreeMap::new();
    if let Ok(dir) = std::fs::read_dir(salida.join("entradas")) {
        for e in dir.filter_map(Result::ok) {
            let fuente = e.file_name().to_string_lossy().into_owned();
            if e.path().is_dir() && ![PLANILLA, LICENCIAS, BASE_PERSONAL].contains(&fuente.as_str())
            {
                let archivos = rutas(&fuente);
                if !archivos.is_empty() {
                    otras.insert(fuente, archivos);
                }
            }
        }
    }
    Ok(Entradas {
        planillas: rutas(PLANILLA),
        licencias: rutas(LICENCIAS).pop(),
        base_personal: rutas(BASE_PERSONAL).pop(),
        otras,
    })
}

fn nombre(ruta: &Path) -> String {
    ruta.file_name().map_or_else(
        || ruta.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    )
}

/// Une los archivos guardados de una fuente con periodo `AAAA-MM` con los nuevos: uno nuevo reemplaza al
/// guardado del mismo periodo; dos nuevos del mismo periodo son error. Los guardados sin periodo reconocible se
/// conservan.
fn combinar_por_periodo(
    contrato: &Contrato,
    guardadas: &[PathBuf],
    nuevas: &[PathBuf],
) -> Result<Vec<PathBuf>, ErrorProceso> {
    let periodo = |ruta: &Path| match fecha_en_nombre(contrato, &nombre(ruta)) {
        Some(FechaNombre::Periodo(p)) => Some(p),
        _ => None,
    };
    let mut por_periodo: BTreeMap<String, PathBuf> = BTreeMap::new();
    for g in guardadas {
        let clave = periodo(g).unwrap_or_else(|| format!("~{}", nombre(g)));
        por_periodo.insert(clave, g.clone());
    }
    let mut nuevas_por_periodo: BTreeMap<String, PathBuf> = BTreeMap::new();
    for n in nuevas {
        let Some(p) = periodo(n) else {
            return Err(ErrorProceso::Datos(format!(
                "No reconocemos el mes en el nombre de «{}». Revise el nombre del archivo.",
                nombre(n)
            )));
        };
        if let Some(otra) = nuevas_por_periodo.insert(p.clone(), n.clone()) {
            return Err(ErrorProceso::Datos(format!(
                "Hay dos archivos del mismo mes ({p}): «{}» y «{}». Quite uno.",
                nombre(&otra),
                nombre(n)
            )));
        }
    }
    por_periodo.extend(nuevas_por_periodo);
    Ok(por_periodo.into_values().collect())
}

fn contrato_de<'a>(
    contratos: &'a BTreeMap<String, Contrato>,
    config: &Path,
    fuente: &str,
) -> Result<&'a Contrato, ErrorProceso> {
    contratos.get(fuente).ok_or_else(|| {
        ErrorProceso::Datos(format!(
            "Falta el contrato «{fuente}» en {}.",
            config.join("fuentes").display()
        ))
    })
}

/// Une lo guardado con lo nuevo: una planilla nueva reemplaza a la guardada del mismo periodo; licencias y
/// base nuevas reemplazan a las guardadas. Plan 8: las fuentes del molde con periodo en el nombre se combinan
/// por periodo, como la planilla; las demás se reemplazan si llegan archivos nuevos.
pub fn combinar(
    config: &Path,
    guardadas: &Entradas,
    nuevas: &Entradas,
) -> Result<Entradas, ErrorProceso> {
    let contratos = cargar_contratos(&config.join("fuentes"))?;
    let planilla = contrato_de(&contratos, config, PLANILLA)?;
    let planillas = combinar_por_periodo(planilla, &guardadas.planillas, &nuevas.planillas)?;

    let mut otras = guardadas.otras.clone();
    for (fuente, archivos) in nuevas.otras_con_archivos() {
        let c = contrato_de(&contratos, config, fuente)?;
        let por_periodo = c
            .archivo
            .fecha_en_nombre
            .as_ref()
            .is_some_and(|f| f.formato == FormatoFechaNombre::AnioMes);
        let usar = if por_periodo {
            let previas = guardadas.otras.get(fuente).cloned().unwrap_or_default();
            combinar_por_periodo(c, &previas, archivos)?
        } else {
            archivos.clone()
        };
        otras.insert(fuente.clone(), usar);
    }
    Ok(Entradas {
        planillas,
        licencias: nuevas
            .licencias
            .clone()
            .or_else(|| guardadas.licencias.clone()),
        base_personal: nuevas
            .base_personal
            .clone()
            .or_else(|| guardadas.base_personal.clone()),
        otras,
    })
}

/// Combina lo guardado con lo nuevo, procesa todo (sin las fuentes de `excluir`) y, solo si sale bien,
/// deja en `entradas/` exactamente lo usado. Las fuentes excluidas no se tocan.
pub fn actualizar(
    config: &Path,
    salida: &Path,
    nuevas: &Entradas,
    excluir: &[String],
    marca: &str,
    avance: &mut dyn FnMut(Avance),
    cancelar: &AtomicBool,
) -> Result<Ejecucion, ErrorProceso> {
    let g = guardadas(config, salida)?;
    let mut usar = combinar(config, &g, nuevas)?;
    let excluida = |f: &str| excluir.iter().any(|e| e == f);
    if excluida(PLANILLA) {
        usar.planillas.clear();
    }
    if excluida(LICENCIAS) {
        usar.licencias = None;
    }
    if excluida(BASE_PERSONAL) {
        usar.base_personal = None;
    }
    usar.otras.retain(|f, _| !excluida(f));
    let ejecucion = procesar_con(config, &usar, salida, marca, avance, cancelar)?;

    let mut grupos: Vec<(&str, Vec<PathBuf>)> = vec![
        (PLANILLA, usar.planillas.clone()),
        (LICENCIAS, usar.licencias.iter().cloned().collect()),
        (BASE_PERSONAL, usar.base_personal.iter().cloned().collect()),
    ];
    grupos.extend(usar.otras.iter().map(|(f, v)| (f.as_str(), v.clone())));
    for (fuente, archivos) in grupos {
        if excluida(fuente) || archivos.is_empty() {
            continue;
        }
        sincronizar(&carpeta(salida, fuente), &archivos)?;
    }
    Ok(ejecucion)
}

/// Deja en `destino` exactamente `archivos` (copiados con su nombre original).
fn sincronizar(destino: &Path, archivos: &[PathBuf]) -> Result<(), ErrorProceso> {
    let error = |ruta: &Path| {
        let ruta = ruta.to_path_buf();
        move |fuente| ErrorProceso::Salida { ruta, fuente }
    };
    std::fs::create_dir_all(destino).map_err(error(destino))?;
    let mut nombres = Vec::new();
    for origen in archivos {
        let n = nombre(origen);
        let copia = destino.join(&n);
        let misma = match (std::fs::canonicalize(origen), std::fs::canonicalize(&copia)) {
            (Ok(a), Ok(b)) => a == b,
            _ => false,
        };
        if !misma {
            std::fs::copy(origen, &copia).map_err(error(&copia))?;
        }
        nombres.push(n);
    }
    // Windows no distingue mayúsculas: al copiar encima, el guardado conserva las suyas.
    let quedan: Vec<String> = nombres.iter().map(|n| n.to_lowercase()).collect();
    for sobra in archivos_de(destino) {
        if !quedan.contains(&sobra.to_lowercase()) {
            let ruta = destino.join(&sobra);
            std::fs::remove_file(&ruta).map_err(error(&ruta))?;
        }
    }
    Ok(())
}
