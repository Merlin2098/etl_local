-- Control de practicantes Gold (Plan 8). Entrada: practicantes_lectura (molde: hoja, fecha_corte, origen).
-- CP-FLG-01…08 con la fórmula del legacy: D = fecha_renovacion − fecha_entrada (diseño §5.2).
-- Correcciones: alertas BOOLEAN; condición comparada normalizada (TR-D24); D nulo o negativo → todo nulo
-- (aviso PRA-01); singular «1 mes» / «1 año»; años de 365 días (el legacy usaba 365,25 y a los 365 días
-- decía «11 meses» con la alerta «cumplió 1 año»).
-- Variables: cp_condicion (normalizada), cp_pc1_desde, cp_pc1_hasta, cp_c1_desde, cp_pc2_desde, cp_pc2_hasta.
CREATE OR REPLACE TABLE practicantes_gold AS
WITH
    base AS (
        SELECT
            *,
            CASE WHEN fecha_renovacion - fecha_entrada >= 0 THEN fecha_renovacion - fecha_entrada END AS d,
            normalizar(condicion) = getvariable('cp_condicion') AS profesional
        FROM practicantes_lectura
    ),
    calculo AS (
        SELECT
            *,
            floor(d / 365)::INTEGER AS anios,
            floor((d % 365) / 30.44)::INTEGER AS meses,
            CASE WHEN d IS NOT NULL THEN profesional
                AND d BETWEEN getvariable('cp_pc1_desde') AND getvariable('cp_pc1_hasta') END AS pc1,
            CASE WHEN d IS NOT NULL THEN profesional AND d >= getvariable('cp_c1_desde') END AS c1,
            CASE WHEN d IS NOT NULL THEN profesional
                AND d BETWEEN getvariable('cp_pc2_desde') AND getvariable('cp_pc2_hasta') END AS pc2
        FROM base
    )
SELECT
    numero_documento,
    condicion,
    fecha_entrada,
    fecha_renovacion,
    fecha_corte,
    sede,
    universidad,
    jefe_inmediato,
    gerencia,
    d AS dias_servicio,
    anios AS anios_servicio,
    meses AS meses_servicio,
    CASE
        WHEN d IS NULL THEN NULL
        WHEN pc1 THEN '1 año (En límite)'
        WHEN pc2 THEN '2 años (Límite Máximo)'
        WHEN anios = 0 THEN meses || CASE WHEN meses = 1 THEN ' mes' ELSE ' meses' END
        WHEN meses = 0 THEN anios || CASE WHEN anios = 1 THEN ' año' ELSE ' años' END
        ELSE anios || CASE WHEN anios = 1 THEN ' año' ELSE ' años' END
            || ' y ' || meses || CASE WHEN meses = 1 THEN ' mes' ELSE ' meses' END
    END AS tiempo_servicio,
    pc1 AS por_cumplir_1,
    c1 AS cumplio_1,
    pc2 AS por_cumplir_2
FROM calculo
ORDER BY fecha_entrada DESC NULLS LAST, archivo_origen, fila_origen;
