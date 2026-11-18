-- Desglose por gerencia (Plan 7, decisión del usuario 2): IND-01, IND-07 e IND-08.
-- La gerencia sale de la base de personal; quien no tiene registro en ella no entra (eso lo cuenta IND-C1).
CREATE OR REPLACE TABLE ind_por_gerencia AS
SELECT 'IND-01' AS id, p.gerencia, count(DISTINCT p.numero_documento)::DOUBLE AS valor
FROM planilla_f AS p
WHERE p.periodo = getvariable('f_periodo')
  AND p.gerencia IS NOT NULL AND trim(p.gerencia) <> ''
GROUP BY p.gerencia
UNION ALL
SELECT 'IND-07', e.gerencia, count(*) FILTER (WHERE e.alerta_contrato_plazo)::DOUBLE
FROM empleados_f AS e
WHERE e.gerencia IS NOT NULL AND trim(e.gerencia) <> ''
GROUP BY e.gerencia
HAVING count(*) FILTER (WHERE e.alerta_contrato_plazo) > 0
UNION ALL
SELECT 'IND-08', e.gerencia, count(*) FILTER (WHERE e.alerta_jubilacion)::DOUBLE
FROM empleados_f AS e
WHERE e.gerencia IS NOT NULL AND trim(e.gerencia) <> ''
GROUP BY e.gerencia
HAVING count(*) FILTER (WHERE e.alerta_jubilacion) > 0;
