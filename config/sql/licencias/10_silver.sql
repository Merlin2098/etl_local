-- Licencias Silver: las dos hojas en una tabla (Plan 4, Tarea 4). Se ejecuta antes que la planilla Gold.
-- Entrada: licencias_lectura (tipo_licencia lo pone Rust según la hoja, LIC-SLV-05).
-- LIC-SLV-01: el motivo de las dos hojas ya llega como motivo_licencia (contrato).
-- LIC-SLV-04/06: se excluyen filas sin motivo, documento o periodo.
-- LIC-SLV-09: orden de columnas.
CREATE OR REPLACE TABLE licencias_silver AS
SELECT
    periodo,
    numero_documento,
    apellido_paterno,
    apellido_materno,
    nombres,
    motivo_licencia,
    tipo_licencia,
    archivo_origen,
    fila_origen
FROM licencias_lectura
WHERE nullif(trim(motivo_licencia), '') IS NOT NULL
  AND nullif(trim(numero_documento), '') IS NOT NULL
  AND nullif(trim(periodo), '') IS NOT NULL
ORDER BY tipo_licencia, fila_origen;
