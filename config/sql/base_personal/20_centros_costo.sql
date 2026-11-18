-- Catálogo de centros de costo (BD-CC-01…03), desde toda la base (empleados y practicantes).
-- Primera aparición por centro_costo según el orden del archivo; se excluyen los nulos (decisión 7 del Plan 4);
-- orden por texto, como el legacy.
CREATE OR REPLACE TABLE centros_costo AS
SELECT centro_costo, nombre_centro_costo, categoria_centro_costo, gerencia
FROM (
    SELECT *, row_number() OVER (PARTITION BY centro_costo ORDER BY fila_origen) AS orden
    FROM base_personal_silver
    WHERE centro_costo IS NOT NULL
)
WHERE orden = 1
ORDER BY centro_costo;
