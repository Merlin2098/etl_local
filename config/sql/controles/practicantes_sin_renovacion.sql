-- PRA-01 (Plan 8, CP §9.4): practicantes sin fecha de renovación o con renovación anterior al ingreso.
SELECT
    'PRA-01' AS codigo,
    'aviso' AS severidad,
    min(archivo_origen) AS archivo,
    min(hoja) AS hoja,
    NULL AS celda,
    count(*) || ' practicantes sin fecha de renovación o con renovación anterior al ingreso: no se calcula su '
        || 'tiempo de servicio (filas ' || string_agg(fila_origen::VARCHAR, ', ' ORDER BY fila_origen) || ').'
        AS mensaje
FROM practicantes_lectura
WHERE fecha_renovacion IS NULL OR fecha_entrada IS NULL OR fecha_renovacion < fecha_entrada
HAVING count(*) > 0
