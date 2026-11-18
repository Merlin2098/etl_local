-- Base de personal Silver (Plan 4, Tarea 5).
-- Entrada: base_personal_lectura (fecha_corte sale del nombre del archivo, BD-SLV-08 / TR-D3).
-- BD-SLV-01: todas las columnas del contrato, en orden; BD-SLV-07: sin filtros ni deduplicación aquí.
CREATE OR REPLACE TABLE base_personal_silver AS
SELECT
    numero_documento,
    codigo_empleado,
    nombre_completo,
    gerencia,
    sexo,
    sede,
    tipo_trabajador,
    modalidad_contrato,
    fecha_fin_contrato,
    servicio,
    regimen_trabajo,
    fecha_entrada,
    fecha_nacimiento,
    centro_costo,
    nombre_centro_costo,
    categoria_centro_costo,
    fecha_corte,
    archivo_origen,
    fila_origen
FROM base_personal_lectura
ORDER BY fila_origen;
