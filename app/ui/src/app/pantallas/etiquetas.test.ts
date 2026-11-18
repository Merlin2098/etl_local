import { expect, test } from "vitest";
import type { ArchivoElegido } from "../tipos";
import { etiquetaArchivo } from "./etiquetas";

const archivo = (nombre: string, fecha_nombre: ArchivoElegido["fecha_nombre"]): ArchivoElegido => ({
  ruta: `C:/x/${nombre}`,
  nombre,
  carpeta: "x",
  bytes: 1,
  modificado: null,
  fecha_nombre,
});

test("etiqueta por mes o por corte", () => {
  expect(etiquetaArchivo(archivo("Planilla 2025-11 Empleados.xlsx", { periodo: "2025-11" }))).toBe("Noviembre 2025");
  expect(etiquetaArchivo(archivo("BD. 12.11.2025..xlsx", { fecha_corte: "2025-11-12" }))).toBe("Corte: 12 nov 2025");
});

test("el complemento sin fecha se nombra por su fuente, no repite el nombre del archivo", () => {
  expect(etiquetaArchivo(archivo("CONTROL DE LICENCIAS.xlsx", null), "licencias")).toBe("Control de licencias");
  expect(etiquetaArchivo(archivo("otro.xlsx", null))).toBe("otro.xlsx");
});
