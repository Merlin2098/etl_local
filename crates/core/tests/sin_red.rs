//! El motor no descarga nada de internet (Plan 9): la extensión `parquet` de DuckDB va enlazada en el
//! binario y la instalación o carga automática de extensiones está desactivada. Sin esto, la app
//! instalada necesitaba internet la primera vez que escribía un Parquet, y en CI dos tests chocaban al
//! instalarla a la vez.

use etl_local_core::almacen::Almacen;

#[test]
fn no_instala_ni_carga_extensiones_automaticamente() {
    let almacen = Almacen::nuevo().unwrap();
    almacen
        .ejecutar(
            "SELECT CASE WHEN current_setting('autoinstall_known_extensions') \
                OR current_setting('autoload_known_extensions') \
             THEN error('DuckDB puede descargar o cargar extensiones por su cuenta') END",
        )
        .unwrap();
}

#[test]
fn escribe_y_lee_parquet_sin_extensiones_externas() {
    let dir = tempfile::tempdir().unwrap();
    let ruta = dir.path().join("prueba.parquet");
    let almacen = Almacen::nuevo().unwrap();
    almacen
        .ejecutar(&format!(
            "COPY (SELECT 1 AS uno) TO '{}' (FORMAT parquet);",
            ruta.display().to_string().replace('\\', "/")
        ))
        .unwrap();
    almacen.tabla_parquet("leida", &ruta).unwrap();
    almacen
        .ejecutar("SELECT CASE WHEN (SELECT count(*) FROM leida) <> 1 THEN error('no se leyó') END")
        .unwrap();
}
