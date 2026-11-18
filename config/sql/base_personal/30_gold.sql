-- Base de personal Gold: practicantes y empleados con flags y alertas (Plan 4, Tarea 5).
-- Variables (config/parametros.yaml, TR-D10): edad_jubilacion, edad_jubilacion_extendida, modalidad_obra,
-- anios_alerta_obra, modalidad_incremento, anios_alerta_incremento, practicante_contiene (ya normalizadas).

-- TR-D4: una fila por documento (la primera del archivo); el control BD-SLV-07 avisa de las repetidas.
CREATE OR REPLACE TABLE base_personal_unica AS
SELECT * EXCLUDE (orden)
FROM (
    SELECT *, row_number() OVER (PARTITION BY numero_documento ORDER BY fila_origen) AS orden
    FROM base_personal_silver
)
WHERE orden = 1;

-- BD-GLD-06 con TR-D24: practicante si la modalidad normalizada contiene el texto del parámetro.
-- BD-GLD-01: 13 columnas del contrato + fecha_corte.
CREATE OR REPLACE TABLE base_personal_practicantes_gold AS
SELECT
    numero_documento, codigo_empleado, nombre_completo, gerencia, sexo, sede, tipo_trabajador,
    modalidad_contrato, fecha_fin_contrato, servicio, regimen_trabajo, fecha_entrada, fecha_nacimiento,
    fecha_corte
FROM base_personal_unica
WHERE contains(coalesce(normalizar(modalidad_contrato), ''), getvariable('practicante_contiene'))
ORDER BY fila_origen;

-- Empleados: el resto, incluidas las personas sin modalidad (TR-D24, corrige BD-GLD-07).
-- Flags en un solo SELECT sobre una CTE, sin auto-joins (TR-D4, corrige BD-FLG-12).
CREATE OR REPLACE TABLE base_personal_empleados_gold AS
WITH
    base AS (
        SELECT
            *,
            normalizar(modalidad_contrato) AS modalidad_normalizada,
            -- BD-FLG-02/03: antigüedad hasta el fin de contrato o, si no hay, hasta la fecha de corte (TR-D3).
            date_diff('day', fecha_entrada, coalesce(fecha_fin_contrato, fecha_corte)) / 365.25 AS anios
        FROM base_personal_unica
        WHERE NOT contains(coalesce(normalizar(modalidad_contrato), ''), getvariable('practicante_contiene'))
    ),
    flags AS (
        SELECT
            *,
            -- BD-FLG-04: formato legacy; nulo sin fecha de entrada o con antigüedad negativa (corrige Q-15).
            CASE WHEN anios >= 0 THEN
                floor(anios)::BIGINT || ' años, ' || floor((anios - floor(anios)) * 12)::BIGINT || ' meses'
            END AS tiempo_servicio_texto,
            -- BD-FLG-05…07, TR-D15: «cumple N durante el año de corte / el siguiente», por año de nacimiento.
            year(fecha_nacimiento) = year(fecha_corte) - getvariable('edad_jubilacion') AS cumple_65_este_anio,
            year(fecha_nacimiento) = year(fecha_corte) + 1 - getvariable('edad_jubilacion') AS cumple_65_proximo_anio,
            year(fecha_nacimiento) = year(fecha_corte) - getvariable('edad_jubilacion_extendida') AS cumple_70_este_anio,
            year(fecha_nacimiento) = year(fecha_corte) + 1 - getvariable('edad_jubilacion_extendida') AS cumple_70_proximo_anio,
            -- BD-FLG-08…10 con TR-D24: modalidad normalizada; lógica de tres valores como el legacy.
            modalidad_normalizada = getvariable('modalidad_obra')
                AND anios >= getvariable('anios_alerta_obra') AS alerta_contrato_obra,
            modalidad_normalizada = getvariable('modalidad_incremento')
                AND anios >= getvariable('anios_alerta_incremento') AS alerta_contrato_incremento
        FROM base
    )
SELECT
    numero_documento, codigo_empleado, nombre_completo, gerencia, sexo, sede, tipo_trabajador,
    modalidad_contrato, fecha_fin_contrato, servicio, regimen_trabajo, fecha_entrada, fecha_nacimiento,
    fecha_corte,
    tiempo_servicio_texto,
    cumple_65_este_anio, cumple_65_proximo_anio, cumple_70_este_anio, cumple_70_proximo_anio,
    alerta_contrato_obra, alerta_contrato_incremento,
    -- IND-07 / IND-08: nunca nulas.
    coalesce(alerta_contrato_obra, false) OR coalesce(alerta_contrato_incremento, false) AS alerta_contrato_plazo,
    coalesce(cumple_65_este_anio OR cumple_65_proximo_anio OR cumple_70_este_anio OR cumple_70_proximo_anio, false)
        AS alerta_jubilacion
FROM flags
ORDER BY fila_origen;
