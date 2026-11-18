// Tipos de los comandos de la app de escritorio. Los datos salen de los tipos de Rust (`tipos.gen.ts`,
// generado con `npm run tipos`, Plan 7); aquí quedan solo lo que la UI envía y sus utilidades.

import type { FechaNombre } from "./tipos.gen";

export type {
  ArchivoElegido,
  Avance,
  Cifra,
  Ejecucion,
  ErrorApp,
  EstadoEtapa,
  EstadoReporte,
  Etapa,
  FechaNombre,
  Fuente,
  Genero,
  Inicio,
  Paso,
  Problema,
  ProblemaVista,
  Recorte,
  Reporte,
  Resultado,
  RevisionArchivo,
  Severidad,
  TarjetaReporte,
  Vigencia,
} from "./tipos.gen";

/** Lo que la UI pide abrir (comando `abrir`; solo lo deserializa Rust). */
export type Abrir = { tipo: "carpeta" } | { tipo: "excel"; reporte: string } | { tipo: "archivo"; ruta: string };

/** Periodo `AAAA-MM` o fecha `AAAA-MM-DD` de una `FechaNombre`. */
export function textoFecha(f: FechaNombre | null | undefined): string | null {
  if (!f) return null;
  return "periodo" in f ? f.periodo : f.fecha_corte;
}
