use std::path::Path;

use etl_local_core::contrato::{Contrato, cargar_contratos};

const CARPETA_CONTRATOS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../config/fuentes");
const ESQUEMA: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../config/esquema-contrato.json"
);

#[test]
fn cargan_las_fuentes() {
    let todos = cargar_contratos(Path::new(CARPETA_CONTRATOS)).unwrap();
    assert_eq!(
        todos.keys().collect::<Vec<_>>(),
        vec![
            "altas_personal",
            "base_personal",
            "examenes_salida",
            "licencias",
            "planilla",
            "planilla_regimen_especial",
            "practicantes"
        ]
    );
}

#[test]
fn cada_hoja_tiene_una_clave() {
    for (fuente, contrato) in cargar_contratos(Path::new(CARPETA_CONTRATOS)).unwrap() {
        for hoja in &contrato.hojas {
            assert!(hoja.clave().is_some(), "{fuente}: {:?}", hoja.nombre);
        }
    }
}

#[test]
fn esquema_contrato_al_dia() {
    let generado = serde_json::to_string_pretty(&schemars::schema_for!(Contrato)).unwrap() + "\n";
    if std::env::var("ACTUALIZAR_ESQUEMA").as_deref() == Ok("1") {
        std::fs::write(ESQUEMA, &generado).unwrap();
    }
    let guardado = std::fs::read_to_string(ESQUEMA)
        .unwrap_or_default()
        .replace("\r\n", "\n");
    assert!(
        guardado == generado,
        "config/esquema-contrato.json está desactualizado. Regenerar con:\n\
         ACTUALIZAR_ESQUEMA=1 cargo test -p etl-local-core --test contratos_config esquema_contrato_al_dia"
    );
}
