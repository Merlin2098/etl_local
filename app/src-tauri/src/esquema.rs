//! JSON Schema de lo que reciben las pantallas 1–5 (Plan 7, Tarea 2): de aquí salen
//! `app/ui/src/app/tipos.gen.ts` (`npm --prefix app/ui run tipos`). Para regenerarlo:
//! `ACTUALIZAR_ESQUEMA=1 cargo test -p etl-local-app --lib esquema`.

use std::path::Path;

use etl_local_core::proceso::Avance;
use etl_local_core::tablero::esquema::{definir, esquema_de};

use crate::error::ErrorApp;
use crate::servicio::{ArchivoElegido, Inicio, Resultado, RevisionArchivo};

fn esquema_app() -> serde_json::Value {
    esquema_de(|g| {
        definir::<Inicio>(g);
        definir::<ArchivoElegido>(g);
        definir::<RevisionArchivo>(g);
        definir::<Avance>(g);
        definir::<Resultado>(g);
        definir::<ErrorApp>(g);
    })
}

#[test]
fn esquema_app_al_dia() {
    let json = serde_json::to_string_pretty(&esquema_app()).unwrap() + "\n";
    let ruta = Path::new(env!("CARGO_MANIFEST_DIR")).join("../ui/src/app/esquema.json");
    if std::env::var_os("ACTUALIZAR_ESQUEMA").is_some() {
        std::fs::write(&ruta, &json).unwrap();
    }
    let guardado = std::fs::read_to_string(&ruta).unwrap_or_default();
    assert_eq!(
        guardado.replace("\r\n", "\n"),
        json,
        "esquema.json desactualizado: ACTUALIZAR_ESQUEMA=1 y luego npm --prefix app/ui run tipos"
    );
}
