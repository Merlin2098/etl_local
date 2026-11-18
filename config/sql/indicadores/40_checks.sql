-- Checks de calidad que se muestran al usuario (indicadores-tablero §5). IND-C2 sale de ejecucion.json
-- (Plan 5, decisión 7) y no está aquí.
CREATE OR REPLACE TABLE ind_check AS
-- IND-C1: personas de la planilla del mes sin registro en la base de personal. Sin filtros de dimensión:
-- son justamente las que desaparecen al filtrar por gerencia o sede.
SELECT
    t.periodo,
    'IND-C1' AS id,
    (
        SELECT count(DISTINCT p.numero_documento)
        FROM planilla AS p
        WHERE p.periodo = t.periodo
          AND NOT EXISTS (
              SELECT 1 FROM base_personal_empleados AS b WHERE b.numero_documento = p.numero_documento
          )
    )::DOUBLE AS valor,
    NULL::DOUBLE AS cambio_personas,
    NULL::DOUBLE AS entradas_menos_salidas
FROM ind_periodos AS t
UNION ALL
-- IND-C3: (IND-01(mes) − IND-01(ant)) − (IND-02 − IND-03). NULL sin mes anterior en los datos.
SELECT
    m.periodo,
    'IND-C3',
    ((m.personas - a.personas) - (m.entradas - m.salidas))::DOUBLE,
    (m.personas - a.personas)::DOUBLE,
    (m.entradas - m.salidas)::DOUBLE
FROM ind_mes AS m
LEFT JOIN ind_mes AS a ON a.periodo = m.periodo_anterior;
