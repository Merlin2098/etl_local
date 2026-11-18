import { describe, expect, test } from "vitest";
import type { Filtros } from "../lib/api/tipos.gen";
import { type ClienteApi, FalloApi } from "./api";
import { EstadoTablero } from "./estado.svelte";
import { datosDePrueba } from "./prueba";

type Llamada = { metodo: string; filtros?: Filtros; tipo?: string };

function clienteFalso(opciones: { fallar?: (m: string, f?: Filtros) => FalloApi | null } = {}) {
  const llamadas: Llamada[] = [];
  const d = datosDePrueba();
  const responder = <T>(metodo: string, valor: T, filtros?: Filtros, tipo?: string): Promise<T> => {
    llamadas.push({ metodo, filtros, tipo });
    const fallo = opciones.fallar?.(metodo, filtros);
    return fallo ? Promise.reject(fallo) : Promise.resolve(valor);
  };
  const cliente: ClienteApi = {
    estado: () => responder("estado", d.estado),
    filtros: () => responder("filtros", d.opciones),
    indicadores: (f) => responder("indicadores", d.indicadores, f),
    series: (f) => responder("series", d.series, f),
    checks: (f) => responder("checks", d.checks, f),
    detalle: (tipo, f) => responder("detalle", { tipo, filas: [] }, f, tipo),
  };
  return { cliente, llamadas };
}

function urlFalsa(inicial: string) {
  const u = { actual: inicial, leer: () => u.actual, escribir: (q: string) => (u.actual = q) };
  return u;
}

const sinEspera = () => Promise.resolve();

describe("EstadoTablero: pedidos superpuestos (revisión final)", () => {
  test("una respuesta vieja no pisa a la nueva", async () => {
    const d = datosDePrueba();
    const pendientes: { gerencia: string | null | undefined; resolver: () => void }[] = [];
    const lento = <T>(valor: T, f?: Filtros) =>
      new Promise<T>((r) => pendientes.push({ gerencia: f?.gerencia, resolver: () => r(valor) }));
    const marcar = (g: string | null | undefined) => ({ ...d.indicadores, periodo: g ?? "todas" });
    const cliente: ClienteApi = {
      estado: () => Promise.resolve(d.estado),
      filtros: () => Promise.resolve(d.opciones),
      indicadores: (f) => lento(marcar(f.gerencia), f),
      series: (f) => Promise.resolve({ ...d.series, filtros: f }),
      checks: () => Promise.resolve(d.checks),
      detalle: (tipo) => Promise.resolve({ tipo, filas: [] }),
    };
    const e = new EstadoTablero(cliente, urlFalsa(""), sinEspera);
    const primero = e.cambiarFiltro("gerencia", "A");
    const segundo = e.cambiarFiltro("gerencia", "B");
    await Promise.resolve();
    // Responde primero el pedido nuevo (B) y después el viejo (A).
    pendientes.find((p) => p.gerencia === "B")!.resolver();
    await segundo;
    pendientes.find((p) => p.gerencia === "A")!.resolver();
    await primero;
    expect(e.filtros.gerencia).toBe("B");
    expect(e.datos?.indicadores.periodo).toBe("B");
    expect(e.cargando).toBe(false);
  });
});

describe("EstadoTablero", () => {
  test("cargar pide los cinco datos del resumen y nunca el detalle", async () => {
    const { cliente, llamadas } = clienteFalso();
    const e = new EstadoTablero(cliente, urlFalsa(""), sinEspera);
    await e.cargar();
    expect(llamadas.map((l) => l.metodo).sort()).toEqual(["checks", "estado", "filtros", "indicadores", "series"]);
    expect(e.datos?.indicadores.periodo).toBe("2025-11");
    expect(e.vacio).toBe(false);
  });

  test("los filtros viajan desde la URL y cambiarFiltro la reescribe", async () => {
    const { cliente, llamadas } = clienteFalso();
    const url = urlFalsa("?periodo=2025-10&gerencia=Operaciones");
    const e = new EstadoTablero(cliente, url, sinEspera);
    await e.cargar();
    expect(llamadas.find((l) => l.metodo === "indicadores")?.filtros).toEqual({ periodo: "2025-10", gerencia: "Operaciones" });
    await e.cambiarFiltro("sede", "Lima");
    expect(url.actual).toBe("?periodo=2025-10&gerencia=Operaciones&sede=Lima");
    expect(llamadas.filter((l) => l.metodo === "indicadores").at(-1)?.filtros).toEqual({
      periodo: "2025-10",
      gerencia: "Operaciones",
      sede: "Lima",
    });
    await e.cambiarFiltro("gerencia", null);
    expect(url.actual).toBe("?periodo=2025-10&sede=Lima");
  });

  test("un filtro que ya no existe se quita con un aviso (foco 4)", async () => {
    const { cliente } = clienteFalso({
      fallar: (_m, f) =>
        f?.gerencia === "Vieja" ? new FalloApi("filtro_invalido", "gerencia: «Vieja» no existe", 400) : null,
    });
    const url = urlFalsa("?periodo=2025-10&gerencia=Vieja&sede=Lima");
    const e = new EstadoTablero(cliente, url, sinEspera);
    await e.cargar();
    expect(e.filtros).toEqual({ periodo: "2025-10" });
    expect(url.actual).toBe("?periodo=2025-10");
    expect(e.aviso).toBe("El filtro «Vieja» ya no existe en los datos; se muestran todos.");
    expect(e.error).toBeNull();
    expect(e.datos).not.toBeNull();
  });

  test("sin datos queda vacío, sin error (foco 1)", async () => {
    const { cliente } = clienteFalso({ fallar: () => new FalloApi("sin_datos", "Todavía no hay resultados", 404) });
    const e = new EstadoTablero(cliente, urlFalsa(""), sinEspera);
    await e.cargar();
    expect(e.vacio).toBe(true);
    expect(e.error).toBeNull();
  });

  test("mientras se actualizan los datos reintenta una vez (foco 5)", async () => {
    let fallos = 1;
    const esperas: number[] = [];
    const { cliente } = clienteFalso({
      fallar: (m) => (m === "indicadores" && fallos-- > 0 ? new FalloApi("actualizando", "Espere", 503) : null),
    });
    const e = new EstadoTablero(cliente, urlFalsa(""), async (ms) => void esperas.push(ms));
    await e.cargar();
    expect(esperas).toEqual([1000]);
    expect(e.error).toBeNull();
    expect(e.datos).not.toBeNull();

    const siempre = clienteFalso({ fallar: () => new FalloApi("actualizando", "Espere", 503) });
    const e2 = new EstadoTablero(siempre.cliente, urlFalsa(""), sinEspera);
    await e2.cargar();
    expect(e2.error?.codigo).toBe("actualizando");
  });

  test("Alertas pide los dos detalles; volver al resumen no los pide", async () => {
    const { cliente, llamadas } = clienteFalso();
    const url = urlFalsa("");
    const e = new EstadoTablero(cliente, url, sinEspera);
    await e.cargar();
    await e.irA("alertas");
    expect(llamadas.filter((l) => l.metodo === "detalle").map((l) => l.tipo)).toEqual(["alertas", "sin-base"]);
    expect(e.pestana).toBe("alertas");
    const antes = llamadas.length;
    await e.irA("resumen");
    expect(llamadas.slice(antes).some((l) => l.metodo === "detalle")).toBe(false);
  });
});
