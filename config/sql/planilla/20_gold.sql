-- Planilla Gold, ya con los motivos de licencia (Plan 4, Tarea 4).
-- Entradas: planilla_silver, licencias_silver (vacía si no se cargó el control de licencias).
-- NOM-GLD-05: nombre_mes después de mes; en español porque el tablero es en español (el legacy: January…).
-- NOM-ENR-01/02: un motivo por (periodo, numero_documento) y tipo, con LEFT JOIN: la planilla nunca pierde
--   ni multiplica filas. DISTINCT corrige el texto repetido cuando hay licencias duplicadas (decisión 6).
-- NOM-ENR-03/04: un solo Gold (el legacy escribía dos idénticos).
CREATE OR REPLACE TABLE planilla_gold AS
WITH
    con_goce AS (
        SELECT periodo, numero_documento,
               string_agg(DISTINCT motivo_licencia, ' | ' ORDER BY motivo_licencia) AS motivo
        FROM licencias_silver
        WHERE tipo_licencia = 'CON_GOCE'
        GROUP BY periodo, numero_documento
    ),
    sin_goce AS (
        SELECT periodo, numero_documento,
               string_agg(DISTINCT motivo_licencia, ' | ' ORDER BY motivo_licencia) AS motivo
        FROM licencias_silver
        WHERE tipo_licencia = 'SIN_GOCE'
        GROUP BY periodo, numero_documento
    )
SELECT
    s.periodo,
    s.anio,
    s.mes,
    CASE s.mes
        WHEN 1 THEN 'Enero' WHEN 2 THEN 'Febrero' WHEN 3 THEN 'Marzo' WHEN 4 THEN 'Abril'
        WHEN 5 THEN 'Mayo' WHEN 6 THEN 'Junio' WHEN 7 THEN 'Julio' WHEN 8 THEN 'Agosto'
        WHEN 9 THEN 'Septiembre' WHEN 10 THEN 'Octubre' WHEN 11 THEN 'Noviembre' WHEN 12 THEN 'Diciembre'
    END AS nombre_mes,
    s.* EXCLUDE (periodo, anio, mes, archivo_origen, fila_origen),
    cg.motivo AS motivo_licencia_con_goce,
    sg.motivo AS motivo_licencia_sin_goce
FROM planilla_silver AS s
LEFT JOIN con_goce AS cg ON cg.periodo = s.periodo AND cg.numero_documento = s.numero_documento
LEFT JOIN sin_goce AS sg ON sg.periodo = s.periodo AND sg.numero_documento = s.numero_documento
ORDER BY s.periodo, s.archivo_origen, s.fila_origen;
