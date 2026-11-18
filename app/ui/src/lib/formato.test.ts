import { describe, expect, test } from "vitest";
import { fechaCorta, listaMeses, nombreMes, numero, saludo } from "./formato";

describe("formato", () => {
  test("nombreMes", () => {
    expect(nombreMes("2025-09")).toBe("Septiembre 2025");
    expect(nombreMes("2025-01")).toBe("Enero 2025");
  });

  test("fechaCorta", () => {
    expect(fechaCorta("2025-11-12")).toBe("12 nov 2025");
    expect(fechaCorta("2025-01-05")).toBe("5 ene 2025");
  });

  test("numero con punto de miles también en 4 cifras", () => {
    expect(numero(1227)).toBe("1.227");
    expect(numero(86)).toBe("86");
    expect(numero(3715)).toBe("3.715");
    expect(numero(1234567)).toBe("1.234.567");
    expect(numero(7.29)).toBe("7,29");
    expect(numero(-1500)).toBe("-1.500");
  });

  test("saludo según la hora", () => {
    expect(saludo(8)).toBe("Buenos días");
    expect(saludo(12)).toBe("Buenas tardes");
    expect(saludo(18)).toBe("Buenas tardes");
    expect(saludo(19)).toBe("Buenas noches");
  });

  test("listaMeses", () => {
    expect(listaMeses(["2025-09", "2025-10", "2025-11"])).toBe("septiembre, octubre y noviembre");
    expect(listaMeses(["2025-10", "2025-11"])).toBe("octubre y noviembre");
    expect(listaMeses(["2025-11"])).toBe("noviembre");
    expect(listaMeses([])).toBe("");
  });
});
