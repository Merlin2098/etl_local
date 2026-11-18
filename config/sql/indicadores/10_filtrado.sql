-- Filtros del tablero (Plan 5). `filtros` la crea el motor con parámetros preparados: el texto del pedido
-- nunca entra en el SQL (ADR-0004 §2). Un filtro nulo deja pasar todo.
SET VARIABLE f_periodo = (SELECT periodo FROM filtros);
SET VARIABLE f_gerencia = (SELECT gerencia FROM filtros);
SET VARIABLE f_sede = (SELECT sede FROM filtros);
SET VARIABLE f_tipo_trabajador = (SELECT tipo_trabajador FROM filtros);

-- indicadores-tablero §1: gerencia, sede y tipo de trabajador vienen de la base de personal.
CREATE OR REPLACE VIEW empleados_f AS
SELECT *
FROM base_personal_empleados
WHERE (getvariable('f_gerencia') IS NULL OR gerencia = getvariable('f_gerencia'))
  AND (getvariable('f_sede') IS NULL OR sede = getvariable('f_sede'))
  AND (getvariable('f_tipo_trabajador') IS NULL OR tipo_trabajador = getvariable('f_tipo_trabajador'));

-- Plan 5, decisión 5: con un filtro de dimensión, quien no está en la base queda fuera (lo explica IND-C1).
CREATE OR REPLACE VIEW planilla_f AS
SELECT p.*, b.gerencia, b.sede, b.tipo_trabajador
FROM planilla AS p
LEFT JOIN base_personal_empleados AS b ON b.numero_documento = p.numero_documento
WHERE (getvariable('f_gerencia') IS NULL OR b.gerencia = getvariable('f_gerencia'))
  AND (getvariable('f_sede') IS NULL OR b.sede = getvariable('f_sede'))
  AND (getvariable('f_tipo_trabajador') IS NULL OR b.tipo_trabajador = getvariable('f_tipo_trabajador'));
