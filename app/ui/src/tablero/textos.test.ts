import { describe, expect, test } from "vitest";
import { datosDePrueba, indicador, parte } from "./prueba";
import {
  cifra,
  csvIndicadores,
  lineaFiltros,
  diasDelMes,
  fuentesUnicas,
  lineaTarjeta,
  masCasos,
  porcentaje,
  textoComparacion,
  textoFuente,
  textoMeta,
  textoRiesgo,
} from "./textos";

describe("formatos del tablero", () => {
  test("porcentaje", () => {
    expect(porcentaje(0.0186)).toBe("1,9 %");
    expect(porcentaje(0)).toBe("0,0 %");
    expect(porcentaje(null)).toBe("—");
    expect(porcentaje(undefined)).toBe("—");
  });

  test("comparación de un indicador neutro en personas", () => {
    expect(textoComparacion(indicador("IND-01", { valor: 1239, anterior: 1243 }), "2025-10")).toBe("4 menos que en octubre");
    expect(textoComparacion(indicador("IND-01", { valor: 1250, anterior: 1243 }), "2025-10")).toBe("7 más que en octubre");
    expect(textoComparacion(indicador("IND-01", { valor: 1243, anterior: 1243 }), "2025-10")).toBe("Igual que en octubre");
  });

  test("comparación con sentido de mejor", () => {
    const rot = indicador("IND-04", { unidad: "fraccion", mejor_si: "baja", valor: 0.0186, anterior: 0.009, comparacion: "peor" });
    expect(textoComparacion(rot, "2025-10")).toBe("Empeoró: era 0,9 % en octubre");
    expect(textoComparacion({ ...rot, comparacion: "mejor" }, "2025-10")).toBe("Mejoró: era 0,9 % en octubre");
    expect(textoComparacion({ ...rot, comparacion: "igual", valor: 0.009 }, "2025-10")).toBe("Igual que en octubre");
  });

  test("sin mes anterior no hay comparación", () => {
    expect(textoComparacion(indicador("IND-01", { valor: 5, anterior: null }), null)).toBeNull();
    expect(textoComparacion(indicador("IND-04", { mejor_si: "baja", valor: null, anterior: null }), "2025-10")).toBeNull();
    // Sin comparación de la API no se inventa «Igual».
    expect(textoComparacion(indicador("IND-04", { mejor_si: "baja", valor: 0.02, anterior: 0.01, comparacion: null }), "2025-10")).toBeNull();
  });

  test("meta", () => {
    expect(textoMeta(indicador("IND-04", { unidad: "fraccion" }))).toBe("Sin meta definida");
    expect(textoMeta(indicador("IND-04", { unidad: "fraccion", meta: 0.015 }))).toBe("Meta: 1,5 %");
  });

  test("datos usados por fuente", () => {
    expect(textoFuente({ fuente: "planilla", archivo: "x", fecha_nombre: { periodo: "2025-10" } }, "2025-11")).toBe(
      "Planilla: noviembre 2025",
    );
    expect(textoFuente({ fuente: "base_personal", archivo: "x", fecha_nombre: { fecha_corte: "2025-11-12" } }, null)).toBe(
      "Base de personal: 12 nov 2025",
    );
    expect(textoFuente({ fuente: "licencias", archivo: "x", fecha_nombre: null }, null)).toBe("Licencias: incluidas");
  });
});

describe("tarjetas y riesgos (Tarea 5)", () => {
  test("segunda línea de cada tarjeta", () => {
    const d = datosDePrueba();
    expect(lineaTarjeta("IND-01", d)).toBe("Octubre: 1.243 · 19 entraron, 23 salieron");
    expect(lineaTarjeta("IND-05", d)).toBe("1.050 días perdidos de 37.170 programados");
    expect(lineaTarjeta("IND-07", d)).toBe("Superan el plazo legal en 6 meses o menos");
    expect(lineaTarjeta("IND-04", d)).toBe("Sin meta definida");
  });

  test("sin valores no aparece NaN, null ni undefined (focos 1 y 3)", () => {
    for (const valor of [null, 0]) {
      const d = datosDePrueba();
      d.indicadores.indicadores = d.indicadores.indicadores.map((i) => ({ ...i, valor, anterior: null, desglose: [] }));
      for (const id of ["IND-01", "IND-04", "IND-05", "IND-07"]) {
        expect(lineaTarjeta(id, d) ?? "").not.toMatch(/NaN|null|undefined/);
      }
    }
    expect(cifra(null, "personas")).toBe("—");
    expect(cifra(Number.NaN, "fraccion")).toBe("—");
  });

  test("un solo mes: la tarjeta usa la nota en vez de la comparación (foco 2)", () => {
    const d = datosDePrueba();
    const nota = "Sin mes anterior: la rotación necesita el mes anterior en los datos.";
    d.indicadores.periodo_anterior = null;
    d.indicadores.indicadores = d.indicadores.indicadores.map((i) =>
      i.id === "IND-04" ? { ...i, valor: null, anterior: null, comparacion: null, nota } : i,
    );
    const rot = d.indicadores.indicadores.find((i) => i.id === "IND-04")!;
    expect(textoComparacion(rot, d.indicadores.periodo_anterior)).toBeNull();
    expect(lineaTarjeta("IND-04", d)).toBe(nota);
  });

  test("riesgo de contratos", () => {
    const i = datosDePrueba().indicadores.indicadores.find((x) => x.id === "IND-07")!;
    expect(textoRiesgo(i)).toEqual({
      cifra: "18",
      titulo: "contratos temporales superarán el plazo legal en los próximos 6 meses",
      detalle:
        "11 por obra o servicio (máx. 5 años) · 7 por incremento de actividad (máx. 3 años). Si se pasan del plazo, pueden considerarse contratos indefinidos.",
      masCasos: "Más casos: Operaciones (9), Mantenimiento (5), Comercial (2), otras (2)",
    });
    expect(textoRiesgo({ ...i, valor: 1 }).titulo).toBe("contrato temporal superará el plazo legal en los próximos 6 meses");
    const cero = textoRiesgo({ ...i, valor: 0, desglose: [], por_gerencia: [] });
    expect(cero.titulo).toBe("Ningún contrato temporal supera su plazo en los próximos 6 meses.");
    expect(cero.masCasos).toBeNull();
  });

  test("riesgo de jubilación", () => {
    const i = datosDePrueba().indicadores.indicadores.find((x) => x.id === "IND-08")!;
    const r = textoRiesgo(i);
    expect(r.titulo).toBe("personas llegan a la edad de jubilación este año o el próximo");
    expect(r.detalle).toBe("9 cumplen 65 años · 4 cumplen 70 años. Conviene planificar reemplazos y traspaso de conocimiento.");
    expect(r.masCasos).toBe("Más casos: Operaciones (6), Mantenimiento (4), Administración (3)");
    expect(textoRiesgo({ ...i, valor: 1 }).titulo).toBe("persona llega a la edad de jubilación este año o el próximo");
    expect(textoRiesgo({ ...i, valor: 0 }).titulo).toBe("Nadie llega a la edad de jubilación este año ni el próximo.");
  });

  test("más casos y días del mes", () => {
    expect(masCasos([parte("A", 3), parte("B", 2)])).toBe("Más casos: A (3), B (2)");
    expect(masCasos([])).toBeNull();
    expect(diasDelMes("2025-11")).toBe(30);
    expect(diasDelMes("2024-02")).toBe(29);
  });
});

describe("resumen en PDF y Datos en Excel (Tarea 9)", () => {
  test("CSV de los indicadores: dice qué filtros tiene y no lleva datos nominales", () => {
    const csv = csvIndicadores(datosDePrueba(), { gerencia: "Operaciones" });
    expect(csv.startsWith("﻿")).toBe(true);
    const lineas = csv.trimEnd().split("\r\n");
    expect(lineas).toHaveLength(10);
    expect(lineas[0]).toBe("﻿Filtros: Periodo: noviembre 2025 · Gerencia: Operaciones · Sede: Todas · Tipo de trabajador: Todos");
    expect(lineas[1]).toBe("Indicador;Valor;Mes anterior;Unidad");
    expect(lineas[5]).toBe("IND-04;1,9 %;0,9 %;%");
    expect(csv).not.toMatch(/numero_documento|nombre_completo|nombres/);
  });

  test("línea de filtros para la impresión", () => {
    const d = datosDePrueba();
    expect(lineaFiltros({}, d.indicadores.periodo)).toBe(
      "Periodo: noviembre 2025 · Gerencia: Todas · Sede: Todas · Tipo de trabajador: Todos",
    );
    expect(lineaFiltros({ gerencia: "Operaciones", sede: "Lima" }, "2025-10")).toBe(
      "Periodo: octubre 2025 · Gerencia: Operaciones · Sede: Lima · Tipo de trabajador: Todos",
    );
  });
});

test("fuentes_del_tablero_ignora_otras: «Datos usados» solo lista planilla, licencias y base (Plan 8)", () => {
  const f = (fuente: string, archivo: string) => ({ fuente, archivo, fecha_nombre: null });
  expect(
    fuentesUnicas([
      f("planilla", "Planilla 2025-10.xlsx"),
      f("examenes_salida", "Examenes de salida.xlsx"),
      f("planilla", "Planilla 2025-11.xlsx"),
      f("practicantes", "BD Practicantes 12.11.2025.xlsx"),
      f("base_personal", "BD. 12.11.2025..xlsx"),
    ]).map((x) => x.archivo),
  ).toEqual(["Planilla 2025-11.xlsx", "BD. 12.11.2025..xlsx"]);
});
