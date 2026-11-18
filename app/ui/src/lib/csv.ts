// CSV para abrir con doble clic en Excel en español (Plan 7, decisión 4): BOM UTF-8, `;`, CRLF y coma decimal.

function celda(v: string | number | null | undefined): string {
  if (v == null) return "";
  const texto = typeof v === "number" ? String(v).replace(".", ",") : v;
  return /[;"\r\n]/.test(texto) ? `"${texto.replace(/"/g, '""')}"` : texto;
}

export function aCsv(encabezados: string[], filas: (string | number | null | undefined)[][]): string {
  const lineas = [encabezados, ...filas].map((f) => f.map(celda).join(";"));
  return `﻿${lineas.join("\r\n")}\r\n`;
}

/** CSV con una primera línea de contexto (por ejemplo, los filtros aplicados) y luego la tabla. */
export function csvConContexto(
  contexto: string,
  encabezados: string[],
  filas: (string | number | null | undefined)[][],
): string {
  return aCsv([contexto], [encabezados, ...filas]);
}

/** Descarga un texto como archivo (en el navegador y en la ventana del tablero de la app). */
export function descargar(nombre: string, contenido: string, tipo = "text/csv;charset=utf-8"): void {
  const url = URL.createObjectURL(new Blob([contenido], { type: tipo }));
  const a = document.createElement("a");
  a.href = url;
  a.download = nombre;
  document.body.append(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}
