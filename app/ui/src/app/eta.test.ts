import { expect, test } from "vitest";
import { textoEta } from "./eta";

test("sin previsión no se muestra nada", () => {
  expect(textoEta(null, 0)).toBeNull();
});

test("redondea hacia arriba a múltiplos de 5 segundos", () => {
  expect(textoEta(30000, 8000)).toBe("Quedan unos 25 segundos");
  expect(textoEta(30000, 0)).toBe("Quedan unos 30 segundos");
  expect(textoEta(30000, 24000)).toBe("Quedan unos 10 segundos");
});

test("falta poco y tarda más de lo habitual", () => {
  expect(textoEta(30000, 27000)).toBe("Falta poco");
  expect(textoEta(30000, 30000)).toBe("Falta poco");
  expect(textoEta(30000, 40000)).toBe("Está tardando más de lo habitual");
});
