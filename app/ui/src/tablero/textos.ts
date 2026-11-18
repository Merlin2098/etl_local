// Textos del tablero (Plan 7, sección «Textos»). El frontend no calcula indicadores: solo da formato.
import { fechaCorta, mesMinuscula, numero } from "../lib/formato";
import { aCsv } from "../lib/csv";
import type { Filtros, Fuente, Indicador, Parte } from "../lib/api/tipos.gen";
import type { DatosResumen } from "./estado.svelte";

/** Fracción a porcentaje con un decimal: `0.0186` → «1,9 %»; sin valor → «—». */
export function porcentaje(f: number | null | undefined): string {
  if (f == null || !Number.isFinite(f)) return "—";
  return `${(f * 100).toFixed(1).replace(".", ",")} %`;
}

/** Cifra según la unidad del indicador; sin valor → «—». */
export function cifra(v: number | null | undefined, unidad: string): string {
  if (v == null || !Number.isFinite(v)) return "—";
  if (unidad === "fraccion") return porcentaje(v);
  if (unidad === "horas") return `${numero(Math.round(v * 10) / 10)} h`;
  return numero(Math.round(v));
}

/** «4 menos que en octubre», «Empeoró: era 0,9 % en octubre»…; `null` sin mes anterior. */
export function textoComparacion(i: Indicador, mesAnterior: string | null | undefined): string | null {
  if (!mesAnterior || i.valor == null || i.anterior == null) return null;
  const mes = mesMinuscula(mesAnterior);
  if (i.mejor_si === "neutro") {
    const dif = Math.round(i.valor - i.anterior);
    if (dif === 0) return `Igual que en ${mes}`;
    return `${numero(Math.abs(dif))} ${dif < 0 ? "menos" : "más"} que en ${mes}`;
  }
  if (i.comparacion == null) return null;
  if (i.comparacion === "igual") return `Igual que en ${mes}`;
  const palabra = i.comparacion === "mejor" ? "Mejoró" : "Empeoró";
  return `${palabra}: era ${cifra(i.anterior, i.unidad)} en ${mes}`;
}

export function textoMeta(i: Indicador): string {
  return i.meta == null ? "Sin meta definida" : `Meta: ${cifra(i.meta, i.unidad)}`;
}

const NOMBRE_FUENTE: Record<string, string> = {
  planilla: "Planilla",
  base_personal: "Base de personal",
  licencias: "Licencias",
};

/** Una entrada de «Datos usados». La planilla muestra el periodo elegido. */
export function textoFuente(f: Fuente, periodo: string | null | undefined): string {
  const nombre = NOMBRE_FUENTE[f.fuente] ?? f.fuente;
  if (f.fuente === "planilla" && periodo) return `${nombre}: ${mesMinuscula(periodo)} ${periodo.slice(0, 4)}`;
  const fn = f.fecha_nombre;
  if (fn && "periodo" in fn) return `${nombre}: ${mesMinuscula(fn.periodo)} ${fn.periodo.slice(0, 4)}`;
  if (fn && "fecha_corte" in fn) return `${nombre}: ${fechaCorta(fn.fecha_corte)}`;
  return `${nombre}: incluidas`;
}

/** Fuentes que alimentan el tablero; los reportes del molde (Plan 8) no entran en sus indicadores. */
const FUENTES_DEL_TABLERO = new Set(["planilla", "licencias", "base_personal"]);

/** Las fuentes de «Datos usados», una por tipo (la planilla trae un archivo por mes). */
export function fuentesUnicas(fuentes: Fuente[]): Fuente[] {
  const vistas = new Map<string, Fuente>();
  for (const f of fuentes) if (FUENTES_DEL_TABLERO.has(f.fuente)) vistas.set(f.fuente, f);
  return [...vistas.values()];
}

const mayuscula = (t: string) => t.charAt(0).toUpperCase() + t.slice(1);

/** Días del mes de un periodo `AAAA-MM`. */
export function diasDelMes(periodo: string): number {
  const [anio = 0, mes = 1] = periodo.split("-").map(Number);
  return new Date(Date.UTC(anio, mes, 0)).getUTCDate();
}

export function buscar(d: DatosResumen, id: string): Indicador | undefined {
  return d.indicadores.indicadores.find((i) => i.id === id);
}

/** Segunda línea de cada tarjeta (sección «Textos»). Sin datos, la nota de la API o `null`. */
export function lineaTarjeta(id: string, d: DatosResumen): string | null {
  const i = buscar(d, id);
  if (!i) return null;
  if (i.valor == null && i.nota) return i.nota;
  switch (id) {
    case "IND-01": {
      const ent = buscar(d, "IND-02")?.valor;
      const sal = buscar(d, "IND-03")?.valor;
      const movimientos = ent != null && sal != null ? `${numero(ent)} entraron, ${numero(sal)} salieron` : null;
      const ant = d.indicadores.periodo_anterior;
      if (!ant || i.anterior == null) return movimientos;
      const base = `${mayuscula(mesMinuscula(ant))}: ${numero(i.anterior)}`;
      return movimientos ? `${base} · ${movimientos}` : base;
    }
    case "IND-04":
      return i.nota ?? textoMeta(i);
    case "IND-05": {
      const personas = buscar(d, "IND-01")?.valor;
      const periodo = d.indicadores.periodo;
      if (i.valor == null || personas == null || !periodo) return null;
      const perdidos = i.desglose.reduce((s, p) => s + p.valor, 0);
      return `${numero(Math.round(perdidos))} días perdidos de ${numero(personas * diasDelMes(periodo))} programados`;
    }
    case "IND-07":
      return "Superan el plazo legal en 6 meses o menos";
    default:
      return null;
  }
}

/** «Más casos: Operaciones (9), Mantenimiento (5), Comercial (2), otras (2)»; `null` sin datos. */
export function masCasos(partes: Parte[]): string | null {
  if (partes.length === 0) return null;
  const primeras = partes.slice(0, 3).map((p) => `${p.etiqueta} (${numero(p.valor)})`);
  const resto = partes.slice(3).reduce((s, p) => s + p.valor, 0);
  if (resto > 0) primeras.push(`otras (${numero(resto)})`);
  return `Más casos: ${primeras.join(", ")}`;
}

const suma = (i: Indicador, categorias: string[]) =>
  i.desglose.filter((p) => categorias.includes(p.categoria)).reduce((s, p) => s + p.valor, 0);

/** Un bloque de «Riesgos a atender» (IND-07 o IND-08). */
export function textoRiesgo(i: Indicador): { cifra: string; titulo: string; detalle: string; masCasos: string | null } {
  const v = Math.round(i.valor ?? 0);
  if (i.id === "IND-07") {
    if (v === 0)
      return { cifra: "0", titulo: "Ningún contrato temporal supera su plazo en los próximos 6 meses.", detalle: "", masCasos: null };
    const obra = suma(i, ["alerta_contrato_obra"]);
    const incremento = suma(i, ["alerta_contrato_incremento"]);
    return {
      cifra: numero(v),
      titulo:
        v === 1
          ? "contrato temporal superará el plazo legal en los próximos 6 meses"
          : "contratos temporales superarán el plazo legal en los próximos 6 meses",
      detalle: `${numero(obra)} por obra o servicio (máx. 5 años) · ${numero(incremento)} por incremento de actividad (máx. 3 años). Si se pasan del plazo, pueden considerarse contratos indefinidos.`,
      masCasos: masCasos(i.por_gerencia),
    };
  }
  if (v === 0)
    return { cifra: "0", titulo: "Nadie llega a la edad de jubilación este año ni el próximo.", detalle: "", masCasos: null };
  const de65 = suma(i, ["cumple_65_este_anio", "cumple_65_proximo_anio"]);
  const de70 = suma(i, ["cumple_70_este_anio", "cumple_70_proximo_anio"]);
  return {
    cifra: numero(v),
    titulo:
      v === 1
        ? "persona llega a la edad de jubilación este año o el próximo"
        : "personas llegan a la edad de jubilación este año o el próximo",
    detalle: `${numero(de65)} cumplen 65 años · ${numero(de70)} cumplen 70 años. Conviene planificar reemplazos y traspaso de conocimiento.`,
    masCasos: masCasos(i.por_gerencia),
  };
}

const UNIDAD: Record<string, string> = { personas: "personas", fraccion: "%", horas: "horas" };

/** «Datos en Excel» (decisión 4): los indicadores visibles, sin nombres ni documentos (TR-D18). */
export function csvIndicadores(d: DatosResumen, f: Filtros): string {
  const filas = d.indicadores.indicadores.map((i) => [
    i.nombre,
    i.valor == null ? "" : cifra(i.valor, i.unidad),
    i.anterior == null ? "" : cifra(i.anterior, i.unidad),
    UNIDAD[i.unidad] ?? i.unidad,
  ]);
  // Primera línea: qué filtros produjeron estas cifras (revisión final: un subconjunto no debe parecer el total).
  return aCsv([`Filtros: ${lineaFiltros(f, d.indicadores.periodo)}`], [["Indicador", "Valor", "Mes anterior", "Unidad"], ...filas]);
}

/** «Periodo: noviembre 2025 · Gerencia: Todas · Sede: Todas · Tipo de trabajador: Todos» (impresión). */
export function lineaFiltros(f: Filtros, periodo: string | null | undefined): string {
  const p = periodo ? `${mesMinuscula(periodo)} ${periodo.slice(0, 4)}` : "—";
  return [
    `Periodo: ${p}`,
    `Gerencia: ${f.gerencia || "Todas"}`,
    `Sede: ${f.sede || "Todas"}`,
    `Tipo de trabajador: ${f.tipo_trabajador || "Todos"}`,
  ].join(" · ");
}
