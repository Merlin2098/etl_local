import { fechaCorta, nombreMes } from "../../lib/formato";
import type { ArchivoElegido } from "../tipos";

/** Nombre de cada fuente complementaria (las que no traen fecha en el nombre del archivo). */
const FUENTES: Record<string, string> = { licencias: "Control de licencias" };

/**
 * «Septiembre 2025», «Corte: 12 nov 2025»; un complemento sin fecha se nombra por su fuente
 * («Control de licencias»); si no, el nombre del archivo.
 */
export function etiquetaArchivo(a: ArchivoElegido, fuente?: string): string {
  const f = a.fecha_nombre;
  if (!f) return (fuente && FUENTES[fuente]) || a.nombre;
  return "periodo" in f ? nombreMes(f.periodo) : `Corte: ${fechaCorta(f.fecha_corte)}`;
}
