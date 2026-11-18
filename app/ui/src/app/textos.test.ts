import { describe, expect, test } from "vitest";
import {
  pasosArreglo,
  textoBotonProcesar,
  textoPaso,
  textoSeguirSin,
  tituloListo,
  tituloRevision,
} from "./textos";
import type { Problema, Reporte, RevisionArchivo } from "./tipos";

const planilla: Reporte = {
  id: "planilla",
  titulo: "Planilla mensual",
  descripcion: "",
  fuente: "planilla",
  varios_archivos: true,
  complementos: ["licencias"],
  excel: "planilla",
  genero: "femenino",
};
const base: Reporte = { ...planilla, id: "base_personal", titulo: "Base de personal", fuente: "base_personal", varios_archivos: false, complementos: [] };

function problema(extra: Partial<Problema> = {}): Problema {
  return { codigo: "PRE-09", severidad: "error", archivo: "a.xlsx", hoja: null, celda: null, mensaje: "m", ...extra };
}

function revision(periodo: string, conError: boolean): RevisionArchivo {
  return {
    archivo: { ruta: `C:/x/${periodo}.xlsx`, nombre: `${periodo}.xlsx`, carpeta: "x", bytes: 1, modificado: null, fecha_nombre: { periodo } },
    personas: conError ? null : 10,
    problemas: conError ? [{ problema: problema(), recorte: null }] : [],
  };
}

describe("textoPaso", () => {
  const ctx = { archivos: 3, periodos: ["2025-09", "2025-10", "2025-11"] };
  test("cada paso en lenguaje de oficina", () => {
    expect(textoPaso("leer", ctx)).toBe("Leer los 3 archivos de Excel");
    expect(textoPaso("leer", { ...ctx, archivos: 1 })).toBe("Leer el archivo de Excel");
    expect(textoPaso("licencias", ctx)).toBe("Agregar las licencias de cada persona");
    expect(textoPaso("planilla", ctx)).toBe("Juntar septiembre, octubre y noviembre");
    expect(textoPaso("planilla", { archivos: 1, periodos: ["2025-11"] })).toBe("Preparar noviembre");
    // Al procesar solo la base de personal se rehace la planilla guardada: sin meses elegidos.
    expect(textoPaso("planilla", { archivos: 1, periodos: [] })).toBe("Preparar la planilla guardada");
    expect(textoPaso("base_personal", ctx)).toBe("Preparar la base de personal y sus alertas");
    expect(textoPaso("controles", ctx)).toBe("Revisar que los datos cuadren");
    expect(textoPaso("otros_reportes", ctx)).toBe("Ordenar los demás reportes");
    expect(textoPaso("guardar", ctx)).toBe("Guardar el reporte");
  });
});

describe("revisión", () => {
  test("tituloRevision según archivos con error", () => {
    expect(tituloRevision([revision("2025-09", false)])).toBe("Todo listo para procesar");
    expect(tituloRevision([revision("2025-09", false), revision("2025-11", true)])).toBe(
      "Hay que arreglar 1 archivo antes de seguir",
    );
    expect(tituloRevision([revision("2025-10", true), revision("2025-11", true)])).toBe(
      "Hay que arreglar 2 archivos antes de seguir",
    );
  });

  test("textoSeguirSin nombra los meses con error", () => {
    expect(textoSeguirSin([revision("2025-09", false), revision("2025-11", true)])).toBe("Seguir sin noviembre");
    expect(textoSeguirSin([revision("2025-10", true), revision("2025-11", true)])).toBe(
      "Seguir sin octubre y noviembre",
    );
  });

  test("textoBotonProcesar", () => {
    expect(textoBotonProcesar(planilla, 3)).toBe("Procesar los 3 meses");
    expect(textoBotonProcesar(planilla, 1)).toBe("Procesar el mes");
    expect(textoBotonProcesar(base, 1)).toBe("Procesar la base de personal");
    // Revisión final del Plan 8: el artículo sigue el género del reporte.
    const practicantes: Reporte = { ...base, id: "practicantes", titulo: "Control de practicantes", genero: "masculino" };
    expect(textoBotonProcesar(practicantes, 1)).toBe("Procesar el control de practicantes");
  });

  test("pasosArreglo para cabecera desplazada y genérico", () => {
    expect(pasosArreglo(problema({ codigo: "PRE-10", celda: "A6", celda_encontrada: "A7" }))).toEqual([
      "Abre el archivo y deja los títulos de las columnas empezando en A6.",
      "Guarda y cierra el archivo.",
      "Vuelve aquí y pulsa «Revisar de nuevo».",
    ]);
    expect(pasosArreglo(problema())[0]).toBe("Abre el archivo y corrige lo indicado.");
  });
});

test("tituloListo con concordancia", () => {
  expect(tituloListo(planilla)).toBe("Tu planilla mensual está lista");
  expect(tituloListo({ ...planilla, titulo: "Reporte de altas", genero: "masculino" })).toBe(
    "Tu reporte de altas está listo",
  );
});

test("textoVigencia de las tarjetas", async () => {
  const { textoVigencia } = await import("./textos");
  expect(textoVigencia({ reporte: planilla, vigencia: "atrasado", ultimo: { periodo: "2025-10" } })).toBe(
    "Último mes: octubre 2025",
  );
  expect(textoVigencia({ reporte: planilla, vigencia: "al_dia", ultimo: { periodo: "2025-11" } })).toBe(
    "Al día · noviembre 2025",
  );
  expect(textoVigencia({ reporte: base, vigencia: "al_dia", ultimo: { fecha_corte: "2025-11-12" } })).toBe(
    "Al día · 12 nov 2025",
  );
  expect(textoVigencia({ reporte: base, vigencia: "atrasado", ultimo: { fecha_corte: "2025-09-30" } })).toBe(
    "Corte: 30 sep 2025",
  );
  expect(textoVigencia({ reporte: base, vigencia: "nunca", ultimo: null })).toBe("Aún no se ha cargado");
  // Plan 8: un reporte sin fecha en el nombre del archivo (exámenes de salida) cargado está al día, sin fecha.
  expect(textoVigencia({ reporte: base, vigencia: "al_dia", ultimo: null })).toBe("Al día");
});
