-- NOM-GLD-15 / TR-D23: una persona repetida en el mismo periodo se conserva, con un aviso que dice las filas.
-- Columnas de todo control: codigo, severidad (error | aviso), archivo, hoja, celda, mensaje.
SELECT
    'NOM-GLD-15' AS codigo,
    'aviso' AS severidad,
    min(archivo_origen) AS archivo,
    NULL AS hoja,
    NULL AS celda,
    numero_documento || ' aparece ' || count(*) || ' veces en ' || periodo
        || ' (filas ' || string_agg(fila_origen::VARCHAR, ', ' ORDER BY fila_origen) || ').' AS mensaje
FROM planilla_silver
GROUP BY periodo, numero_documento
HAVING count(*) > 1
ORDER BY periodo, numero_documento
