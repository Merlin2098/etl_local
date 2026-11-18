// Generado desde el esquema de Rust: no editar. Regenerar con `npm run tipos`.

/**
 * Comparación con el mes anterior, en palabras (nunca solo el color, §4).
 *
 * This interface was referenced by `TiposApi`'s JSON-Schema
 * via the `definition` "Comparacion".
 */
export type Comparacion = "mejor" | "peor" | "igual";
/**
 * Detalle nominal disponible (TR-D18).
 *
 * This interface was referenced by `TiposApi`'s JSON-Schema
 * via the `definition` "Detalle".
 */
export type Detalle = "alertas" | "sin-base";
/**
 * Fecha extraída del nombre del archivo (TR-D3).
 *
 * This interface was referenced by `TiposApi`'s JSON-Schema
 * via the `definition` "FechaNombre".
 */
export type FechaNombre =
  | {
      periodo: string;
    }
  | {
      fecha_corte: string;
    };
/**
 * Sentido de "mejor" de un indicador (indicadores-tablero §4).
 *
 * This interface was referenced by `TiposApi`'s JSON-Schema
 * via the `definition` "MejorSi".
 */
export type MejorSi = "baja" | "sube" | "neutro";

export interface TiposApi {}
/**
 * This interface was referenced by `TiposApi`'s JSON-Schema
 * via the `definition` "Check".
 */
export interface Check {
  id: string;
  mensaje: string;
  valor?: number | null;
}
/**
 * Cuerpo de todo error de la API: `{ "codigo", "mensaje", "opciones"? }` (ADR-0004 §3).
 *
 * This interface was referenced by `TiposApi`'s JSON-Schema
 * via the `definition` "ErrorApi".
 */
export interface ErrorApi {
  codigo: string;
  mensaje: string;
  /**
   * Valores válidos, cuando el error es un filtro inválido.
   */
  opciones?: string[] | null;
}
/**
 * "Datos usados" (TR-D16), con el aviso de periodos mezclados (IND-C2).
 *
 * This interface was referenced by `TiposApi`'s JSON-Schema
 * via the `definition` "Estado".
 */
export interface Estado {
  aviso_periodos?: string | null;
  /**
   * Número de avisos de la ejecución (lectura y controles).
   */
  avisos: number;
  fecha_corte?: string | null;
  /**
   * Archivos usados, con la fecha leída de su nombre (TR-D3).
   */
  fuentes: Fuente[];
  marca: string;
  periodos: string[];
  periodos_mezclados: boolean;
  version_motor: string;
}
/**
 * This interface was referenced by `TiposApi`'s JSON-Schema
 * via the `definition` "Fuente".
 */
export interface Fuente {
  archivo: string;
  fecha_nombre?: FechaNombre | null;
  fuente: string;
}
/**
 * Filtros globales del tablero. Un campo ausente significa "todos" (el periodo: el último).
 *
 * This interface was referenced by `TiposApi`'s JSON-Schema
 * via the `definition` "Filtros".
 */
export interface Filtros {
  gerencia?: string | null;
  periodo?: string | null;
  sede?: string | null;
  tipo_trabajador?: string | null;
}
/**
 * This interface was referenced by `TiposApi`'s JSON-Schema
 * via the `definition` "Indicador".
 */
export interface Indicador {
  anterior?: number | null;
  comparacion?: Comparacion | null;
  desglose: Parte[];
  id: string;
  mejor_si: MejorSi;
  meta?: number | null;
  nombre: string;
  nota?: string | null;
  origen: Origen;
  /**
   * Plan 7: personas por gerencia (IND-01, IND-07, IND-08), de mayor a menor; vacío en el resto.
   */
  por_gerencia: Parte[];
  pregunta: string;
  /**
   * `personas`, `fraccion` (0.0073 = 0,73 %) u `horas`.
   */
  unidad: string;
  valor?: number | null;
  variacion?: number | null;
}
/**
 * Una parte del desglose de un indicador.
 *
 * This interface was referenced by `TiposApi`'s JSON-Schema
 * via the `definition` "Parte".
 */
export interface Parte {
  categoria: string;
  etiqueta: string;
  valor: number;
}
/**
 * De dónde sale un número (TR-D17).
 *
 * This interface was referenced by `TiposApi`'s JSON-Schema
 * via the `definition` "Origen".
 */
export interface Origen {
  archivos: string[];
  fecha_corte?: string | null;
  fuente: string;
  periodo?: string | null;
  reglas: string[];
}
/**
 * Valores posibles de cada filtro.
 *
 * This interface was referenced by `TiposApi`'s JSON-Schema
 * via the `definition` "OpcionesFiltro".
 */
export interface OpcionesFiltro {
  gerencias: string[];
  periodos: string[];
  sedes: string[];
  tipos_trabajador: string[];
}
/**
 * This interface was referenced by `TiposApi`'s JSON-Schema
 * via the `definition` "Punto".
 */
export interface Punto {
  periodo: string;
  valor?: number | null;
}
/**
 * This interface was referenced by `TiposApi`'s JSON-Schema
 * via the `definition` "RespuestaChecks".
 */
export interface RespuestaChecks {
  checks: Check[];
  periodo?: string | null;
}
/**
 * Detalle nominal (TR-D18): la única respuesta con nombres y documentos.
 *
 * This interface was referenced by `TiposApi`'s JSON-Schema
 * via the `definition` "RespuestaDetalle".
 */
export interface RespuestaDetalle {
  filas: {
    [k: string]: unknown;
  }[];
  periodo?: string | null;
  tipo: Detalle;
}
/**
 * This interface was referenced by `TiposApi`'s JSON-Schema
 * via the `definition` "RespuestaIndicadores".
 */
export interface RespuestaIndicadores {
  fecha_corte?: string | null;
  filtros: Filtros1;
  indicadores: Indicador[];
  periodo?: string | null;
  periodo_anterior?: string | null;
}
/**
 * Filtros aplicados, con el periodo ya resuelto.
 */
export interface Filtros1 {
  gerencia?: string | null;
  periodo?: string | null;
  sede?: string | null;
  tipo_trabajador?: string | null;
}
/**
 * This interface was referenced by `TiposApi`'s JSON-Schema
 * via the `definition` "RespuestaSeries".
 */
export interface RespuestaSeries {
  filtros: Filtros;
  series: Serie[];
}
/**
 * This interface was referenced by `TiposApi`'s JSON-Schema
 * via the `definition` "Serie".
 */
export interface Serie {
  id: string;
  nombre: string;
  puntos: Punto[];
  unidad: string;
}
