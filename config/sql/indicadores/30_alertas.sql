-- IND-07 e IND-08 (indicadores-tablero §2): conteos sobre la última base de personal, con los filtros de
-- dimensión. No responden al periodo (periodo NULL). Las columnas de alerta nunca son nulas (bd.md BD-FLG).

-- IND-07: contratos por superar su plazo (BD-FLG-08, BD-FLG-09; modalidades normalizadas, TR-D24).
-- IND-08: llegan a 65 o 70 este año o el próximo (BD-FLG-05…07; por año de la fecha de corte, TR-D15).
INSERT INTO ind_valor
SELECT NULL, 'IND-07', count(*) FILTER (WHERE alerta_contrato_plazo)::DOUBLE FROM empleados_f
UNION ALL
SELECT NULL, 'IND-08', count(*) FILTER (WHERE alerta_jubilacion)::DOUBLE FROM empleados_f;

INSERT INTO ind_desglose
SELECT NULL, 'IND-07', c.categoria,
       count(*) FILTER (WHERE CASE c.categoria
           WHEN 'alerta_contrato_obra' THEN e.alerta_contrato_obra
           WHEN 'alerta_contrato_incremento' THEN e.alerta_contrato_incremento
       END)::DOUBLE,
       c.orden
FROM (VALUES ('alerta_contrato_obra', 1), ('alerta_contrato_incremento', 2)) AS c(categoria, orden)
LEFT JOIN empleados_f AS e ON true
GROUP BY c.categoria, c.orden
UNION ALL
SELECT NULL, 'IND-08', c.categoria,
       count(*) FILTER (WHERE CASE c.categoria
           WHEN 'cumple_65_este_anio' THEN e.cumple_65_este_anio
           WHEN 'cumple_65_proximo_anio' THEN e.cumple_65_proximo_anio
           WHEN 'cumple_70_este_anio' THEN e.cumple_70_este_anio
           WHEN 'cumple_70_proximo_anio' THEN e.cumple_70_proximo_anio
       END)::DOUBLE,
       c.orden
FROM (VALUES
    ('cumple_65_este_anio', 1), ('cumple_65_proximo_anio', 2),
    ('cumple_70_este_anio', 3), ('cumple_70_proximo_anio', 4)
) AS c(categoria, orden)
LEFT JOIN empleados_f AS e ON true
GROUP BY c.categoria, c.orden;
