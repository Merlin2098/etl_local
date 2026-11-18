import { expect, test } from "vitest";
import { PARES, generar } from "../../../scripts/tipos.mjs";

// Plan 7, Tarea 2: los .gen.ts corresponden a los esquemas que escribe Rust.
const archivos = import.meta.glob(["./*.json", "./*.gen.ts", "../../app/*.json", "../../app/*.gen.ts"], { query: "?raw", import: "default", eager: true });

function leer(ruta: string): string {
  const nombre = ruta.replace(/^src\/lib\/api\//, "./").replace(/^src\/app\//, "../../app/");
  const clave = Object.keys(archivos).find((k) => k === nombre);
  if (!clave) throw new Error(`falta ${ruta}`);
  return (archivos[clave] as string).replace(/\r\n/g, "\n");
}

test.each(PARES)("%s genera %s sin cambios", async (entrada, salida, nombre) => {
  const esperado = await generar(JSON.parse(leer(entrada)), nombre);
  expect(leer(salida), "regenerar con npm run tipos").toBe(esperado);
});
