// Plan 7 (revisión final): con un manifiesto de la app, sus comandos pasan por el ACL de Tauri y solo los
// recibe la ventana `principal` (capabilities/principal.json). La ventana del tablero no tiene capacidades.
const COMANDOS: &[&str] = &[
    "inicio",
    "reconocer",
    "complemento_guardado",
    "revisar",
    "procesar",
    "detener",
    "abrir",
    "abrir_tablero",
];

fn main() {
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .app_manifest(tauri_build::AppManifest::new().commands(COMANDOS)),
    )
    .expect("no se pudo preparar la compilación de la app");
}
