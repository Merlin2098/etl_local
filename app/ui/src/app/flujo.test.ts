import { describe, expect, test, vi } from "vitest";
import { Flujo } from "./flujo.svelte";
import { backendSimulado } from "./simulado";

const PLANILLAS = [
  "C:/Usuarios/demo/Planilla 2025-09 Empleados.xlsx",
  "C:/Usuarios/demo/Planilla 2025-10 Empleados.xlsx",
  "C:/Usuarios/demo/Planilla 2025-11 Empleados.xlsx",
];

async function enArchivos(opciones?: Parameters<typeof backendSimulado>[0], reporte = "planilla") {
  const backend = backendSimulado({ demoraMs: 0, ...opciones });
  const flujo = new Flujo(backend);
  await flujo.cargarInicio();
  await flujo.elegirReporte(reporte);
  return { flujo, backend };
}

describe("Flujo", () => {
  test("elegir planilla pasa a archivos y carga el complemento guardado", async () => {
    const { flujo } = await enArchivos();
    expect(flujo.pantalla).toBe("archivos");
    expect(flujo.reporte?.id).toBe("planilla");
    expect(flujo.complemento.usar).toBe(true);
    expect(flujo.complemento.archivo?.nombre).toBe("CONTROL DE LICENCIAS.xlsx");
  });

  test("agregar ignora un PDF y avisa", async () => {
    const { flujo } = await enArchivos();
    await flujo.agregar([PLANILLAS[0]!, "C:/Usuarios/demo/notas.pdf"]);
    expect(flujo.elegidos.map((a) => a.nombre)).toEqual(["Planilla 2025-09 Empleados.xlsx"]);
    expect(flujo.aviso).toContain("notas.pdf");
  });

  test("base de personal se queda con un solo archivo", async () => {
    const { flujo } = await enArchivos({}, "base_personal");
    await flujo.agregar(["C:/x/BD. 12.11.2025..xlsx", "C:/x/BD. 30.11.2025..xlsx"]);
    expect(flujo.elegidos.map((a) => a.nombre)).toEqual(["BD. 30.11.2025..xlsx"]);
    expect(flujo.aviso).toContain("un solo archivo");
  });

  test("dos archivos del mismo mes bloquean la revisión con su motivo", async () => {
    const { flujo } = await enArchivos();
    expect(flujo.puedeRevisar).toEqual({ ok: false, motivo: "Elige al menos un archivo." });
    await flujo.agregar([PLANILLAS[2]!, "C:/otra/Planilla 2025-11 Empleados Final.xlsx"]);
    expect(flujo.puedeRevisar).toEqual({ ok: false, motivo: "Hay dos archivos de Noviembre 2025; quita uno." });
    flujo.quitar("C:/otra/Planilla 2025-11 Empleados Final.xlsx");
    expect(flujo.puedeRevisar.ok).toBe(true);
  });

  test("mes no reconocido bloquea la revisión", async () => {
    const { flujo } = await enArchivos();
    await flujo.agregar(["C:/x/planilla sin mes.xlsx"]);
    expect(flujo.puedeRevisar).toEqual({
      ok: false,
      motivo: "No reconocemos el mes de «planilla sin mes.xlsx». Revisa el nombre del archivo.",
    });
  });

  test("revisión con error desactiva procesar y ofrece seguir sin ese mes", async () => {
    const { flujo } = await enArchivos({ conError: true });
    await flujo.agregar(PLANILLAS);
    await flujo.revisar();
    expect(flujo.pantalla).toBe("revision");
    expect(flujo.puedeProcesar).toEqual({ ok: false, motivo: "Se activa cuando todos los archivos estén en orden." });
    expect(flujo.sinLosConError.map((a) => a.nombre)).toEqual([
      "Planilla 2025-09 Empleados.xlsx",
      "Planilla 2025-10 Empleados.xlsx",
    ]);
  });

  test("seguirSin procesa solo los archivos sin error", async () => {
    const { flujo, backend } = await enArchivos({ conError: true });
    const espia = vi.spyOn(backend, "procesar");
    await flujo.agregar(PLANILLAS);
    await flujo.revisar();
    await flujo.seguirSin();
    expect(espia.mock.calls[0]?.[1]).toEqual(PLANILLAS.slice(0, 2));
    expect(flujo.pantalla).toBe("listo");
    // Revisión final del Plan 6: los pasos nombran solo lo que se procesa, no el mes dejado fuera.
    expect(flujo.contextoPasos).toEqual({ archivos: 2, periodos: ["2025-09", "2025-10"] });
  });

  test("procesar recorre los avances y termina en listo", async () => {
    const { flujo } = await enArchivos();
    const vistos: number[] = [];
    const original = flujo.alAvanzar.bind(flujo);
    flujo.alAvanzar = (a) => {
      original(a);
      vistos.push(a.indice);
    };
    await flujo.agregar(PLANILLAS);
    await flujo.revisar();
    await flujo.procesar();
    expect(vistos).toEqual([0, 1, 2, 3, 4, 5]);
    expect(flujo.pasos).toHaveLength(6);
    expect(flujo.pantalla).toBe("listo");
    expect(flujo.resultado?.cifras.length).toBeGreaterThan(0);
  });

  test("desmarcar licencias las excluye sin borrarlas", async () => {
    const { flujo, backend } = await enArchivos();
    const espia = vi.spyOn(backend, "procesar");
    await flujo.agregar(PLANILLAS);
    flujo.complemento.usar = false;
    await flujo.revisar();
    await flujo.procesar();
    expect(espia.mock.calls[0]?.[2]).toEqual([]);
    expect(espia.mock.calls[0]?.[3]).toEqual(["licencias"]);
  });

  test("un complemento nuevo se revisa y se pasa al procesar", async () => {
    const { flujo, backend } = await enArchivos();
    const revisar = vi.spyOn(backend, "revisar");
    const procesar = vi.spyOn(backend, "procesar");
    await flujo.agregar(PLANILLAS);
    await flujo.cambiarComplemento("C:/Usuarios/demo/CONTROL DE LICENCIAS nuevo.xlsx");
    await flujo.revisar();
    await flujo.procesar();
    expect(revisar.mock.calls[0]?.[2]).toEqual(["C:/Usuarios/demo/CONTROL DE LICENCIAS nuevo.xlsx"]);
    expect(procesar.mock.calls[0]?.[2]).toEqual(["C:/Usuarios/demo/CONTROL DE LICENCIAS nuevo.xlsx"]);
    expect(procesar.mock.calls[0]?.[3]).toEqual([]);
  });

  test("detener vuelve al inicio con el aviso de cancelado", async () => {
    const { flujo } = await enArchivos({ demoraMs: 20 });
    await flujo.agregar(PLANILLAS);
    await flujo.revisar();
    const proceso = flujo.procesar();
    await new Promise((r) => setTimeout(r, 30));
    await flujo.detener();
    await proceso;
    expect(flujo.pantalla).toBe("inicio");
    expect(flujo.aviso).toBe("Se detuvo el proceso; no se cambió ningún reporte.");
  });

  test("error de entradas al procesar muestra los problemas", async () => {
    const { flujo } = await enArchivos({
      errorAlProcesar: {
        codigo: "entradas",
        mensaje: "Hay archivos con errores; no se procesó nada.",
        problemas: [
          { codigo: "PRE-09", severidad: "error", archivo: "a.xlsx", hoja: "Planilla", celda: null, mensaje: "Falta la hoja «Planilla»." },
        ],
      },
    });
    await flujo.agregar(PLANILLAS);
    await flujo.revisar();
    await flujo.procesar();
    expect(flujo.pantalla).toBe("procesando");
    expect(flujo.error?.codigo).toBe("entradas");
    expect(flujo.error?.problemas).toHaveLength(1);
  });

  test("volver al inicio borra el aviso de la pantalla anterior", async () => {
    const { flujo } = await enArchivos();
    await flujo.agregar([PLANILLAS[0]!, "C:/Usuarios/demo/notas.txt"]);
    expect(flujo.aviso).not.toBeNull();
    await flujo.volverAlInicio();
    expect(flujo.aviso).toBeNull();
  });

  test("solo promete la copia anterior si el reporte ya se había cargado", async () => {
    const { flujo } = await enArchivos();
    await flujo.agregar(PLANILLAS);
    await flujo.revisar();
    await flujo.procesar();
    expect(flujo.huboAnterior).toBe(true);

    const nuevo = new Flujo(backendSimulado({ demoraMs: 0 }));
    await nuevo.cargarInicio();
    nuevo.inicio!.tarjetas[0]!.estado.vigencia = "nunca";
    await nuevo.elegirReporte("planilla");
    await nuevo.agregar(PLANILLAS);
    await nuevo.revisar();
    await nuevo.procesar();
    expect(nuevo.pantalla).toBe("listo");
    expect(nuevo.huboAnterior).toBe(false);
  });

  test("ver el tablero pide abrir su ventana", async () => {
    const { flujo, backend } = await enArchivos();
    const espia = vi.spyOn(backend, "abrirTablero");
    await flujo.verTablero();
    expect(espia).toHaveBeenCalledWith(undefined);
    await flujo.verTablero("alertas");
    expect(espia).toHaveBeenLastCalledWith("alertas");
  });

  test("sin ningún reporte cargado, ver el tablero espera con su motivo", async () => {
    const flujo = new Flujo(backendSimulado({ demoraMs: 0 }));
    await flujo.cargarInicio();
    expect(flujo.puedeVerTablero).toEqual({ ok: true, motivo: null });
    for (const t of flujo.inicio!.tarjetas) t.estado.vigencia = "nunca";
    expect(flujo.puedeVerTablero).toEqual({ ok: false, motivo: "Procesa un reporte primero." });
  });
});
