-- LIC-ENR-04: licencias cuya persona no está en la planilla del mismo periodo (el legacy las perdía sin aviso).
-- Solo cuentan los periodos que tienen planilla: las licencias de otros meses no son huérfanas.
SELECT
    'LIC-ENR-04' AS codigo,
    'aviso' AS severidad,
    NULL AS archivo,
    NULL AS hoja,
    NULL AS celda,
    count(*) || ' licencias no tienen a la persona en la planilla del mismo periodo.' AS mensaje
FROM licencias_silver AS l
WHERE l.periodo IN (SELECT DISTINCT periodo FROM planilla_silver)
  AND NOT EXISTS (
    SELECT 1 FROM planilla_silver AS p
    WHERE p.periodo = l.periodo AND p.numero_documento = l.numero_documento
)
HAVING count(*) > 0
