// Colores y tipografía de los gráficos, con los valores de docs/diseno §Lenguaje visual (los mismos de
// tokens.css; ECharts dibuja en SVG y no lee variables CSS). Modo oscuro: pendiente en diseno.

export const COLORES = {
  entraron: "#2B6CB0",
  salieron: "#C2620F",
  unica: "#1E6B47",
} as const;

export const TEMA = {
  fuente: '"Atkinson Hyperlegible", system-ui, sans-serif',
  texto: "#1B2420",
  texto2: "#55605A",
  borde: "#C9D0C8",
  rejilla: "#EEF0EC",
} as const;
