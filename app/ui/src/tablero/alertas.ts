// Pestaña «Alertas (solo RR. HH.)»: lo único del tablero con nombres y documentos (TR-D18).
import { aCsv } from "../lib/csv";

type Fila = Record<string, unknown>;

const texto = (v: unknown): string => (v == null ? "" : String(v));

/** `"2026-03-31"` → `"31/03/2026"`; vacío si no hay fecha. */
function fecha(v: unknown): string {
  const t = texto(v);
  const m = /^(\d{4})-(\d{2})-(\d{2})/.exec(t);
  return m ? `${m[3]}/${m[2]}/${m[1]}` : t;
}

const ALERTAS: [string, string][] = [
  ["alerta_contrato_obra", "Contrato por obra o servicio por superar su plazo"],
  ["alerta_contrato_incremento", "Contrato por incremento de actividad por superar su plazo"],
  ["cumple_65_este_anio", "Cumple 65 este año"],
  ["cumple_65_proximo_anio", "Cumple 65 el próximo año"],
  ["cumple_70_este_anio", "Cumple 70 este año"],
  ["cumple_70_proximo_anio", "Cumple 70 el próximo año"],
];

export function textoAlertas(fila: Fila): string[] {
  return ALERTAS.filter(([clave]) => fila[clave] === true).map(([, t]) => t);
}

export const COLUMNAS_ALERTAS = [
  "Documento",
  "Nombre",
  "Gerencia",
  "Sede",
  "Tipo de trabajador",
  "Alertas",
  "Fin de contrato",
  "Nacimiento",
];

export function celdasAlerta(f: Fila): string[] {
  return [
    texto(f.numero_documento),
    texto(f.nombre_completo),
    texto(f.gerencia),
    texto(f.sede),
    texto(f.tipo_trabajador),
    textoAlertas(f).join(" · "),
    fecha(f.fecha_fin_contrato),
    fecha(f.fecha_nacimiento),
  ];
}

export const COLUMNAS_SIN_BASE = ["Documento", "Apellidos y nombres", "Área", "Centro de costo"];

export function celdasSinBase(f: Fila): string[] {
  const apellidos = [texto(f.apellido_paterno), texto(f.apellido_materno)].filter(Boolean).join(" ");
  const nombre = [apellidos, texto(f.nombres)].filter(Boolean).join(", ");
  return [texto(f.numero_documento), nombre, texto(f.area), texto(f.centro_costo)];
}

/** CSV de una tabla de Alertas. El documento (primera columna) va como `="01234567"` para que Excel no le quite
 *  los ceros a la izquierda (revisión final). */
export function csvDetalle(columnas: string[], filas: string[][]): string {
  return aCsv(
    columnas,
    filas.map(([doc = "", ...resto]) => [doc ? `="${doc}"` : "", ...resto]),
  );
}
