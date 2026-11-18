-- EXS-01 (Plan 8): sin base de personal en la ejecución, los exámenes de salida no tienen centro de costo.
SELECT
    'EXS-01' AS codigo,
    'aviso' AS severidad,
    NULL AS archivo,
    NULL AS hoja,
    NULL AS celda,
    'Cargue la base de personal para completar el centro de costo de los exámenes de salida.' AS mensaje
FROM examenes_salida_gold
WHERE estado_cruce = 'sin_catalogo'
HAVING count(*) > 0
