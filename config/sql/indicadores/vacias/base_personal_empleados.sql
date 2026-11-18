-- Plan 5, decisión 9: base de personal vacía cuando actual/ no trae gold/base_personal_empleados.parquet.
-- Solo las columnas que usa config/sql/indicadores/.
CREATE OR REPLACE TABLE base_personal_empleados (
    numero_documento VARCHAR, nombre_completo VARCHAR, gerencia VARCHAR, sede VARCHAR, tipo_trabajador VARCHAR,
    modalidad_contrato VARCHAR, fecha_fin_contrato DATE, fecha_nacimiento DATE, fecha_corte DATE,
    cumple_65_este_anio BOOLEAN, cumple_65_proximo_anio BOOLEAN, cumple_70_este_anio BOOLEAN,
    cumple_70_proximo_anio BOOLEAN,
    alerta_contrato_obra BOOLEAN, alerta_contrato_incremento BOOLEAN,
    alerta_contrato_plazo BOOLEAN, alerta_jubilacion BOOLEAN
);
