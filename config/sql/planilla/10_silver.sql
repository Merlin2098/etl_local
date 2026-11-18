-- Planilla Silver: unión de los meses leídos (Plan 4, Tarea 4).
-- Entrada: planilla_lectura (una fila por persona y mes; `periodo` sale del nombre del archivo).
-- NOM-SLV-03: periodo primero.
-- NOM-SLV-06: unión por nombre de columna (la hace Rust con UNION ALL BY NAME).
-- NOM-SLV-08: anio y mes salen del periodo (enteros; el legacy usaba texto para anio).
-- NOM-SLV-09: orden periodo, anio, mes y luego las columnas del contrato.
-- NOM-SLV-04 / vocabulario §4: dias_subsidio_maternidad siempre existe; dias_subsidio_otros suma el resto.
CREATE OR REPLACE TABLE planilla_silver AS
SELECT
    periodo,
    CAST(substr(periodo, 1, 4) AS INTEGER) AS anio,
    CAST(substr(periodo, 6, 2) AS INTEGER) AS mes,
    codigo_empleado,
    apellido_paterno,
    apellido_materno,
    nombres,
    tipo_documento,
    numero_documento,
    sexo,
    fecha_nacimiento,
    area,
    centro_costo,
    fecha_entrada,
    fecha_salida,
    motivo_salida,
    dias_descanso_medico,
    dias_licencia_con_goce,
    dias_licencia_sin_goce,
    dias_licencia_paternidad,
    dias_subsidio_enfermedad,
    horas_extra_25,
    horas_extra_35,
    horas_extra_50,
    horas_extra_100,
    dias_subsidio_maternidad,
    dias_subsidio_otros,
    archivo_origen,
    fila_origen
FROM planilla_lectura
ORDER BY periodo, archivo_origen, fila_origen;
