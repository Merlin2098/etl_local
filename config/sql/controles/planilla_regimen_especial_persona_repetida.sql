-- PRE-RE-01 (Plan 8, RM-GLD-08 / TR-D23): una persona repetida en el mismo periodo de la planilla del régimen
-- especial se conserva, con un aviso que dice las filas.
SELECT
    'PRE-RE-01' AS codigo,
    'aviso' AS severidad,
    min(archivo_origen) AS archivo,
    NULL AS hoja,
    NULL AS celda,
    numero_documento || ' aparece ' || count(*) || ' veces en ' || periodo
        || ' (filas ' || string_agg(fila_origen::VARCHAR, ', ' ORDER BY fila_origen) || ').' AS mensaje
FROM planilla_regimen_especial_lectura
GROUP BY periodo, numero_documento
HAVING count(*) > 1
ORDER BY periodo, numero_documento
