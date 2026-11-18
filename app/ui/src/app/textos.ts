// Textos de la UI (Plan 6, sección «Textos»): sin jerga técnica (diseno §Principios 3).

import { fechaCorta, listaMeses, mesMinuscula } from "../lib/formato";
import type { EstadoReporte, Paso, Problema, Reporte, RevisionArchivo } from "./tipos";
import { textoFecha } from "./tipos";

export function textoPaso(paso: Paso, ctx: { archivos: number; periodos: string[] }): string {
  switch (paso) {
    case "leer":
      return ctx.archivos === 1 ? "Leer el archivo de Excel" : `Leer los ${ctx.archivos} archivos de Excel`;
    case "licencias":
      return "Agregar las licencias de cada persona";
    case "planilla":
      if (ctx.periodos.length === 0) return "Preparar la planilla guardada";
      return ctx.periodos.length === 1
        ? `Preparar ${mesMinuscula(ctx.periodos[0] ?? "")}`
        : `Juntar ${listaMeses(ctx.periodos)}`;
    case "base_personal":
      return "Preparar la base de personal y sus alertas";
    case "otros_reportes":
      return "Ordenar los demás reportes";
    case "controles":
      return "Revisar que los datos cuadren";
    case "guardar":
      return "Guardar el reporte";
  }
}

export function tieneError(r: RevisionArchivo): boolean {
  return r.problemas.some((p) => p.problema.severidad === "error");
}

export function tituloRevision(revs: RevisionArchivo[]): string {
  const n = revs.filter(tieneError).length;
  if (n === 0) return "Todo listo para procesar";
  return `Hay que arreglar ${n} ${n === 1 ? "archivo" : "archivos"} antes de seguir`;
}

/** «Seguir sin noviembre»: meses (o nombres de archivo, si no hay mes) de los archivos con error. */
export function textoSeguirSin(revs: RevisionArchivo[]): string {
  const conError = revs.filter(tieneError);
  const periodos = conError.map((r) => textoFecha(r.archivo.fecha_nombre));
  if (periodos.every((p): p is string => p !== null && p.length === 7)) {
    return `Seguir sin ${listaMeses(periodos)}`;
  }
  return conError.length === 1 ? "Seguir sin ese archivo" : "Seguir sin esos archivos";
}

export function textoBotonProcesar(reporte: Reporte, n: number): string {
  const articulo = reporte.genero === "femenino" ? "la" : "el";
  if (!reporte.varios_archivos) return `Procesar ${articulo} ${reporte.titulo.toLowerCase()}`;
  return n === 1 ? "Procesar el mes" : `Procesar los ${n} meses`;
}

const CERRAR = ["Guarda y cierra el archivo.", "Vuelve aquí y pulsa «Revisar de nuevo»."];

export function pasosArreglo(p: Problema): string[] {
  if (p.codigo === "PRE-10" && p.celda_encontrada && p.celda) {
    return [`Abre el archivo y deja los títulos de las columnas empezando en ${p.celda}.`, ...CERRAR];
  }
  return ["Abre el archivo y corrige lo indicado.", ...CERRAR];
}

export function tituloListo(reporte: Reporte): string {
  const lista = reporte.genero === "femenino" ? "lista" : "listo";
  return `Tu ${reporte.titulo.toLowerCase()} está ${lista}`;
}

/** «Estamos preparando tu planilla mensual». */
export function tituloProcesando(reporte: Reporte): string {
  return `Estamos preparando tu ${reporte.titulo.toLowerCase()}`;
}

/** Estado de una tarjeta de inicio: «Último mes: octubre 2025», «Al día · 12 nov 2025»… */
export function textoVigencia(e: EstadoReporte): string {
  const u = e.ultimo;
  // Plan 8: los reportes sin fecha en el nombre del archivo están al día en cuanto se cargan.
  if (e.vigencia === "al_dia" && u == null) return "Al día";
  if (e.vigencia === "nunca" || u == null) return "Aún no se ha cargado";
  const fecha = "periodo" in u ? `${mesMinuscula(u.periodo)} ${u.periodo.slice(0, 4)}` : fechaCorta(u.fecha_corte);
  if (e.vigencia === "al_dia") return `Al día · ${fecha}`;
  return "periodo" in u ? `Último mes: ${fecha}` : `Corte: ${fecha}`;
}
