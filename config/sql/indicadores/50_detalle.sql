-- Detalle nominal (TR-D18): lo único del tablero que lleva nombres y documentos. Solo lo usa la pestaña
-- "Alertas (solo RR. HH.)" a través de /api/v1/detalle/….

-- Personas con alguna alerta de IND-07 o IND-08, con los filtros de dimensión.
CREATE OR REPLACE VIEW detalle_alertas AS
SELECT
    numero_documento, nombre_completo, gerencia, sede, tipo_trabajador, modalidad_contrato,
    fecha_fin_contrato, fecha_nacimiento,
    alerta_contrato_obra, alerta_contrato_incremento,
    cumple_65_este_anio, cumple_65_proximo_anio, cumple_70_este_anio, cumple_70_proximo_anio
FROM empleados_f
WHERE alerta_contrato_plazo OR alerta_jubilacion
ORDER BY gerencia, sede, nombre_completo, numero_documento;

-- IND-C1 "Ver quiénes": personas de la planilla del mes sin registro en la base (sin filtros de dimensión).
CREATE OR REPLACE VIEW detalle_sin_base AS
SELECT DISTINCT
    p.numero_documento, p.apellido_paterno, p.apellido_materno, p.nombres, p.area, p.centro_costo
FROM planilla AS p
WHERE p.periodo = getvariable('f_periodo')
  AND NOT EXISTS (
      SELECT 1 FROM base_personal_empleados AS b WHERE b.numero_documento = p.numero_documento
  )
ORDER BY p.apellido_paterno, p.apellido_materno, p.nombres, p.numero_documento;
