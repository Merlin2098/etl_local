// Cliente de la API local del tablero (ADR-0004). Solo `fetch` a `/api/v1/`: nada de Tauri, para que el
// mismo código sirva en el navegador y en la ventana de la app.
import type {
  Detalle,
  ErrorApi,
  Estado,
  Filtros,
  OpcionesFiltro,
  RespuestaChecks,
  RespuestaDetalle,
  RespuestaIndicadores,
  RespuestaSeries,
} from "../lib/api/tipos.gen";

/** Error de la API con su `codigo` (`sin_datos`, `filtro_invalido`, `actualizando`, …) y el estado HTTP. */
export class FalloApi extends Error {
  constructor(
    readonly codigo: string,
    mensaje: string,
    readonly estado: number,
  ) {
    super(mensaje);
    this.name = "FalloApi";
  }
}

export interface ClienteApi {
  estado(): Promise<Estado>;
  filtros(): Promise<OpcionesFiltro>;
  indicadores(f: Filtros): Promise<RespuestaIndicadores>;
  series(f: Filtros): Promise<RespuestaSeries>;
  checks(f: Filtros): Promise<RespuestaChecks>;
  detalle(tipo: Detalle, f: Filtros): Promise<RespuestaDetalle>;
}

/** Consulta con los filtros que tienen valor, en orden fijo. */
export function consulta(f: Filtros): string {
  const q = new URLSearchParams();
  for (const clave of ["periodo", "gerencia", "sede", "tipo_trabajador"] as const) {
    const v = f[clave];
    if (v) q.set(clave, v);
  }
  const texto = q.toString();
  return texto ? `?${texto}` : "";
}

export function crearCliente(base = "", pedir: typeof fetch = (...a) => fetch(...a)): ClienteApi {
  async function obtener<T>(ruta: string, f: Filtros = {}): Promise<T> {
    let resp: Response;
    try {
      resp = await pedir(`${base}/api/v1/${ruta}${consulta(f)}`);
    } catch (e) {
      throw new FalloApi("sin_conexion", `No se pudo conectar con el tablero: ${String(e)}`, 0);
    }
    const cuerpo: unknown = await resp.json().catch(() => null);
    if (!resp.ok) {
      const e = cuerpo as Partial<ErrorApi> | null;
      throw new FalloApi(e?.codigo ?? "interno", e?.mensaje ?? `Error ${resp.status}`, resp.status);
    }
    return cuerpo as T;
  }
  return {
    estado: () => obtener("estado"),
    filtros: () => obtener("filtros"),
    indicadores: (f) => obtener("indicadores", f),
    series: (f) => obtener("series", f),
    checks: (f) => obtener("checks", f),
    detalle: (tipo, f) => obtener(`detalle/${tipo}`, f),
  };
}
