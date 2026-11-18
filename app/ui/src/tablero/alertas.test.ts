import { expect, test } from "vitest";
import { COLUMNAS_ALERTAS, COLUMNAS_SIN_BASE, celdasAlerta, celdasSinBase, csvDetalle, textoAlertas } from "./alertas";

const fila = {
  numero_documento: "99000001",
  nombre_completo: "Persona de prueba",
  gerencia: "Operaciones",
  sede: "Lima",
  tipo_trabajador: "Empleado",
  fecha_fin_contrato: "2026-03-31",
  fecha_nacimiento: "1961-05-21",
  alerta_contrato_obra: true,
  alerta_contrato_incremento: false,
  cumple_65_este_anio: false,
  cumple_65_proximo_anio: true,
  cumple_70_este_anio: false,
  cumple_70_proximo_anio: false,
};

test("textos de las alertas de una persona", () => {
  expect(textoAlertas(fila)).toEqual(["Contrato por obra o servicio por superar su plazo", "Cumple 65 el próximo año"]);
  expect(textoAlertas({ alerta_contrato_incremento: true, cumple_70_este_anio: true })).toEqual([
    "Contrato por incremento de actividad por superar su plazo",
    "Cumple 70 este año",
  ]);
});

test("columnas y celdas de la tabla de alertas", () => {
  expect(COLUMNAS_ALERTAS).toEqual([
    "Documento",
    "Nombre",
    "Gerencia",
    "Sede",
    "Tipo de trabajador",
    "Alertas",
    "Fin de contrato",
    "Nacimiento",
  ]);
  expect(celdasAlerta(fila)).toEqual([
    "99000001",
    "Persona de prueba",
    "Operaciones",
    "Lima",
    "Empleado",
    "Contrato por obra o servicio por superar su plazo · Cumple 65 el próximo año",
    "31/03/2026",
    "21/05/1961",
  ]);
  expect(celdasAlerta({ numero_documento: "1" })[6]).toBe("");
});

test("columnas y celdas de las personas sin base", () => {
  expect(COLUMNAS_SIN_BASE).toEqual(["Documento", "Apellidos y nombres", "Área", "Centro de costo"]);
  expect(
    celdasSinBase({
      numero_documento: "2",
      apellido_paterno: "Pérez",
      apellido_materno: "Soto",
      nombres: "Ana",
      area: "Planta",
      centro_costo: "CC-10",
    }),
  ).toEqual(["2", "Pérez Soto, Ana", "Planta", "CC-10"]);
});

test("en el CSV el documento conserva sus ceros a la izquierda (revisión final)", () => {
  const csv = csvDetalle(COLUMNAS_SIN_BASE, [celdasSinBase({ numero_documento: "01234567", nombres: "Ana" })]);
  expect(csv.split("\r\n")[1]).toBe('"=""01234567""";Ana;;');
});
