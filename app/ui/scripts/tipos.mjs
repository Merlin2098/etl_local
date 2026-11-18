// Plan 7: tipos TypeScript derivados de los tipos de Rust (ADR-0005 §3).
// Rust escribe los JSON Schema (`ACTUALIZAR_ESQUEMA=1 cargo test …`) y este script los convierte:
//   src/lib/api/esquema.json -> src/lib/api/tipos.gen.ts   (API del tablero)
//   src/app/esquema.json     -> src/app/tipos.gen.ts       (comandos de la app de escritorio)
// Uso: `npm run tipos`. Un test de Vitest falla si los .gen.ts quedaron desfasados.
import { readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { compile } from "json-schema-to-typescript";

export const PARES = [
  ["src/lib/api/esquema.json", "src/lib/api/tipos.gen.ts", "TiposApi"],
  ["src/app/esquema.json", "src/app/tipos.gen.ts", "TiposApp"],
];

const CABECERA = "// Generado desde el esquema de Rust: no editar. Regenerar con `npm run tipos`.";

/** Convierte un esquema con `$defs` en un módulo TypeScript con una declaración por definición. */
export async function generar(esquema, nombre) {
  const raiz = { ...esquema, title: nombre, type: "object", additionalProperties: false };
  const ts = await compile(raiz, nombre, {
    bannerComment: CABECERA,
    unreachableDefinitions: true,
    additionalProperties: false,
    declareExternallyReferenced: true,
    format: true,
    style: { printWidth: 120, semi: true, singleQuote: false },
  });
  return ts.replace(/\r\n/g, "\n");
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const raiz = new URL("..", import.meta.url);
  for (const [entrada, salida, nombre] of PARES) {
    const esquema = JSON.parse(readFileSync(new URL(entrada, raiz), "utf8"));
    writeFileSync(new URL(salida, raiz), await generar(esquema, nombre));
    console.log(`${salida} al día`);
  }
}
