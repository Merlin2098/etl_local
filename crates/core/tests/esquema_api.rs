//! Plan 7, Tarea 2: el JSON Schema de las respuestas de la API está al día con los tipos de Rust.
//! Regenerar: `ACTUALIZAR_ESQUEMA=1 cargo test -p etl-local-core --test esquema_api` y luego
//! `npm --prefix app/ui run tipos`.

use std::path::Path;

use etl_local_core::tablero::esquema::esquema_api;

#[test]
fn esquema_api_al_dia() {
    let json = serde_json::to_string_pretty(&esquema_api()).unwrap() + "\n";
    let ruta = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../app/ui/src/lib/api/esquema.json");
    if std::env::var_os("ACTUALIZAR_ESQUEMA").is_some() {
        std::fs::create_dir_all(ruta.parent().unwrap()).unwrap();
        std::fs::write(&ruta, &json).unwrap();
    }
    let guardado = std::fs::read_to_string(&ruta).unwrap_or_default();
    assert_eq!(
        guardado.replace("\r\n", "\n"),
        json,
        "esquema.json desactualizado: ACTUALIZAR_ESQUEMA=1 y luego npm --prefix app/ui run tipos"
    );
}

#[test]
fn esquema_api_trae_cada_respuesta() {
    let e = esquema_api();
    for t in [
        "RespuestaIndicadores",
        "Estado",
        "OpcionesFiltro",
        "RespuestaSeries",
        "RespuestaChecks",
        "RespuestaDetalle",
        "Filtros",
        "ErrorApi",
    ] {
        assert!(e["$defs"].get(t).is_some(), "falta {t}");
    }
}
