-- Plan 5, decisión 9: planilla vacía cuando actual/ no trae gold/planilla.parquet.
-- Solo las columnas que usa config/sql/indicadores/.
CREATE OR REPLACE TABLE planilla (
    periodo VARCHAR, anio INTEGER, mes INTEGER, numero_documento VARCHAR,
    apellido_paterno VARCHAR, apellido_materno VARCHAR, nombres VARCHAR, area VARCHAR, centro_costo VARCHAR,
    fecha_entrada DATE, fecha_salida DATE, motivo_salida VARCHAR,
    dias_descanso_medico BIGINT, dias_licencia_con_goce BIGINT, dias_licencia_sin_goce BIGINT,
    dias_licencia_paternidad BIGINT, dias_subsidio_enfermedad BIGINT, dias_subsidio_maternidad BIGINT,
    dias_subsidio_otros BIGINT,
    horas_extra_25 DOUBLE, horas_extra_35 DOUBLE, horas_extra_50 DOUBLE, horas_extra_100 DOUBLE
);
