// Formatos en español para la UI (diseno §Lenguaje visual: hablar el idioma de Excel).

const MESES = [
  "enero",
  "febrero",
  "marzo",
  "abril",
  "mayo",
  "junio",
  "julio",
  "agosto",
  "septiembre",
  "octubre",
  "noviembre",
  "diciembre",
] as const;

const MESES_CORTOS = ["ene", "feb", "mar", "abr", "may", "jun", "jul", "ago", "sep", "oct", "nov", "dic"] as const;

function mayuscula(texto: string): string {
  return texto.charAt(0).toUpperCase() + texto.slice(1);
}

function partes(iso: string): { anio: string; mes: number; dia: number } {
  const [anio = "", mes = "1", dia = "1"] = iso.split("-");
  return { anio, mes: Number(mes), dia: Number(dia) };
}

/** Mes en minúscula: `"2025-09"` → `"septiembre"`. */
export function mesMinuscula(periodo: string): string {
  return MESES[partes(periodo).mes - 1] ?? periodo;
}

/** Mes abreviado: `"2025-09"` → `"sep"`. */
export function mesCorto(periodo: string): string {
  return MESES_CORTOS[partes(periodo).mes - 1] ?? periodo;
}

/** `"2025-09"` → `"Septiembre 2025"`. */
export function nombreMes(periodo: string): string {
  return `${mayuscula(mesMinuscula(periodo))} ${partes(periodo).anio}`;
}

/** `"2025-11-12"` → `"12 nov 2025"`. */
export function fechaCorta(iso: string): string {
  const { anio, mes, dia } = partes(iso);
  return `${dia} ${MESES_CORTOS[mes - 1] ?? mes} ${anio}`;
}

/** Miles con `.` (también con 4 cifras) y decimales con `,`. `es-ES` no agrupa 4 cifras, por eso no se usa Intl. */
export function numero(n: number): string {
  const signo = n < 0 ? "-" : "";
  const [entera = "0", decimales] = String(Math.abs(n)).split(".");
  const conMiles = entera.replace(/\B(?=(\d{3})+(?!\d))/g, ".");
  return signo + conMiles + (decimales ? `,${decimales}` : "");
}

/** Saludo según la hora local (0–23). */
export function saludo(hora: number): string {
  if (hora < 12) return "Buenos días";
  if (hora < 19) return "Buenas tardes";
  return "Buenas noches";
}

/** `["2025-09","2025-10","2025-11"]` → `"septiembre, octubre y noviembre"`. */
export function listaMeses(periodos: string[]): string {
  const nombres = periodos.map(mesMinuscula);
  if (nombres.length <= 1) return nombres[0] ?? "";
  return `${nombres.slice(0, -1).join(", ")} y ${nombres[nombres.length - 1]}`;
}
