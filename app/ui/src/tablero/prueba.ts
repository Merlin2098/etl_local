// Datos de prueba del tablero (sintéticos, con las cifras de los ejemplos del Plan 7).
import type { Indicador, Parte, RespuestaIndicadores } from "../lib/api/tipos.gen";
import type { DatosResumen } from "./estado.svelte";

export function indicador(id: string, campos: Partial<Indicador> = {}): Indicador {
  return {
    id,
    nombre: id,
    pregunta: "",
    unidad: "personas",
    mejor_si: "neutro",
    desglose: [],
    por_gerencia: [],
    origen: { archivos: [], fuente: "planilla", reglas: [id], periodo: "2025-11" },
    valor: 0,
    anterior: null,
    ...campos,
  };
}

export const parte = (categoria: string, valor: number, etiqueta = categoria): Parte => ({ categoria, etiqueta, valor });

export function datosDePrueba(): DatosResumen {
  const p = parte;
  const indicadores: RespuestaIndicadores = {
    periodo: "2025-11",
    periodo_anterior: "2025-10",
    fecha_corte: "2025-11-12",
    filtros: { periodo: "2025-11" },
    indicadores: [
      indicador("IND-01", {
        valor: 1239,
        anterior: 1243,
        por_gerencia: [p("Operaciones", 596), p("Mantenimiento", 284), p("Administración", 141)],
      }),
      indicador("IND-02", { valor: 19, anterior: 15 }),
      indicador("IND-03", { valor: 23, anterior: 11 }),
      indicador("IND-04", { unidad: "fraccion", mejor_si: "baja", valor: 0.0186, anterior: 0.009, comparacion: "peor" }),
      indicador("IND-05", {
        unidad: "fraccion",
        mejor_si: "baja",
        valor: 1050 / 37170,
        anterior: 0.024,
        comparacion: "peor",
        desglose: [
          p("dias_descanso_medico", 412, "Descanso médico"),
          p("dias_licencia_con_goce", 268, "Licencia con goce de sueldo"),
          p("dias_subsidio_enfermedad", 190, "Subsidio por enfermedad"),
          p("dias_licencia_sin_goce", 96, "Licencia sin goce de sueldo"),
          p("dias_subsidio_maternidad", 84, "Subsidio por maternidad"),
          p("dias_subsidio_otros", 0, "Otros subsidios"),
        ],
      }),
      indicador("IND-06", { unidad: "horas", mejor_si: "baja", valor: 7.3 }),
      indicador("IND-07", {
        mejor_si: "baja",
        valor: 18,
        origen: { archivos: [], fuente: "base_personal", reglas: ["IND-07"], fecha_corte: "2025-11-12" },
        desglose: [p("alerta_contrato_obra", 11), p("alerta_contrato_incremento", 7)],
        por_gerencia: [p("Operaciones", 9), p("Mantenimiento", 5), p("Comercial", 2), p("Seguridad", 1), p("Tecnología", 1)],
      }),
      indicador("IND-08", {
        valor: 13,
        origen: { archivos: [], fuente: "base_personal", reglas: ["IND-08"], fecha_corte: "2025-11-12" },
        desglose: [
          p("cumple_65_este_anio", 5),
          p("cumple_65_proximo_anio", 4),
          p("cumple_70_este_anio", 1),
          p("cumple_70_proximo_anio", 3),
        ],
        por_gerencia: [p("Operaciones", 6), p("Mantenimiento", 4), p("Administración", 3)],
      }),
    ],
  };
  const meses = ["2025-04", "2025-05", "2025-06", "2025-07", "2025-08", "2025-09", "2025-10", "2025-11"];
  return {
    estado: {
      marca: "m1",
      version_motor: "0.1.0",
      fecha_corte: "2025-11-12",
      periodos: meses.slice(2),
      fuentes: [
        { fuente: "planilla", archivo: "Planilla 2025-11 Empleados.xlsx", fecha_nombre: { periodo: "2025-11" } },
        { fuente: "base_personal", archivo: "BD. 12.11.2025..xlsx", fecha_nombre: { fecha_corte: "2025-11-12" } },
        { fuente: "licencias", archivo: "CONTROL DE LICENCIAS.xlsx", fecha_nombre: null },
      ],
      avisos: 0,
      periodos_mezclados: false,
    },
    opciones: {
      periodos: ["2025-10", "2025-11"],
      gerencias: ["Administración", "Mantenimiento", "Operaciones"],
      sedes: ["Lima"],
      tipos_trabajador: ["Empleado"],
    },
    indicadores,
    series: {
      filtros: {},
      series: ["IND-02", "IND-03"].map((id, k) => ({
        id,
        nombre: id,
        unidad: "personas",
        puntos: meses.map((periodo, i) => ({ periodo, valor: 10 + i + k })),
      })),
    },
    checks: { periodo: "2025-11", checks: [{ id: "IND-C1", valor: 8, mensaje: "8 personas sin base." }] },
  };
}
