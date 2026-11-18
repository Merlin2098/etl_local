import "../lib/estilos/fuentes.css";
import "../lib/estilos/tokens.css";
import "../lib/estilos/base.css";

import { mount } from "svelte";
import App from "./App.svelte";
import { type Backend, backendTauri } from "./backend";
import { Flujo } from "./flujo.svelte";

async function elegirBackend(): Promise<Backend> {
  const parametros = new URLSearchParams(location.search);
  // Solo en desarrollo: `app.html?simulado` (con `&error` para la cabecera desplazada). El simulado no
  // entra en el build de producción porque `import.meta.env.DEV` es `false` allí.
  if (import.meta.env.DEV && parametros.has("simulado")) {
    const { backendSimulado } = await import("./simulado");
    return backendSimulado({ conError: parametros.has("error") });
  }
  return backendTauri();
}

const destino = document.getElementById("app");
if (!destino) throw new Error("Falta el elemento #app");

const flujo = new Flujo(await elegirBackend());
if (import.meta.env.DEV) (window as unknown as { __flujo: Flujo }).__flujo = flujo;

export default mount(App, { target: destino, props: { flujo } });
