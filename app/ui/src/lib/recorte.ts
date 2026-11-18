// Recorte de hoja de Excel en los errores (TR-D13): qué celdas se destacan.

import type { Recorte } from "../app/tipos";

export type Marca = "esperada" | "real" | null;

/** `1` → `A`, `27` → `AA`. */
export function letraColumna(n: number): string {
  let letras = "";
  let resto = n;
  while (resto > 0) {
    const m = (resto - 1) % 26;
    letras = String.fromCharCode(65 + m) + letras;
    resto = Math.floor((resto - 1) / 26);
  }
  return letras;
}

function posicion(celda: string): { fila: number; columna: number } | null {
  const m = /^([A-Z]+)(\d+)$/.exec(celda.trim().toUpperCase());
  if (!m?.[1] || !m[2]) return null;
  let columna = 0;
  for (const c of m[1]) columna = columna * 26 + (c.charCodeAt(0) - 64);
  return { fila: Number(m[2]), columna };
}

/** La celda esperada se marca `esperada`; la fila de la celda encontrada, `real`. */
export function marcas(r: Recorte, esperada: string | null, encontrada: string | null): Marca[][] {
  const e = esperada ? posicion(esperada) : null;
  const f = encontrada ? posicion(encontrada) : null;
  return r.celdas.map((fila, i) =>
    fila.map((_, j) => {
      const numFila = r.fila_inicio + i;
      const numColumna = r.columna_inicio + j;
      if (e && e.fila === numFila && e.columna === numColumna) return "esperada";
      if (f && f.fila === numFila) return "real";
      return null;
    }),
  );
}
