// Backend falso con datos sintéticos inventados: tests de Vitest y revisión visual (`app.html?simulado`).
// Nunca entra en el build de producción (main.ts solo lo importa en desarrollo).

import { type Backend, FalloApp } from "./backend";
import type {
  Abrir,
  ArchivoElegido,
  Avance,
  ErrorApp,
  FechaNombre,
  Inicio,
  Paso,
  Reporte,
  Resultado,
  RevisionArchivo,
} from "./tipos";

export interface OpcionesSimulado {
  /** La planilla de noviembre trae la cabecera desplazada (PRE-10). */
  conError?: boolean;
  /** Pausa entre avances, en ms. */
  demoraMs?: number;
  /** Error con el que termina `procesar`. */
  errorAlProcesar?: ErrorApp;
}

const PLANILLA: Reporte = {
  id: "planilla",
  titulo: "Planilla mensual",
  descripcion: "Horas extra, subsidios, licencias y salidas",
  fuente: "planilla",
  varios_archivos: true,
  complementos: ["licencias"],
  excel: "planilla",
  genero: "femenino",
};

const BASE: Reporte = {
  id: "base_personal",
  titulo: "Base de personal",
  descripcion: "Quién trabaja hoy y alertas de contrato y edad",
  fuente: "base_personal",
  varios_archivos: false,
  complementos: [],
  excel: "base_personal_empleados",
  genero: "femenino",
};

/** Plan 8: reportes del molde, como en config/reportes.yaml (en el modo navegador solo se ven en Inicio). */
const MOLDE: Reporte[] = [
  ["planilla_regimen_especial", "Planilla régimen especial", "Días y horas del personal con régimen especial", true, "femenino"],
  ["altas_personal", "Altas de personal", "Quién entró cada mes, empleados y practicantes", true, "femenino"],
  ["practicantes", "Control de practicantes", "Tiempo de servicio y alertas de 1 y 2 años", false, "masculino"],
  ["examenes_salida", "Control de exámenes médicos de salida", "Ceses y su centro de costo", false, "masculino"],
].map(([id, titulo, descripcion, varios, genero]) => ({
  id: id as string,
  titulo: titulo as string,
  descripcion: descripcion as string,
  fuente: id as string,
  varios_archivos: varios as boolean,
  complementos: [],
  excel: id as string,
  genero: genero as Reporte["genero"],
}));

const CANCELADO: ErrorApp = {
  codigo: "cancelado",
  mensaje: "Se detuvo el proceso; no se cambió ningún reporte.",
  problemas: [],
};

function partir(ruta: string): { nombre: string; carpeta: string } {
  const partes = ruta.split(/[\\/]/);
  const nombre = partes.pop() ?? ruta;
  return { nombre, carpeta: partes.slice(-2).join(" › ") };
}

function fechaDe(reporte: string, nombre: string): FechaNombre | null {
  if (reporte === "planilla") {
    const m = /^Planilla (\d{4}-\d{2})/.exec(nombre);
    return m?.[1] ? { periodo: m[1] } : null;
  }
  const m = /(\d{2})\.(\d{2})\.(\d{4})/.exec(nombre);
  return m ? { fecha_corte: `${m[3]}-${m[2]}-${m[1]}` } : null;
}

function elegido(reporte: string, ruta: string): ArchivoElegido {
  const { nombre, carpeta } = partir(ruta);
  return { ruta, nombre, carpeta, bytes: 2_150_000, modificado: "2025-11-10", fecha_nombre: fechaDe(reporte, nombre) };
}

const espera = (ms: number) => new Promise((r) => setTimeout(r, ms));

export function backendSimulado(opciones: OpcionesSimulado = {}): Backend {
  const demora = opciones.demoraMs ?? 600;
  let detenido = false;

  return {
    async inicio(): Promise<Inicio> {
      return {
        carpeta_visible: "Documentos › Reportes RR. HH.",
        tarjetas: [
          {
            estado: { reporte: PLANILLA, vigencia: "atrasado", ultimo: { periodo: "2025-10" } },
            duracion_prevista_ms: 6 * demora + 2000,
          },
          {
            estado: { reporte: BASE, vigencia: "al_dia", ultimo: { fecha_corte: "2025-11-12" } },
            duracion_prevista_ms: null,
          },
          ...MOLDE.map((reporte) => ({
            estado: { reporte, vigencia: "nunca" as const, ultimo: null },
            duracion_prevista_ms: null,
          })),
        ],
      };
    },

    async reconocer(reporte, rutas) {
      return rutas.map((r) => elegido(reporte, r));
    },

    async complementoGuardado(fuente) {
      if (fuente !== "licencias") return null;
      return elegido("licencias", "C:/Usuarios/demo/Documentos/Reportes RR. HH./entradas/licencias/CONTROL DE LICENCIAS.xlsx");
    },

    async revisar(reporte, rutas, complementos): Promise<RevisionArchivo[]> {
      return [...rutas, ...complementos].map((ruta, i) => {
        const archivo = elegido(reporte, ruta);
        const fecha = archivo.fecha_nombre;
        const esNoviembre = fecha != null && "periodo" in fecha && fecha.periodo === "2025-11";
        if (opciones.conError && esNoviembre) {
          return {
            archivo,
            personas: null,
            problemas: [
              {
                problema: {
                  codigo: "PRE-10",
                  severidad: "error",
                  archivo: archivo.nombre,
                  hoja: "Planilla",
                  celda: "A6",
                  celda_encontrada: "A7",
                  mensaje: "Los títulos de las columnas no están donde deberían.",
                },
                recorte: {
                  hoja: "Planilla",
                  fila_inicio: 5,
                  columna_inicio: 1,
                  celdas: [
                    ["", "", "", ""],
                    ["", "", "", ""],
                    ["APELLIDO PATERNO", "NOMBRES", "TIPO DE DOCUMENTO", "DNI/CEX"],
                    ["Pérez", "Ana", "DNI", "00000001"],
                  ],
                },
              },
            ],
          };
        }
        const conAviso = i === 1;
        return {
          archivo,
          personas: 1200 + i * 16,
          problemas: conAviso
            ? [
                {
                  problema: {
                    codigo: "PRE-21",
                    severidad: "aviso",
                    archivo: archivo.nombre,
                    hoja: "Planilla",
                    celda: null,
                    mensaje: "Trae 2 columnas nuevas de subsidio. Las sumaremos al total automáticamente; no tienes que hacer nada.",
                  },
                  recorte: null,
                },
              ]
            : [],
        };
      });
    },

    async procesar(reporte, rutas, _complementos, _excluir, alAvanzar: (a: Avance) => void): Promise<Resultado> {
      detenido = false;
      // Como el núcleo: se reprocesan también las fuentes guardadas de otros reportes.
      void reporte;
      const pasos: Paso[] = ["leer", "licencias", "planilla", "base_personal", "controles", "guardar"];
      for (const [indice, paso] of pasos.entries()) {
        if (detenido) throw new FalloApp(CANCELADO);
        alAvanzar({ paso, indice, total: pasos.length, pasos });
        await espera(demora);
      }
      if (detenido) throw new FalloApp(CANCELADO);
      if (opciones.errorAlProcesar) throw new FalloApp(opciones.errorAlProcesar);
      const periodos = rutas
        .map((r) => fechaDe("planilla", partir(r).nombre))
        .flatMap((f) => (f && "periodo" in f ? [f.periodo] : []));
      return {
        titulo_periodos: "Septiembre a noviembre de 2025",
        cifras: [
          { valor: "1.239", etiqueta: "personas en la planilla de noviembre" },
          {
            valor: periodos.length === 1 ? "1 mes" : `${periodos.length} meses`,
            etiqueta: periodos.map((p) => ["sep", "oct", "nov"][Number(p.slice(5)) - 9] ?? p).join(" · "),
          },
          { valor: "2", etiqueta: "avisos para revisar" },
        ],
        para_revisar: [
          {
            codigo: "LIC-ENR-04",
            severidad: "aviso",
            archivo: "",
            hoja: null,
            celda: null,
            mensaje: "4 licencias no tienen a la persona en la planilla del mismo periodo.",
          },
          {
            codigo: "PLA-CTL-01",
            severidad: "aviso",
            archivo: "",
            hoja: null,
            celda: null,
            mensaje: "2 personas aparecen dos veces en la planilla de octubre.",
          },
        ],
        ejecucion: {
          marca: "20251117-104200",
          version_motor: "0.1.0",
          fecha_corte: "2025-11-12",
          periodos: ["2025-09", "2025-10", "2025-11"],
          fuentes: [],
          etapas: [
            { nombre: "planilla", estado: "ok", filas: 3715 },
            { nombre: "licencias", estado: "ok", filas: 120 },
            { nombre: "base_personal", estado: "ok", filas: 1284 },
            { nombre: "controles", estado: "aviso", filas: 2 },
          ],
          salidas: [],
          problemas: [],
        },
      };
    },

    async detener() {
      detenido = true;
    },

    async abrir(_que: Abrir) {},
    async abrirTablero(_pestana?: "alertas") {},

    async elegirArchivos(multiple) {
      return multiple
        ? [
            "C:/Usuarios/demo/Documentos/Planillas/Planilla 2025-09 Empleados.xlsx",
            "C:/Usuarios/demo/Documentos/Planillas/Planilla 2025-10 Empleados.xlsx",
            "C:/Usuarios/demo/Escritorio/Planilla 2025-11 Empleados.xlsx",
          ]
        : ["C:/Usuarios/demo/Documentos/BD. 12.11.2025..xlsx"];
    },

    async alSoltarArchivos() {
      return () => {};
    },
  };
}
