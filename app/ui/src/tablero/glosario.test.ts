import { expect, test } from "vitest";
import { ALCANCE_FILTROS, GLOSARIO, calculoConEjemplo, ejemplo } from "./glosario";
import { datosDePrueba } from "./prueba";

test("cada indicador del Resumen tiene su explicación", () => {
  for (const id of ["IND-01", "IND-02", "IND-03", "IND-04", "IND-05", "IND-07", "IND-08"]) {
    expect(GLOSARIO[id]?.calculo, id).toBeTruthy();
  }
});

test("ejemplos con los datos vigentes", () => {
  const d = datosDePrueba();
  // IND-03 = 23; promedio de IND-01 (1.239 y 1.243) = 1.241; IND-04 = 1,9 %.
  expect(ejemplo("IND-04", d)).toBe("Este mes: 23 ÷ 1.241 = 1,9 %");
  // 1.050 días ÷ (1.239 × 30) = 37.170 → 2,8 %.
  expect(ejemplo("IND-05", d)).toBe("Este mes: 1.050 ÷ 37.170 = 2,8 %");
  d.indicadores.indicadores = d.indicadores.indicadores.map((i) => (i.id === "IND-04" ? { ...i, valor: null } : i));
  expect(ejemplo("IND-04", d)).toBeNull();
  expect(ejemplo("IND-05", null)).toBeNull();
});

test("alcance de filtros: una fila por sección del Resumen", () => {
  expect(ALCANCE_FILTROS.map((f) => f.grafico)).toEqual([
    "Indicadores de arriba",
    "Riesgos a atender",
    "Entradas y salidas de personal por mes",
    "Personas en planilla por gerencia",
    "Días de ausencia por motivo",
  ]);
  expect(ALCANCE_FILTROS[1]!.periodo).toBe("No (última base de personal)");
  expect(ALCANCE_FILTROS[2]!.periodo).toBe("No (últimos 6 meses)");
  expect(ALCANCE_FILTROS.every((f) => f.gerencia && f.sede && f.tipo)).toBe(true);
});

test("el ejemplo va separado del cálculo por un espacio (prueba manual)", () => {
  expect(calculoConEjemplo("Salidas ÷ promedio.", "Este mes: 23 ÷ 1.237 = 1,9 %")).toBe("Salidas ÷ promedio. Este mes: 23 ÷ 1.237 = 1,9 %.");
  expect(calculoConEjemplo("Salidas ÷ promedio.", null)).toBe("Salidas ÷ promedio.");
});
