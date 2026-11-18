//! JSON Schema de las respuestas de la API (Plan 7): de aquí salen los tipos TypeScript del tablero
//! (`app/ui/src/lib/api/tipos.gen.ts`, con `npm run tipos`).

use schemars::JsonSchema;
use schemars::generate::SchemaSettings;
use serde_json::{Map, Value};

use super::Filtros;
use super::respuesta::{
    ErrorApi, Estado, OpcionesFiltro, RespuestaChecks, RespuestaDetalle, RespuestaIndicadores,
    RespuestaSeries,
};

/// Arma un documento con un `$defs` por tipo (más sus dependencias) y nada en la raíz.
pub fn esquema_de(agregar: impl FnOnce(&mut schemars::SchemaGenerator)) -> Value {
    let mut generador = SchemaSettings::draft2020_12().into_generator();
    agregar(&mut generador);
    let defs: Map<String, Value> = generador.take_definitions(true).into_iter().collect();
    serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$defs": defs,
    })
}

/// Agrega `T` al generador como definición con nombre.
pub fn definir<T: JsonSchema>(g: &mut schemars::SchemaGenerator) {
    g.subschema_for::<T>();
}

/// Esquema de todas las respuestas de `/api/v1/`.
pub fn esquema_api() -> Value {
    esquema_de(|g| {
        definir::<RespuestaIndicadores>(g);
        definir::<Estado>(g);
        definir::<OpcionesFiltro>(g);
        definir::<RespuestaSeries>(g);
        definir::<RespuestaChecks>(g);
        definir::<RespuestaDetalle>(g);
        definir::<Filtros>(g);
        definir::<ErrorApi>(g);
    })
}
