-- Altas de personal Gold (Plan 8). Entrada: altas_personal_lectura (molde: hoja, periodo), unión de las hojas
-- EMPLEADOS y PRACTICANTES por nombre: lo que una hoja no tiene queda nulo.
-- PDT-NEG-01/03: anio, mes y nombre_mes del periodo del nombre del archivo, en español (el legacy: inglés).
-- Un solo Gold con `tipo` según la hoja (el legacy escribía uno por hoja). Sin imputaciones de SERVICIO/REGIMEN
-- ni «POR DEFINIR» (PDT §9.2–9.3).
CREATE OR REPLACE TABLE altas_personal_gold AS
SELECT
    periodo,
    substr(periodo, 1, 4)::INTEGER AS anio,
    substr(periodo, 6, 2)::INTEGER AS mes,
    CASE substr(periodo, 6, 2)::INTEGER
        WHEN 1 THEN 'Enero' WHEN 2 THEN 'Febrero' WHEN 3 THEN 'Marzo' WHEN 4 THEN 'Abril'
        WHEN 5 THEN 'Mayo' WHEN 6 THEN 'Junio' WHEN 7 THEN 'Julio' WHEN 8 THEN 'Agosto'
        WHEN 9 THEN 'Septiembre' WHEN 10 THEN 'Octubre' WHEN 11 THEN 'Noviembre' WHEN 12 THEN 'Diciembre'
    END AS nombre_mes,
    CASE WHEN normalizar(hoja) = 'PRACTICANTES' THEN 'practicante' ELSE 'empleado' END AS tipo,
    * EXCLUDE (periodo, hoja, archivo_origen, fila_origen)
FROM altas_personal_lectura
ORDER BY periodo, tipo, fecha_entrada, archivo_origen, fila_origen;
