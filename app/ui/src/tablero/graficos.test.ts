import { describe, expect, test } from "vitest";
import { datosDePrueba, indicador, parte } from "./prueba";
import { filasAusencias, filasGerencia, opcionMovimientos } from "./graficos";

type Serie = { name: string; color: string; data: number[] };

describe("gráficos del Resumen", () => {
  test("movimientos: los últimos 6 meses, Entraron y Salieron con sus colores", () => {
    const o = opcionMovimientos(datosDePrueba().series)!;
    const eje = (o.xAxis as { data: string[] }).data;
    expect(eje).toEqual(["jun", "jul", "ago", "sep", "oct", "nov"]);
    const series = o.series as Serie[];
    expect(series.map((s) => s.name)).toEqual(["Entraron", "Salieron"]);
    expect(series.map((s) => s.color)).toEqual(["#2B6CB0", "#C2620F"]);
    expect(series[0]!.data).toEqual([12, 13, 14, 15, 16, 17]);
    expect((o.aria as { enabled: boolean }).enabled).toBe(true);
  });

  test("movimientos con un solo mes: una barra por serie (foco 2)", () => {
    const d = datosDePrueba();
    d.series.series = d.series.series.map((s) => ({ ...s, puntos: s.puntos.slice(-1) }));
    const o = opcionMovimientos(d.series)!;
    expect((o.xAxis as { data: string[] }).data).toEqual(["nov"]);
    expect((o.series as Serie[])[1]!.data).toHaveLength(1);
  });

  test("movimientos sin datos devuelve null (foco 3)", () => {
    const d = datosDePrueba();
    d.series.series = d.series.series.map((s) => ({ ...s, puntos: s.puntos.map((p) => ({ ...p, valor: null })) }));
    expect(opcionMovimientos(d.series)).toBeNull();
    expect(opcionMovimientos({ filtros: {}, series: [] })).toBeNull();
  });

  test("un filtro sin personas: la API responde ceros, no null, y también es «sin datos» (revisión final)", () => {
    const d = datosDePrueba();
    d.series.series = d.series.series.map((s) => ({ ...s, puntos: s.puntos.map((p) => ({ ...p, valor: 0 })) }));
    expect(opcionMovimientos(d.series)).toBeNull();
  });

  test("por gerencia: de mayor a menor con su porcentaje", () => {
    const filas = filasGerencia(indicador("IND-01", { por_gerencia: [parte("B", 25), parte("A", 75)] }));
    expect(filas).toEqual([
      { gerencia: "A", valor: 75, pct: 75 },
      { gerencia: "B", valor: 25, pct: 25 },
    ]);
    expect(filasGerencia(indicador("IND-01"))).toEqual([]);
  });

  test("ausencias: sin ceros, de mayor a menor, porcentajes que suman 100", () => {
    const i = datosDePrueba().indicadores.indicadores.find((x) => x.id === "IND-05")!;
    const filas = filasAusencias(i);
    expect(filas.map((f) => f.motivo)).toEqual([
      "Descanso médico",
      "Licencia con goce de sueldo",
      "Subsidio por enfermedad",
      "Licencia sin goce de sueldo",
      "Subsidio por maternidad",
    ]);
    expect(filas[0]!.dias).toBe(412);
    expect(Math.abs(filas.reduce((s, f) => s + f.pct, 0) - 100)).toBeLessThan(0.1);
    expect(filasAusencias(indicador("IND-05", { desglose: [parte("x", 0)] }))).toEqual([]);
  });
});
