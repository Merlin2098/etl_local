// Estado del flujo de actualización (pantallas 1–5). Recibe el backend inyectado: real (Tauri) o simulado.

import { nombreMes } from "../lib/formato";
import { aErrorApp, type Backend } from "./backend";
import { tieneError } from "./textos";
import type {
  ArchivoElegido,
  Avance,
  ErrorApp,
  Inicio,
  Paso,
  Reporte,
  Resultado,
  RevisionArchivo,
} from "./tipos";
import { textoFecha } from "./tipos";

export type Pantalla = "inicio" | "archivos" | "revision" | "procesando" | "listo";

const EXTENSIONES = [".xlsx", ".xlsm", ".xls"];

function nombreDe(ruta: string): string {
  return ruta.split(/[\\/]/).pop() ?? ruta;
}

function esExcel(ruta: string): boolean {
  const r = ruta.toLowerCase();
  return EXTENSIONES.some((e) => r.endsWith(e));
}

export interface Complemento {
  usar: boolean;
  archivo: ArchivoElegido | null;
  /** El archivo lo eligió el usuario ahora (no es el guardado). */
  nuevo: boolean;
}

interface Pedido {
  rutas: string[];
  complementos: string[];
  excluir: string[];
}

export class Flujo {
  pantalla = $state<Pantalla>("inicio");
  inicio = $state<Inicio | null>(null);
  reporte = $state<Reporte | null>(null);
  elegidos = $state<ArchivoElegido[]>([]);
  complemento = $state<Complemento>({ usar: false, archivo: null, nuevo: false });
  revisiones = $state<RevisionArchivo[]>([]);
  revisando = $state(false);
  avance = $state<Avance | null>(null);
  pasos = $state<Paso[]>([]);
  inicioProceso = $state(0);
  resultado = $state<Resultado | null>(null);
  error = $state<ErrorApp | null>(null);
  aviso = $state<string | null>(null);
  /** El reporte ya se había cargado antes de este proceso: hay una versión anterior en el histórico. */
  huboAnterior = $state(false);

  #backend: Backend;
  #reloj: () => number;
  #ultimoPedido: Pedido | null = null;

  constructor(backend: Backend, reloj: () => number = Date.now) {
    this.#backend = backend;
    this.#reloj = reloj;
  }

  get backend(): Backend {
    return this.#backend;
  }

  get ahora(): number {
    return this.#reloj();
  }

  /** Duración de la última ejecución de este reporte, si existe (para el tiempo estimado). */
  get duracionPrevista(): number | null {
    const id = this.reporte?.id;
    return this.inicio?.tarjetas.find((t) => t.estado.reporte.id === id)?.duracion_prevista_ms ?? null;
  }

  async cargarInicio(): Promise<void> {
    try {
      this.inicio = await this.#backend.inicio();
      this.error = null;
    } catch (e) {
      this.error = aErrorApp(e);
    }
  }

  #limpiar(): void {
    this.elegidos = [];
    this.complemento = { usar: false, archivo: null, nuevo: false };
    this.revisiones = [];
    this.avance = null;
    this.pasos = [];
    this.resultado = null;
    this.error = null;
    this.#ultimoPedido = null;
  }

  async elegirReporte(id: string): Promise<void> {
    const reporte = this.inicio?.tarjetas.find((t) => t.estado.reporte.id === id)?.estado.reporte;
    if (!reporte) return;
    this.#limpiar();
    this.aviso = null;
    this.reporte = reporte;
    const fuente = reporte.complementos?.[0];
    if (fuente) {
      try {
        const guardado = await this.#backend.complementoGuardado(fuente);
        this.complemento = { usar: guardado !== null, archivo: guardado, nuevo: false };
      } catch (e) {
        this.error = aErrorApp(e);
      }
    }
    this.pantalla = "archivos";
  }

  async agregar(rutas: string[]): Promise<void> {
    const reporte = this.reporte;
    if (!reporte) return;
    const excel = rutas.filter(esExcel);
    const ignorados = rutas.filter((r) => !esExcel(r)).map(nombreDe);
    const avisos: string[] = [];
    if (ignorados.length > 0) {
      avisos.push(
        ignorados.length === 1
          ? `«${ignorados[0]}» no es un archivo de Excel; no lo agregamos.`
          : `${ignorados.map((n) => `«${n}»`).join(", ")} no son archivos de Excel; no los agregamos.`,
      );
    }
    let nuevas = excel.filter((r) => !this.elegidos.some((a) => a.ruta === r));
    if (!reporte.varios_archivos && nuevas.length > 0) {
      const ultima = nuevas[nuevas.length - 1]!;
      if (nuevas.length > 1 || this.elegidos.length > 0) {
        avisos.push(`La ${reporte.titulo.toLowerCase()} es un solo archivo; usamos «${nombreDe(ultima)}».`);
      }
      nuevas = [ultima];
    }
    this.aviso = avisos.length > 0 ? avisos.join(" ") : null;
    if (nuevas.length === 0) return;
    try {
      const reconocidos = await this.#backend.reconocer(reporte.id, nuevas);
      this.elegidos = reporte.varios_archivos ? [...this.elegidos, ...reconocidos] : reconocidos;
    } catch (e) {
      this.error = aErrorApp(e);
    }
  }

  quitar(ruta: string): void {
    this.elegidos = this.elegidos.filter((a) => a.ruta !== ruta);
  }

  async cambiarComplemento(ruta: string): Promise<void> {
    const reporte = this.reporte;
    if (!reporte || !esExcel(ruta)) return;
    try {
      const [archivo] = await this.#backend.reconocer(reporte.id, [ruta]);
      if (archivo) this.complemento = { usar: true, archivo, nuevo: true };
    } catch (e) {
      this.error = aErrorApp(e);
    }
  }

  get puedeRevisar(): { ok: boolean; motivo: string | null } {
    if (this.elegidos.length === 0) return { ok: false, motivo: "Elige al menos un archivo." };
    const varios = this.reporte?.varios_archivos ?? true;
    const sinFecha = this.elegidos.find((a) => a.fecha_nombre === null);
    if (sinFecha) {
      const que = varios ? "el mes" : "la fecha de corte";
      return { ok: false, motivo: `No reconocemos ${que} de «${sinFecha.nombre}». Revisa el nombre del archivo.` };
    }
    const vistos = new Set<string>();
    for (const a of this.elegidos) {
      const f = textoFecha(a.fecha_nombre) ?? "";
      if (vistos.has(f)) return { ok: false, motivo: `Hay dos archivos de ${nombreMes(f)}; quita uno.` };
      vistos.add(f);
    }
    return { ok: true, motivo: null };
  }

  #complementoARevisar(): string[] {
    const c = this.complemento;
    return c.usar && c.nuevo && c.archivo ? [c.archivo.ruta] : [];
  }

  async revisar(): Promise<void> {
    const reporte = this.reporte;
    if (!reporte || !this.puedeRevisar.ok) return;
    this.revisando = true;
    try {
      this.revisiones = await this.#backend.revisar(
        reporte.id,
        this.elegidos.map((a) => a.ruta),
        this.#complementoARevisar(),
      );
      this.error = null;
      this.pantalla = "revision";
    } catch (e) {
      this.error = aErrorApp(e);
    } finally {
      this.revisando = false;
    }
  }

  revisarDeNuevo(): Promise<void> {
    return this.revisar();
  }

  get puedeProcesar(): { ok: boolean; motivo: string | null } {
    if (this.revisiones.some(tieneError)) {
      return { ok: false, motivo: "Se activa cuando todos los archivos estén en orden." };
    }
    return { ok: true, motivo: null };
  }

  #conError(): Set<string> {
    return new Set(this.revisiones.filter(tieneError).map((r) => r.archivo.ruta));
  }

  /** Archivos elegidos sin error (para «Seguir sin …»). */
  get sinLosConError(): ArchivoElegido[] {
    const malos = this.#conError();
    return this.elegidos.filter((a) => !malos.has(a.ruta));
  }

  /** El complemento nuevo con error también queda fuera. */
  #pedido(rutas: string[]): Pedido {
    const c = this.complemento;
    const fuente = this.reporte?.complementos?.[0];
    const malos = this.#conError();
    const complementos =
      c.usar && c.nuevo && c.archivo && !malos.has(c.archivo.ruta) ? [c.archivo.ruta] : [];
    const excluir = fuente && c.archivo !== null && !c.usar ? [fuente] : [];
    return { rutas, complementos, excluir };
  }

  async seguirSin(): Promise<void> {
    const rutas = this.sinLosConError.map((a) => a.ruta);
    if (rutas.length === 0) return;
    await this.#ejecutar(this.#pedido(rutas));
  }

  async procesar(): Promise<void> {
    if (!this.puedeProcesar.ok) return;
    await this.#ejecutar(this.#pedido(this.elegidos.map((a) => a.ruta)));
  }

  async reintentar(): Promise<void> {
    if (this.#ultimoPedido) await this.#ejecutar(this.#ultimoPedido);
  }

  /** Recibe cada avance del backend (se puede envolver en los tests). */
  alAvanzar(a: Avance): void {
    this.avance = a;
    if (a.pasos.length > 0) this.pasos = a.pasos;
  }

  async #ejecutar(pedido: Pedido): Promise<void> {
    const reporte = this.reporte;
    if (!reporte) return;
    this.#ultimoPedido = pedido;
    this.pantalla = "procesando";
    this.error = null;
    this.avance = null;
    this.pasos = [];
    this.huboAnterior =
      this.inicio?.tarjetas.find((t) => t.estado.reporte.id === reporte.id)?.estado.vigencia !== "nunca";
    this.inicioProceso = this.#reloj();
    try {
      this.resultado = await this.#backend.procesar(
        reporte.id,
        pedido.rutas,
        pedido.complementos,
        pedido.excluir,
        (a) => this.alAvanzar(a),
      );
      this.pantalla = "listo";
    } catch (e) {
      const app = aErrorApp(e);
      if (app.codigo === "cancelado") {
        this.#limpiar();
        this.aviso = app.mensaje;
        this.pantalla = "inicio";
        await this.cargarInicio();
      } else {
        this.error = app;
      }
    }
  }

  /** Lo que se está procesando, para nombrar los pasos (sin los meses dejados fuera con «Seguir sin …»). */
  get contextoPasos(): { archivos: number; periodos: string[] } {
    const rutas = this.#ultimoPedido?.rutas ?? this.elegidos.map((a) => a.ruta);
    const usados = this.elegidos.filter((a) => rutas.includes(a.ruta));
    return {
      archivos: usados.length || 1,
      periodos: usados.map((a) => textoFecha(a.fecha_nombre) ?? "").filter((p) => p.length === 7),
    };
  }

  /** Fuente del complemento si `ruta` es el complemento elegido (para nombrarlo «Control de licencias»). */
  fuenteDe(ruta: string): string | undefined {
    return this.complemento.archivo?.ruta === ruta ? this.reporte?.complementos?.[0] : undefined;
  }

  /** Plan 7: el tablero necesita algún reporte procesado. */
  get puedeVerTablero(): { ok: boolean; motivo: string | null } {
    const alguno = this.inicio?.tarjetas.some((t) => t.estado.vigencia !== "nunca") ?? false;
    return alguno ? { ok: true, motivo: null } : { ok: false, motivo: "Procesa un reporte primero." };
  }

  async verTablero(pestana?: "alertas"): Promise<void> {
    try {
      await this.#backend.abrirTablero(pestana);
    } catch (e) {
      this.error = aErrorApp(e);
    }
  }

  async detener(): Promise<void> {
    try {
      await this.#backend.detener();
    } catch (e) {
      this.error = aErrorApp(e);
    }
  }

  async volverAlInicio(): Promise<void> {
    this.#limpiar();
    this.aviso = null;
    this.reporte = null;
    this.pantalla = "inicio";
    await this.cargarInicio();
  }
}
