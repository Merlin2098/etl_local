-- IND-01…06 por mes (indicadores-tablero §2–§3), para todos los periodos de la planilla.
-- Fecha de referencia: el periodo, nunca el reloj (TR-D3).

-- Periodos presentes en la planilla, sin filtros: un filtro sin personas igual da filas (en 0).
CREATE OR REPLACE TABLE ind_periodos AS
SELECT DISTINCT
    periodo,
    make_date(anio, mes, 1) AS inicio,
    last_day(make_date(anio, mes, 1)) AS fin,
    day(last_day(make_date(anio, mes, 1))) AS dias,  -- IND-05: días calendario del mes
    strftime(make_date(anio, mes, 1) - INTERVAL 1 MONTH, '%Y-%m') AS periodo_anterior
FROM planilla;

CREATE OR REPLACE TABLE ind_mes AS
SELECT
    t.periodo,
    t.dias,
    t.periodo_anterior,
    -- IND-01
    count(DISTINCT p.numero_documento) AS personas,
    -- IND-02
    count(DISTINCT p.numero_documento) FILTER (WHERE p.fecha_entrada BETWEEN t.inicio AND t.fin) AS entradas,
    -- IND-03
    count(DISTINCT p.numero_documento) FILTER (WHERE p.fecha_salida BETWEEN t.inicio AND t.fin) AS salidas,
    -- IND-05 §3: cada columna una sola vez; nunca el total legacy DIAS SUBSIDIOS TOTAL.
    -- Plan 5, decisión 10: se suman todas las filas del mes (incluida la persona repetida de TR-D23).
    coalesce(sum(
        coalesce(p.dias_descanso_medico, 0) + coalesce(p.dias_licencia_con_goce, 0)
        + coalesce(p.dias_licencia_sin_goce, 0) + coalesce(p.dias_licencia_paternidad, 0)
        + coalesce(p.dias_subsidio_enfermedad, 0) + coalesce(p.dias_subsidio_maternidad, 0)
        + coalesce(p.dias_subsidio_otros, 0)
    ), 0)::DOUBLE AS dias_ausencia,
    -- IND-06
    coalesce(sum(
        coalesce(p.horas_extra_25, 0) + coalesce(p.horas_extra_35, 0)
        + coalesce(p.horas_extra_50, 0) + coalesce(p.horas_extra_100, 0)
    ), 0)::DOUBLE AS horas_extra
FROM ind_periodos AS t
LEFT JOIN planilla_f AS p ON p.periodo = t.periodo
GROUP BY t.periodo, t.dias, t.periodo_anterior;

-- Valores por periodo (periodo NULL = no depende del periodo; ver 30_alertas.sql).
-- Todo divisor pasa por nullif: el JSON nunca lleva NaN ni infinitos (Plan 5, decisión 8).
CREATE OR REPLACE TABLE ind_valor AS
WITH m AS (
    SELECT m.*, a.personas AS personas_anterior
    FROM ind_mes AS m
    LEFT JOIN ind_mes AS a ON a.periodo = m.periodo_anterior
)
SELECT periodo, 'IND-01' AS id, personas::DOUBLE AS valor FROM m
UNION ALL
SELECT periodo, 'IND-02', entradas::DOUBLE FROM m
UNION ALL
SELECT periodo, 'IND-03', salidas::DOUBLE FROM m
UNION ALL
-- IND-04: IND-03 / promedio de IND-01 del mes y del anterior; NULL si el anterior no está en los datos.
SELECT periodo, 'IND-04', salidas / nullif((personas + personas_anterior) / 2.0, 0) FROM m
UNION ALL
-- IND-05: días de ausencia / (IND-01 × días calendario).
SELECT periodo, 'IND-05', dias_ausencia / nullif(personas * dias, 0) FROM m
UNION ALL
-- IND-06: horas extra / IND-01.
SELECT periodo, 'IND-06', horas_extra / nullif(personas, 0) FROM m;

-- Desgloses: IND-03 por motivo (personas), IND-05 por motivo (días), IND-06 por tipo (horas totales).
CREATE OR REPLACE TABLE ind_desglose AS
SELECT t.periodo, 'IND-03' AS id, coalesce(p.motivo_salida, 'Sin motivo') AS categoria,
       count(DISTINCT p.numero_documento)::DOUBLE AS valor, 0 AS orden
FROM ind_periodos AS t
JOIN planilla_f AS p ON p.periodo = t.periodo AND p.fecha_salida BETWEEN t.inicio AND t.fin
GROUP BY t.periodo, coalesce(p.motivo_salida, 'Sin motivo')
UNION ALL
SELECT t.periodo, 'IND-05', c.categoria,
       coalesce(sum(CASE c.categoria
           WHEN 'dias_descanso_medico' THEN p.dias_descanso_medico
           WHEN 'dias_licencia_con_goce' THEN p.dias_licencia_con_goce
           WHEN 'dias_licencia_sin_goce' THEN p.dias_licencia_sin_goce
           WHEN 'dias_licencia_paternidad' THEN p.dias_licencia_paternidad
           WHEN 'dias_subsidio_enfermedad' THEN p.dias_subsidio_enfermedad
           WHEN 'dias_subsidio_maternidad' THEN p.dias_subsidio_maternidad
           WHEN 'dias_subsidio_otros' THEN p.dias_subsidio_otros
       END), 0)::DOUBLE,
       c.orden
FROM ind_periodos AS t
CROSS JOIN (VALUES
    ('dias_descanso_medico', 1), ('dias_licencia_con_goce', 2), ('dias_licencia_sin_goce', 3),
    ('dias_licencia_paternidad', 4), ('dias_subsidio_enfermedad', 5), ('dias_subsidio_maternidad', 6),
    ('dias_subsidio_otros', 7)
) AS c(categoria, orden)
LEFT JOIN planilla_f AS p ON p.periodo = t.periodo
GROUP BY t.periodo, c.categoria, c.orden
UNION ALL
SELECT t.periodo, 'IND-06', c.categoria,
       coalesce(sum(CASE c.categoria
           WHEN 'horas_extra_25' THEN p.horas_extra_25
           WHEN 'horas_extra_35' THEN p.horas_extra_35
           WHEN 'horas_extra_50' THEN p.horas_extra_50
           WHEN 'horas_extra_100' THEN p.horas_extra_100
       END), 0)::DOUBLE,
       c.orden
FROM ind_periodos AS t
CROSS JOIN (VALUES
    ('horas_extra_25', 1), ('horas_extra_35', 2), ('horas_extra_50', 3), ('horas_extra_100', 4)
) AS c(categoria, orden)
LEFT JOIN planilla_f AS p ON p.periodo = t.periodo
GROUP BY t.periodo, c.categoria, c.orden;
