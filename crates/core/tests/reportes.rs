//! Plan 6, Tarea 4: catálogo de reportes (`config/reportes.yaml`) y su vigencia.

mod comun;

use std::path::Path;

use chrono::NaiveDate;
use comun::{base_personal, planilla};
use etl_local_core::preflight::FechaNombre;
use etl_local_core::reportes::{Vigencia, cargar_reportes, estado_reportes};

const CONFIG: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../config");

fn dia(a: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(a, m, d).unwrap()
}

#[test]
fn catalogo_del_repo_es_valido() {
    let r = cargar_reportes(Path::new(CONFIG)).unwrap();
    let ids: Vec<_> = r.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "planilla",
            "base_personal",
            "planilla_regimen_especial",
            "altas_personal",
            "practicantes",
            "examenes_salida"
        ]
    );
    assert_eq!(r[0].complementos, ["licencias"]);
    assert!(r[0].varios_archivos && !r[1].varios_archivos);
    // Plan 8: los reportes del molde no tienen complementos y su Excel es el de su fuente.
    for x in &r[2..] {
        assert!(x.complementos.is_empty(), "{}", x.id);
        assert_eq!(x.excel, x.fuente);
        assert_eq!(x.id, x.fuente);
    }
    assert!(r[2].varios_archivos && r[3].varios_archivos);
    assert!(!r[4].varios_archivos && !r[5].varios_archivos);
    // Revisión final: títulos en singular, para «Tu … está listo» y «Procesar el …».
    assert_eq!(r[5].titulo, "Control de exámenes médicos de salida");
}

/// Copia `config/` a un temporal reemplazando `reportes.yaml`.
fn config_con(reportes: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let fuentes = dir.path().join("fuentes");
    std::fs::create_dir_all(&fuentes).unwrap();
    for e in std::fs::read_dir(Path::new(CONFIG).join("fuentes")).unwrap() {
        let e = e.unwrap();
        std::fs::copy(e.path(), fuentes.join(e.file_name())).unwrap();
    }
    std::fs::write(dir.path().join("reportes.yaml"), reportes).unwrap();
    dir
}

#[test]
fn fuente_inexistente_falla() {
    let dir = config_con(
        "reportes:\n  - id: nomina\n    titulo: N\n    descripcion: D\n    fuente: nomina\n    varios_archivos: false\n    complementos: []\n    excel: nomina\n    genero: femenino\n",
    );
    let e = cargar_reportes(dir.path()).unwrap_err().to_string();
    assert!(e.contains("nomina"), "{e}");
}

#[test]
fn id_repetido_falla() {
    let uno = "  - id: planilla\n    titulo: P\n    descripcion: D\n    fuente: planilla\n    varios_archivos: true\n    complementos: []\n    excel: planilla\n    genero: femenino\n";
    let dir = config_con(&format!("reportes:\n{uno}{uno}"));
    let e = cargar_reportes(dir.path()).unwrap_err().to_string();
    assert!(e.contains("planilla"), "{e}");
}

fn salida_con(archivos: &[(&str, &Path)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    for (fuente, ruta) in archivos {
        let carpeta = dir.path().join("entradas").join(fuente);
        std::fs::create_dir_all(&carpeta).unwrap();
        std::fs::copy(ruta, carpeta.join(ruta.file_name().unwrap())).unwrap();
    }
    dir
}

#[test]
fn vigencia() {
    let config = Path::new(CONFIG);
    let sin_nada = tempfile::tempdir().unwrap();
    let e = estado_reportes(config, sin_nada.path(), dia(2025, 12, 5)).unwrap();
    assert!(
        e.iter()
            .all(|r| r.vigencia == Vigencia::Nunca && r.ultimo.is_none())
    );

    let s = salida_con(&[
        ("planilla", &planilla("limpio", "2025-10")),
        ("planilla", &planilla("limpio", "2025-11")),
        ("base_personal", &base_personal("limpio")),
    ]);
    let e = estado_reportes(config, s.path(), dia(2025, 12, 5)).unwrap();
    assert_eq!(e[0].ultimo, Some(FechaNombre::Periodo("2025-11".into())));
    assert_eq!(e[0].vigencia, Vigencia::AlDia);
    assert_eq!(
        e[1].ultimo,
        Some(FechaNombre::FechaCorte(dia(2025, 11, 12)))
    );
    assert_eq!(e[1].vigencia, Vigencia::AlDia);

    let e = estado_reportes(config, s.path(), dia(2026, 1, 5)).unwrap();
    assert_eq!(
        e[0].vigencia,
        Vigencia::Atrasado,
        "2025-11 con hoy en enero"
    );
    assert_eq!(
        e[1].vigencia,
        Vigencia::Atrasado,
        "corte de hace más de 31 días"
    );
}

#[test]
fn reporte_sin_fecha_en_nombre_cargado_esta_al_dia() {
    let dir = config_con(
        "reportes:\n  - id: examenes_salida\n    titulo: E\n    descripcion: D\n    fuente: examenes_salida\n    varios_archivos: false\n    complementos: []\n    excel: examenes_salida\n    genero: masculino\n",
    );
    let sin_nada = tempfile::tempdir().unwrap();
    let e = estado_reportes(dir.path(), sin_nada.path(), dia(2025, 12, 5)).unwrap();
    assert_eq!(e[0].vigencia, Vigencia::Nunca);

    let archivo = comun::otro("limpio", "examenes_salida").remove(0);
    let s = salida_con(&[("examenes_salida", &archivo)]);
    let e = estado_reportes(dir.path(), s.path(), dia(2025, 12, 5)).unwrap();
    assert_eq!(e[0].vigencia, Vigencia::AlDia);
    assert_eq!(e[0].ultimo, None);
}
