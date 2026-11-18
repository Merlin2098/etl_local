// Generado desde el esquema de Rust: no editar. Regenerar con `npm run tipos`.

/**
 * Fecha extraída del nombre del archivo (TR-D3).
 *
 * This interface was referenced by `TiposApp`'s JSON-Schema
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
 * Paso del proceso que la app muestra en lenguaje de oficina (Plan 6, pantalla «Procesando»).
 *
 * This interface was referenced by `TiposApp`'s JSON-Schema
 * via the `definition` "Paso".
 */
export type Paso = ("leer" | "licencias" | "planilla" | "base_personal" | "controles" | "guardar") | "otros_reportes";
/**
 * Estado de una etapa (TR-D6).
 *
 * This interface was referenced by `TiposApp`'s JSON-Schema
 * via the `definition` "EstadoEtapa".
 */
export type EstadoEtapa = "ok" | "aviso" | "fallo" | "omitida";
/**
 * This interface was referenced by `TiposApp`'s JSON-Schema
 * via the `definition` "Severidad".
 */
export type Severidad = "error" | "aviso";
/**
 * This interface was referenced by `TiposApp`'s JSON-Schema
 * via the `definition` "Genero".
 */
export type Genero = "femenino" | "masculino";
/**
 * This interface was referenced by `TiposApp`'s JSON-Schema
 * via the `definition` "Vigencia".
 */
export type Vigencia = "nunca" | "al_dia" | "atrasado";

export interface TiposApp {}
/**
 * This interface was referenced by `TiposApp`'s JSON-Schema
 * via the `definition` "ArchivoElegido".
 */
export interface ArchivoElegido {
  bytes: number;
  /**
   * Las dos últimas carpetas, como «Documentos › Planillas».
   */
  carpeta: string;
  fecha_nombre?: FechaNombre | null;
  /**
   * Fecha de modificación (`AAAA-MM-DD`).
   */
  modificado?: string | null;
  nombre: string;
  ruta: string;
}
/**
 * Aviso de avance: `paso` empieza; `indice` es su posición en `pasos(entradas)`.
 *
 * This interface was referenced by `TiposApp`'s JSON-Schema
 * via the `definition` "Avance".
 */
export interface Avance {
  indice: number;
  paso: Paso;
  /**
   * Todos los pasos de esta ejecución, en orden (la app arma la lista con el primer aviso).
   */
  pasos: Paso[];
  total: number;
}
/**
 * This interface was referenced by `TiposApp`'s JSON-Schema
 * via the `definition` "Cifra".
 */
export interface Cifra {
  etiqueta: string;
  valor: string;
}
/**
 * Resumen de una ejecución; se guarda como `ejecucion.json` junto a los Parquet (TR-D16).
 *
 * This interface was referenced by `TiposApp`'s JSON-Schema
 * via the `definition` "Ejecucion".
 */
export interface Ejecucion {
  etapas: Etapa[];
  fecha_corte?: string | null;
  fuentes: Fuente[];
  marca: string;
  periodos: string[];
  /**
   * Avisos de lectura y de los controles.
   */
  problemas: Problema[];
  /**
   * (ruta relativa del Parquet, filas)
   */
  salidas: [unknown, unknown][];
  version_motor: string;
}
/**
 * This interface was referenced by `TiposApp`'s JSON-Schema
 * via the `definition` "Etapa".
 */
export interface Etapa {
  estado: EstadoEtapa;
  filas?: number | null;
  nombre: string;
}
/**
 * This interface was referenced by `TiposApp`'s JSON-Schema
 * via the `definition` "Fuente".
 */
export interface Fuente {
  archivo: string;
  fecha_nombre?: FechaNombre | null;
  fuente: string;
}
/**
 * This interface was referenced by `TiposApp`'s JSON-Schema
 * via the `definition` "Problema".
 */
export interface Problema {
  /**
   * Nombre base del archivo.
   */
  archivo: string;
  /**
   * Celda (`A6`) o ubicación (`fila 6`).
   */
  celda?: string | null;
  /**
   * TR-D13: dónde se encontró lo que se esperaba en `celda` (por ejemplo, la cabecera desplazada a `A7`).
   */
  celda_encontrada?: string | null;
  /**
   * ID de regla de `docs/logica-negocio/` (por ejemplo, `PRE-10`).
   */
  codigo: string;
  /**
   * Detalle técnico para soporte (por ejemplo, el error del lector de Excel).
   */
  detalle?: string | null;
  hoja?: string | null;
  /**
   * Texto en español claro para el usuario.
   */
  mensaje: string;
  severidad: Severidad;
}
/**
 * This interface was referenced by `TiposApp`'s JSON-Schema
 * via the `definition` "ErrorApp".
 */
export interface ErrorApp {
  /**
   * `entradas`, `cancelado`, `archivo_abierto`, `en_curso`, `no_permitido`, `sin_excel` o `interno`.
   */
  codigo: string;
  mensaje: string;
  problemas: Problema[];
}
/**
 * This interface was referenced by `TiposApp`'s JSON-Schema
 * via the `definition` "EstadoReporte".
 */
export interface EstadoReporte {
  reporte: Reporte;
  /**
   * La fecha más reciente entre los archivos guardados de la fuente del reporte.
   */
  ultimo?: FechaNombre | null;
  vigencia: Vigencia;
}
/**
 * Un reporte que el usuario puede actualizar (una tarjeta de la pantalla 1).
 *
 * This interface was referenced by `TiposApp`'s JSON-Schema
 * via the `definition` "Reporte".
 */
export interface Reporte {
  /**
   * Contratos que se pueden agregar a este reporte (por ejemplo, `licencias` en la planilla).
   */
  complementos?: string[];
  descripcion: string;
  /**
   * Nombre del archivo de `actual/excel/` (sin extensión) que abre «Abrir en Excel».
   */
  excel: string;
  /**
   * Contrato principal (`config/fuentes/<fuente>.yaml`).
   */
  fuente: string;
  genero: Genero;
  id: string;
  titulo: string;
  /**
   * Se pueden elegir varios archivos (uno por periodo).
   */
  varios_archivos: boolean;
}
/**
 * This interface was referenced by `TiposApp`'s JSON-Schema
 * via the `definition` "Inicio".
 */
export interface Inicio {
  carpeta_visible: string;
  tarjetas: TarjetaReporte[];
}
/**
 * This interface was referenced by `TiposApp`'s JSON-Schema
 * via the `definition` "TarjetaReporte".
 */
export interface TarjetaReporte {
  duracion_prevista_ms?: number | null;
  estado: EstadoReporte;
}
/**
 * This interface was referenced by `TiposApp`'s JSON-Schema
 * via the `definition` "ProblemaVista".
 */
export interface ProblemaVista {
  problema: Problema;
  recorte?: Recorte | null;
}
/**
 * Un trozo de hoja, como texto, para mostrar dónde está un problema.
 *
 * This interface was referenced by `TiposApp`'s JSON-Schema
 * via the `definition` "Recorte".
 */
export interface Recorte {
  /**
   * `celdas[f][c]` es la celda de la fila `fila_inicio + f` y la columna `columna_inicio + c`.
   */
  celdas: string[][];
  columna_inicio: number;
  fila_inicio: number;
  hoja: string;
}
/**
 * This interface was referenced by `TiposApp`'s JSON-Schema
 * via the `definition` "Resultado".
 */
export interface Resultado {
  cifras: Cifra[];
  ejecucion: Ejecucion;
  para_revisar: Problema[];
  titulo_periodos: string;
}
/**
 * This interface was referenced by `TiposApp`'s JSON-Schema
 * via the `definition` "RevisionArchivo".
 */
export interface RevisionArchivo {
  archivo: ArchivoElegido;
  personas?: number | null;
  problemas: ProblemaVista[];
}
