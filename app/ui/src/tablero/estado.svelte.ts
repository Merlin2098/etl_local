// Estado del tablero (Plan 7): filtros en la URL, datos de la API y pestaña activa. Recibe el cliente y la
// URL inyectados, como `Flujo` en el Plan 6, para probarlo sin navegador.
import type {
  Estado,
  Filtros,
  OpcionesFiltro,
  RespuestaChecks,
  RespuestaDetalle,
  RespuestaIndicadores,
  RespuestaSeries,
} from "../lib/api/tipos.gen";
import { type ClienteApi, FalloApi, consulta } from "./api";

export type Pestana = "resumen" | "alertas" | "como-leer";
export type ClaveFiltro = "periodo" | "gerencia" | "sede" | "tipo_trabajador";

export interface DatosResumen {
  estado: Estado;
  opciones: OpcionesFiltro;
  indicadores: RespuestaIndicadores;
  series: RespuestaSeries;
  checks: RespuestaChecks;
}

export interface Url {
  leer(): string;
  escribir(q: string): void;
}

const CLAVES: ClaveFiltro[] = ["periodo", "gerencia", "sede", "tipo_trabajador"];

function desdeUrl(q: string): Filtros {
  const p = new URLSearchParams(q);
  const f: Filtros = {};
  for (const c of CLAVES) {
    const v = p.get(c);
    if (v) f[c] = v;
  }
  return f;
}

function aFalloApi(e: unknown): FalloApi {
  return e instanceof FalloApi ? e : new FalloApi("interno", String(e), 0);
}

export class EstadoTablero {
  filtros = $state<Filtros>({});
  pestana = $state<Pestana>("resumen");
  datos = $state<DatosResumen | null>(null);
  detalle = $state<{ alertas: RespuestaDetalle; sinBase: RespuestaDetalle } | null>(null);
  vacio = $state(false);
  error = $state<FalloApi | null>(null);
  aviso = $state<string | null>(null);
  cargando = $state(false);

  #cliente: ClienteApi;
  #url: Url;
  #esperar: (ms: number) => Promise<void>;

  constructor(cliente: ClienteApi, url: Url, esperar = (ms: number) => new Promise<void>((r) => setTimeout(r, ms))) {
    this.#cliente = cliente;
    this.#url = url;
    this.#esperar = esperar;
    this.filtros = desdeUrl(url.leer());
  }

  get cliente(): ClienteApi {
    return this.#cliente;
  }

  /** Número del último pedido: una respuesta de un pedido anterior se descarta (revisión final del Plan 7). */
  #version = 0;

  /** Pide los datos de la pestaña activa. Reintenta una vez si `actual/` se está reemplazando (foco 5) y
   *  quita los filtros que ya no existen (foco 4). Si mientras tanto empieza otro pedido, este no escribe nada. */
  async cargar(): Promise<void> {
    const n = ++this.#version;
    const vigente = () => n === this.#version;
    this.cargando = true;
    try {
      await this.#conReintentos(vigente);
      if (!vigente()) return;
      this.error = null;
      this.vacio = false;
    } catch (e) {
      if (!vigente()) return;
      const f = aFalloApi(e);
      if (f.codigo === "sin_datos") {
        this.vacio = true;
        this.error = null;
        this.datos = null;
      } else {
        this.error = f;
      }
    } finally {
      if (vigente()) this.cargando = false;
    }
  }

  async #conReintentos(vigente: () => boolean): Promise<void> {
    let actualizando = 1;
    let filtroInvalido = 1;
    for (;;) {
      try {
        await this.#pedir(vigente);
        return;
      } catch (e) {
        if (!vigente()) return;
        const f = aFalloApi(e);
        if (f.codigo === "actualizando" && actualizando-- > 0) {
          await this.#esperar(1000);
        } else if (f.codigo === "filtro_invalido" && filtroInvalido-- > 0) {
          await this.#quitarFiltrosInvalidos(f, vigente);
        } else {
          throw f;
        }
      }
    }
  }

  async #pedir(vigente: () => boolean): Promise<void> {
    const f = $state.snapshot(this.filtros);
    const [estado, opciones, indicadores, series, checks] = await Promise.all([
      this.#cliente.estado(),
      this.#cliente.filtros(),
      this.#cliente.indicadores(f),
      this.#cliente.series(f),
      this.#cliente.checks(f),
    ]);
    if (!vigente()) return;
    this.datos = { estado, opciones, indicadores, series, checks };
    if (this.pestana === "alertas") await this.#pedirDetalle(f, vigente);
  }

  async #pedirDetalle(f: Filtros, vigente: () => boolean): Promise<void> {
    const [alertas, sinBase] = await Promise.all([this.#cliente.detalle("alertas", f), this.#cliente.detalle("sin-base", f)]);
    if (vigente()) this.detalle = { alertas, sinBase };
  }

  async #quitarFiltrosInvalidos(fallo: FalloApi, vigente: () => boolean): Promise<void> {
    const o = await this.#cliente.filtros();
    if (!vigente()) return;
    const f = this.filtros;
    const validos: Record<ClaveFiltro, string[]> = {
      periodo: o.periodos,
      gerencia: o.gerencias,
      sede: o.sedes,
      tipo_trabajador: o.tipos_trabajador,
    };
    const invalido = CLAVES.map((c) => f[c]).find((v, i) => v && !validos[CLAVES[i]!].includes(v));
    const periodo = f.periodo && o.periodos.includes(f.periodo) ? f.periodo : null;
    this.#fijar(periodo ? { periodo } : {});
    this.aviso = invalido
      ? `El filtro «${invalido}» ya no existe en los datos; se muestran todos.`
      : fallo.message;
  }

  #fijar(f: Filtros): void {
    this.filtros = f;
    this.#url.escribir(consulta(f));
  }

  async cambiarFiltro(clave: ClaveFiltro, valor: string | null): Promise<void> {
    const f: Filtros = { ...$state.snapshot(this.filtros) };
    if (valor) f[clave] = valor;
    else delete f[clave];
    this.#fijar(f);
    this.aviso = null;
    await this.cargar();
  }

  async irA(pestana: Pestana): Promise<void> {
    this.pestana = pestana;
    if (pestana === "alertas" && !this.vacio) {
      const n = this.#version;
      const vigente = () => n === this.#version && this.pestana === "alertas";
      try {
        await this.#pedirDetalle($state.snapshot(this.filtros), vigente);
      } catch (e) {
        if (vigente()) this.error = aFalloApi(e);
      }
    }
  }
}
