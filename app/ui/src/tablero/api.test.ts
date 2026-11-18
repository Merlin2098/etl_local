import { expect, test } from "vitest";
import { FalloApi, crearCliente } from "./api";

function falsoFetch(respuesta: { status: number; cuerpo: unknown }, urls: string[]): typeof fetch {
  return (async (url: string | URL | Request) => {
    urls.push(String(url));
    return new Response(JSON.stringify(respuesta.cuerpo), { status: respuesta.status });
  }) as typeof fetch;
}

test("solo viajan los filtros con valor", async () => {
  const urls: string[] = [];
  const c = crearCliente("", falsoFetch({ status: 200, cuerpo: {} }, urls));
  await c.indicadores({ periodo: "2025-11", gerencia: null, sede: "", tipo_trabajador: undefined });
  expect(urls).toEqual(["/api/v1/indicadores?periodo=2025-11"]);
  await c.detalle("sin-base", {});
  expect(urls[1]).toBe("/api/v1/detalle/sin-base");
});

test("un error de la API lanza FalloApi con código, mensaje y estado", async () => {
  const c = crearCliente("", falsoFetch({ status: 400, cuerpo: { codigo: "filtro_invalido", mensaje: "No existe" } }, []));
  const e = await c.estado().catch((x: unknown) => x);
  expect(e).toBeInstanceOf(FalloApi);
  expect(e).toMatchObject({ codigo: "filtro_invalido", message: "No existe", estado: 400 });
});
