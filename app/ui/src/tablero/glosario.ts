// «Cómo leer este tablero» (pantalla 7, texto del mock v2) y el «Qué cuenta» de «¿De dónde sale?» (TR-D17).
import { numero } from "../lib/formato";
import type { DatosResumen } from "./estado.svelte";
import { buscar, diasDelMes, porcentaje } from "./textos";

export interface Explicacion {
  nombre: string;
  pregunta: string;
  calculo: string;
  fuente: string;
  reglas: string[];
  /** No existía en el reporte anterior (Power BI del legacy). */
  nuevo: boolean;
}

const ENTRADAS_SALIDAS: Explicacion = {
  nombre: "Entradas / Salidas",
  pregunta: "¿Cuántas personas entraron y cuántas se fueron en el mes?",
  calculo:
    "Entradas: personas cuya fecha de entrada cae dentro del mes. Salidas: personas cuya fecha de salida cae dentro del mes.",
  fuente: "Planilla",
  reglas: ["IND-02", "IND-03"],
  nuevo: true,
};

export const GLOSARIO: Record<string, Explicacion> = {
  "IND-01": {
    nombre: "Personas en planilla",
    pregunta: "¿A cuántas personas pagamos este mes?",
    calculo: "Personas distintas en la planilla del mes, contadas por número de documento.",
    fuente: "Planilla",
    reglas: ["IND-01"],
    nuevo: true,
  },
  "IND-02": ENTRADAS_SALIDAS,
  "IND-03": ENTRADAS_SALIDAS,
  "IND-04": {
    nombre: "Rotación del mes",
    pregunta: "¿Qué parte del personal perdimos este mes?",
    calculo: "Salidas del mes ÷ promedio de personas en planilla de este mes y el anterior.",
    fuente: "Planilla",
    reglas: ["IND-04"],
    nuevo: true,
  },
  "IND-05": {
    nombre: "Ausentismo",
    pregunta: "¿Qué parte del tiempo programado se perdió por ausencias?",
    calculo:
      "Días de ausencia (descansos médicos, licencias y subsidios) ÷ días programados (personas × días del mes).",
    fuente: "Planilla",
    reglas: ["IND-05"],
    nuevo: true,
  },
  "IND-07": {
    nombre: "Contratos por superar su plazo",
    pregunta: "¿Qué contratos temporales están por superar su plazo máximo?",
    calculo:
      "Por obra o servicio: 4 años y medio o más de servicio (máximo 5). Por incremento de actividad: 2 años y medio o más (máximo 3).",
    fuente: "Base de personal",
    reglas: ["BD-FLG-08", "BD-FLG-09"],
    nuevo: false,
  },
  "IND-08": {
    nombre: "Edad de jubilación",
    pregunta: "¿Quién llega a los 65 o 70 años este año o el próximo?",
    calculo: "Según el año de nacimiento: cumple 65 o 70 durante el año de la fecha de corte o el siguiente.",
    fuente: "Base de personal",
    reglas: ["BD-FLG-05", "BD-FLG-06", "BD-FLG-07"],
    nuevo: false,
  },
};

/** Entradas de la lista de indicadores de «Cómo leer», sin repetir «Entradas / Salidas». */
export const ORDEN_GLOSARIO = ["IND-01", "IND-02", "IND-04", "IND-05", "IND-07", "IND-08"];

/** «Este mes: 23 ÷ 1.241 = 1,9 %», con los datos vigentes; `null` sin datos (reemplaza los ejemplos fijos del mock). */
export function ejemplo(id: "IND-04" | "IND-05", d: DatosResumen | null): string | null {
  if (!d) return null;
  const i = buscar(d, id);
  const personas = buscar(d, "IND-01");
  if (i?.valor == null || personas?.valor == null) return null;
  if (id === "IND-04") {
    const salidas = buscar(d, "IND-03")?.valor;
    if (salidas == null || personas.anterior == null) return null;
    const promedio = Math.round((personas.valor + personas.anterior) / 2);
    return `Este mes: ${numero(salidas)} ÷ ${numero(promedio)} = ${porcentaje(i.valor)}`;
  }
  const periodo = d.indicadores.periodo;
  if (!periodo) return null;
  const dias = i.desglose.reduce((s, p) => s + p.valor, 0);
  return `Este mes: ${numero(Math.round(dias))} ÷ ${numero(personas.valor * diasDelMes(periodo))} = ${porcentaje(i.valor)}`;
}

export interface Alcance {
  grafico: string;
  periodo: string;
  gerencia: boolean;
  sede: boolean;
  tipo: boolean;
}

/** «Qué filtros afectan a cada gráfico» (pantalla 7). */
export const ALCANCE_FILTROS: Alcance[] = [
  { grafico: "Indicadores de arriba", periodo: "Sí", gerencia: true, sede: true, tipo: true },
  { grafico: "Riesgos a atender", periodo: "No (última base de personal)", gerencia: true, sede: true, tipo: true },
  { grafico: "Entradas y salidas de personal por mes", periodo: "No (últimos 6 meses)", gerencia: true, sede: true, tipo: true },
  { grafico: "Personas en planilla por gerencia", periodo: "Sí", gerencia: true, sede: true, tipo: true },
  { grafico: "Días de ausencia por motivo", periodo: "Sí", gerencia: true, sede: true, tipo: true },
];

/** El cálculo con su ejemplo vigente, separados por un espacio. */
export function calculoConEjemplo(calculo: string, ej: string | null): string {
  return ej ? `${calculo} ${ej}.` : calculo;
}
