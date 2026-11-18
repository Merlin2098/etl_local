import { expect, test } from "vitest";
import type { Recorte } from "../app/tipos";
import { letraColumna, marcas } from "./recorte";

const r: Recorte = {
  hoja: "Planilla",
  fila_inicio: 5,
  columna_inicio: 1,
  celdas: [
    ["", "", "", ""],
    ["", "", "", ""],
    ["APELLIDO PATERNO", "NOMBRES", "TIPO", "DNI"],
    ["Pérez", "Ana", "DNI", "1"],
  ],
};

test("marca la celda esperada y la fila donde están ahora", () => {
  const m = marcas(r, "A6", "A7");
  expect(m[1]?.[0]).toBe("esperada");
  expect(m[2]).toEqual(["real", "real", "real", "real"]);
  expect(m[0]).toEqual([null, null, null, null]);
  expect(m[3]).toEqual([null, null, null, null]);
  expect(m[1]?.slice(1)).toEqual([null, null, null]);
});

test("sin celda encontrada no hay fila real", () => {
  const m = marcas(r, "B6", null);
  expect(m[1]?.[1]).toBe("esperada");
  expect(m.flat().includes("real")).toBe(false);
});

test("letraColumna", () => {
  expect(letraColumna(1)).toBe("A");
  expect(letraColumna(26)).toBe("Z");
  expect(letraColumna(27)).toBe("AA");
  expect(letraColumna(52)).toBe("AZ");
});
