import { expect, test } from "vitest";
import { aCsv, csvConContexto } from "./csv";

test("el CSV de un gráfico dice primero qué filtros lo produjeron (revisión final)", () => {
  expect(csvConContexto("Filtros: Gerencia: A", ["Mes", "Personas"], [["Noviembre 2025", 3]])).toBe(
    "﻿Filtros: Gerencia: A\r\nMes;Personas\r\nNoviembre 2025;3\r\n",
  );
});

test("CSV para Excel en español: BOM, punto y coma, CRLF, comillas y coma decimal", () => {
  expect(aCsv(["Gerencia", "Personas"], [['Op; "A"', 1.5]])).toBe('﻿Gerencia;Personas\r\n"Op; ""A""";1,5\r\n');
  expect(aCsv(["a"], [[null], [1234567]])).toBe("﻿a\r\n\r\n1234567\r\n");
});
