-- BD-SLV-07 / TR-D4: documento repetido en la base de personal; en el Gold queda la primera fila.
SELECT
    'BD-SLV-07' AS codigo,
    'aviso' AS severidad,
    min(archivo_origen) AS archivo,
    NULL AS hoja,
    NULL AS celda,
    numero_documento || ' aparece ' || count(*) || ' veces en la base de personal (filas '
        || string_agg(fila_origen::VARCHAR, ', ' ORDER BY fila_origen) || '); se usó la primera.' AS mensaje
FROM base_personal_silver
GROUP BY numero_documento
HAVING count(*) > 1
ORDER BY numero_documento
