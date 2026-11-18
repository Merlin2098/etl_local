import "../lib/estilos/fuentes.css";
import "../lib/estilos/tokens.css";
import "../lib/estilos/base.css";
import "./impresion.css";

import { mount } from "svelte";
import Tablero from "./Tablero.svelte";
import { crearCliente } from "./api";
import { EstadoTablero } from "./estado.svelte";
import { escucharRetorno } from "./recarga";

const destino = document.getElementById("app");
if (!destino) throw new Error("Falta el elemento #app");

// Los filtros viven en la consulta de la URL (enlaces y PDF reproducibles); la pestaña, en el hash.
const estado = new EstadoTablero(crearCliente(), {
  leer: () => location.search,
  escribir: (q) => history.replaceState(null, "", `${location.pathname}${q}${location.hash}`),
});
if (import.meta.env.DEV) (window as unknown as { __tablero: EstadoTablero }).__tablero = estado;

// Decisión 3: en vez de «Actualizar», volver a pedir los datos cuando la ventana recupera el foco o la visibilidad.
escucharRetorno(window, document, () => void estado.cargar());

export default mount(Tablero, { target: destino, props: { estado } });
