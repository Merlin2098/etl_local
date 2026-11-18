// Acceso al backend: el real (Tauri) y la interfaz que también implementa el simulado (tests y revisión visual).

import { Channel, invoke } from "@tauri-apps/api/core";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open } from "@tauri-apps/plugin-dialog";
import type { Abrir, ArchivoElegido, Avance, ErrorApp, Inicio, Resultado, RevisionArchivo } from "./tipos";

export interface Backend {
  inicio(): Promise<Inicio>;
  reconocer(reporte: string, rutas: string[]): Promise<ArchivoElegido[]>;
  complementoGuardado(fuente: string): Promise<ArchivoElegido | null>;
  /** `complementos`: archivos de las fuentes complementarias (por ejemplo, licencias), revisados con su contrato. */
  revisar(reporte: string, rutas: string[], complementos: string[]): Promise<RevisionArchivo[]>;
  procesar(
    reporte: string,
    rutas: string[],
    complementos: string[],
    excluir: string[],
    alAvanzar: (a: Avance) => void,
  ): Promise<Resultado>;
  detener(): Promise<void>;
  abrir(que: Abrir): Promise<void>;
  /** Plan 7: abre (o enfoca) la ventana del tablero. */
  abrirTablero(pestana?: "alertas"): Promise<void>;
  elegirArchivos(multiple: boolean): Promise<string[]>;
  alSoltarArchivos(cb: (rutas: string[]) => void): Promise<() => void>;
}

/** Error que devuelve un comando, con el `ErrorApp` del backend. */
export class FalloApp extends Error {
  constructor(public app: ErrorApp) {
    super(app.mensaje);
    this.name = "FalloApp";
  }
}

/** Convierte cualquier error en `ErrorApp` (los comandos rechazan con `ErrorApp`; el resto es interno). */
export function aErrorApp(e: unknown): ErrorApp {
  if (e instanceof FalloApp) return e.app;
  if (typeof e === "object" && e !== null && "codigo" in e && "mensaje" in e) {
    const x = e as Partial<ErrorApp>;
    return { codigo: String(x.codigo), mensaje: String(x.mensaje), problemas: x.problemas ?? [] };
  }
  return { codigo: "interno", mensaje: e instanceof Error ? e.message : String(e), problemas: [] };
}

async function llamar<T>(comando: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(comando, args);
  } catch (e) {
    throw new FalloApp(aErrorApp(e));
  }
}

export function backendTauri(): Backend {
  return {
    inicio: () => llamar("inicio"),
    reconocer: (reporte, rutas) => llamar("reconocer", { reporte, rutas }),
    complementoGuardado: (fuente) => llamar("complemento_guardado", { fuente }),
    revisar: (reporte, rutas, complementos) => llamar("revisar", { reporte, rutas, complementos }),
    procesar: (reporte, rutas, complementos, excluir, alAvanzar) => {
      const avance = new Channel<Avance>();
      avance.onmessage = alAvanzar;
      return llamar("procesar", { reporte, rutas, complementos, excluir, avance });
    },
    detener: () => llamar("detener"),
    abrir: (que) => llamar("abrir", { que }),
    abrirTablero: (pestana) => llamar("abrir_tablero", { pestana: pestana ?? null }),
    elegirArchivos: async (multiple) => {
      const elegido = await open({
        multiple,
        directory: false,
        filters: [{ name: "Archivos de Excel", extensions: ["xlsx", "xlsm", "xls"] }],
      });
      if (elegido === null) return [];
      return Array.isArray(elegido) ? elegido : [elegido];
    },
    alSoltarArchivos: async (cb) =>
      getCurrentWebview().onDragDropEvent((evento) => {
        if (evento.payload.type === "drop") cb(evento.payload.paths);
      }),
  };
}
