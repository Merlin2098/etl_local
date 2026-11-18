// Datos de los gráficos del Resumen. Las opciones de ECharts se arman aquí, sin DOM, para poder probarlas.
import type { EChartsCoreOption } from "echarts/core";
import { mesCorto, nombreMes, numero } from "../lib/formato";
import type { Indicador, RespuestaSeries } from "../lib/api/tipos.gen";
import { COLORES, TEMA } from "../lib/graficos/tema";

/** Los últimos 6 periodos con datos de IND-02 o IND-03, con sus valores (también para «Ver como tabla»). */
export function filasMovimientos(r: RespuestaSeries): { periodo: string; entraron: number; salieron: number }[] {
  const serie = (id: string) => r.series.find((s) => s.id === id)?.puntos ?? [];
  const ent = serie("IND-02");
  const sal = serie("IND-03");
  const periodos = [...new Set([...ent, ...sal].filter((p) => p.valor != null).map((p) => p.periodo))].sort().slice(-6);
  const valor = (puntos: typeof ent, p: string) => puntos.find((x) => x.periodo === p)?.valor ?? 0;
  return periodos.map((p) => ({ periodo: p, entraron: valor(ent, p), salieron: valor(sal, p) }));
}

/** «Entradas y salidas de personal por mes» (ECharts, barras agrupadas); `null` sin datos. */
export function opcionMovimientos(r: RespuestaSeries): EChartsCoreOption | null {
  const filas = filasMovimientos(r);
  // Un filtro sin personas devuelve ceros (los periodos salen de la planilla completa): también es «sin datos».
  if (filas.every((f) => f.entraron === 0 && f.salieron === 0)) return null;
  return {
    aria: { enabled: true },
    textStyle: { fontFamily: TEMA.fuente, color: TEMA.texto },
    grid: { left: 40, right: 16, top: 16, bottom: 32 },
    tooltip: {
      trigger: "axis",
      formatter: (params: { dataIndex: number }[]) => {
        const f = filas[params[0]?.dataIndex ?? 0];
        if (!f) return "";
        const dif = f.entraron - f.salieron;
        return [
          `<strong>${nombreMes(f.periodo)}</strong>`,
          `Entraron: ${numero(f.entraron)}`,
          `Salieron: ${numero(f.salieron)}`,
          `Variación: ${dif > 0 ? "+" : dif < 0 ? "−" : ""}${numero(Math.abs(dif))} personas`,
        ].join("<br>");
      },
    },
    xAxis: {
      type: "category",
      data: filas.map((f) => mesCorto(f.periodo)),
      axisLine: { lineStyle: { color: TEMA.borde } },
      axisLabel: { color: TEMA.texto2 },
    },
    yAxis: {
      type: "value",
      minInterval: 1,
      splitLine: { lineStyle: { color: TEMA.rejilla } },
      axisLabel: { color: TEMA.texto2 },
    },
    series: [
      { name: "Entraron", type: "bar", color: COLORES.entraron, data: filas.map((f) => f.entraron), barGap: "10%" },
      { name: "Salieron", type: "bar", color: COLORES.salieron, data: filas.map((f) => f.salieron) },
    ],
  };
}

/** «Personas en planilla por gerencia», de mayor a menor; `pct` sobre la suma de las barras. */
export function filasGerencia(i: Indicador): { gerencia: string; valor: number; pct: number }[] {
  const total = i.por_gerencia.reduce((s, p) => s + p.valor, 0);
  return [...i.por_gerencia]
    .sort((a, b) => b.valor - a.valor || a.etiqueta.localeCompare(b.etiqueta, "es"))
    .map((p) => ({ gerencia: p.etiqueta, valor: p.valor, pct: total > 0 ? (p.valor / total) * 100 : 0 }));
}

/** «Días de ausencia por motivo»: sin los motivos en 0, de mayor a menor. */
export function filasAusencias(i: Indicador): { motivo: string; dias: number; pct: number }[] {
  const partes = i.desglose.filter((p) => p.valor > 0);
  const total = partes.reduce((s, p) => s + p.valor, 0);
  return [...partes]
    .sort((a, b) => b.valor - a.valor)
    .map((p) => ({ motivo: p.etiqueta, dias: p.valor, pct: total > 0 ? (p.valor / total) * 100 : 0 }));
}
