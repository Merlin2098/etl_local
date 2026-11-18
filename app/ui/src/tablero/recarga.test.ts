import { expect, test } from "vitest";
import { escucharRetorno } from "./recarga";

// Revisión final: el tablero vuelve a pedir los datos al recuperar el foco (dos ventanas visibles) y al volver
// a ser visible, sin pedir dos veces por el mismo regreso.
test("recarga al volver el foco o la visibilidad, una vez por regreso", () => {
  const ventana = new EventTarget();
  const documento = Object.assign(new EventTarget(), { visibilityState: "visible" as DocumentVisibilityState });
  let veces = 0;
  let ahora = 0;
  escucharRetorno(ventana, documento, () => void veces++, () => ahora);
  ventana.dispatchEvent(new Event("focus"));
  expect(veces).toBe(1);
  ahora = 100;
  documento.dispatchEvent(new Event("visibilitychange"));
  expect(veces).toBe(1); // mismo regreso: focus y visibilitychange juntos
  ahora = 5000;
  ventana.dispatchEvent(new Event("focus"));
  expect(veces).toBe(2);
  documento.visibilityState = "hidden";
  ahora = 10000;
  documento.dispatchEvent(new Event("visibilitychange"));
  expect(veces).toBe(2);
});
