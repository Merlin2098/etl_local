-- Planilla del régimen especial Gold (Plan 8). Entrada: planilla_regimen_especial_lectura (molde: hoja, periodo).
-- RM-SLV-07 / RM-GLD-06: anio, mes y nombre_mes del periodo; en español, como la planilla estándar (NOM-GLD-05).
-- Sin nombres (RM-GLD-03) ni montos (TR-D22): el contrato no los lee. Sin enriquecimiento con licencias.
CREATE OR REPLACE TABLE planilla_regimen_especial_gold AS
SELECT
    periodo,
    substr(periodo, 1, 4)::INTEGER AS anio,
    substr(periodo, 6, 2)::INTEGER AS mes,
    CASE substr(periodo, 6, 2)::INTEGER
        WHEN 1 THEN 'Enero' WHEN 2 THEN 'Febrero' WHEN 3 THEN 'Marzo' WHEN 4 THEN 'Abril'
        WHEN 5 THEN 'Mayo' WHEN 6 THEN 'Junio' WHEN 7 THEN 'Julio' WHEN 8 THEN 'Agosto'
        WHEN 9 THEN 'Septiembre' WHEN 10 THEN 'Octubre' WHEN 11 THEN 'Noviembre' WHEN 12 THEN 'Diciembre'
    END AS nombre_mes,
    * EXCLUDE (periodo, hoja, archivo_origen, fila_origen)
FROM planilla_regimen_especial_lectura
ORDER BY periodo, archivo_origen, fila_origen;
