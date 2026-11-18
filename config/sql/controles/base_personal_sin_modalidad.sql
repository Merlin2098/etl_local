-- BD-GLD-07 / TR-D24: personas sin modalidad de contrato; se cuentan como empleados.
SELECT
    'BD-GLD-07' AS codigo,
    'aviso' AS severidad,
    min(archivo_origen) AS archivo,
    NULL AS hoja,
    NULL AS celda,
    count(*) || ' personas sin modalidad de contrato; se contaron como empleados (filas '
        || string_agg(fila_origen::VARCHAR, ', ' ORDER BY fila_origen) || ').' AS mensaje
FROM base_personal_unica
WHERE nullif(trim(modalidad_contrato), '') IS NULL
HAVING count(*) > 0
