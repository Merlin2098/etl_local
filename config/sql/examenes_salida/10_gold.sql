-- Exámenes médicos de salida Gold (Plan 8). Entradas: examenes_salida_lectura (molde) y centros_costo (de la base
-- de personal de la misma ejecución; vacía si no se cargó la base).
-- ER-GLD-04: anio, mes y nombre_mes de la fecha de cese, en español (el legacy: inglés).
-- ER-GLD-06: se excluyen los practicantes por cargo, comparado normalizado (variable es_cargo_practicante).
-- ER-JOIN-02…04 simplificado: solo el catálogo actual (sin CC_OLD) y código comparado con trim y mayúsculas.
CREATE OR REPLACE TABLE examenes_salida_gold AS
WITH
    -- Un código por fila después de normalizarlo (revisión final): «ab01» y «AB01» no duplican el examen.
    catalogo AS (
        SELECT upper(trim(centro_costo)) AS codigo, nombre_centro_costo, categoria_centro_costo, gerencia
        FROM centros_costo
        QUALIFY row_number() OVER (PARTITION BY upper(trim(centro_costo)) ORDER BY centro_costo) = 1
    ),
    hay_catalogo AS (SELECT count(*) > 0 AS hay FROM catalogo)
SELECT
    e.nombre_completo,
    e.numero_documento,
    e.fecha_salida,
    year(e.fecha_salida) AS anio,
    month(e.fecha_salida) AS mes,
    CASE month(e.fecha_salida)
        WHEN 1 THEN 'Enero' WHEN 2 THEN 'Febrero' WHEN 3 THEN 'Marzo' WHEN 4 THEN 'Abril'
        WHEN 5 THEN 'Mayo' WHEN 6 THEN 'Junio' WHEN 7 THEN 'Julio' WHEN 8 THEN 'Agosto'
        WHEN 9 THEN 'Septiembre' WHEN 10 THEN 'Octubre' WHEN 11 THEN 'Noviembre' WHEN 12 THEN 'Diciembre'
    END AS nombre_mes,
    e.motivo_salida,
    e.cargo,
    e.centro_costo,
    c.nombre_centro_costo,
    c.categoria_centro_costo,
    c.gerencia,
    CASE
        WHEN NOT h.hay THEN 'sin_catalogo'
        WHEN nullif(trim(e.centro_costo), '') IS NULL THEN 'sin_codigo'
        WHEN c.codigo IS NULL THEN 'codigo_no_encontrado'
        ELSE 'ok'
    END AS estado_cruce
FROM examenes_salida_lectura AS e
CROSS JOIN hay_catalogo AS h
LEFT JOIN catalogo AS c ON c.codigo = upper(trim(e.centro_costo))
WHERE coalesce(normalizar(e.cargo), '') <> getvariable('es_cargo_practicante')
ORDER BY e.fecha_salida, e.archivo_origen, e.fila_origen;
