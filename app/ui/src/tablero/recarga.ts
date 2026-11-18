// Decisión 3 del plan: en vez de «Actualizar», el tablero vuelve a pedir los datos cuando la ventana recupera
// el foco (dos ventanas visibles: la app y el tablero) o vuelve a ser visible. Un solo pedido por regreso.

export function escucharRetorno(
  ventana: EventTarget,
  documento: EventTarget & { visibilityState: DocumentVisibilityState },
  recargar: () => void,
  ahora: () => number = Date.now,
  separacionMs = 2000,
): () => void {
  let ultima = Number.NEGATIVE_INFINITY;
  const alVolver = () => {
    if (documento.visibilityState !== "visible") return;
    const t = ahora();
    if (t - ultima < separacionMs) return;
    ultima = t;
    recargar();
  };
  ventana.addEventListener("focus", alVolver);
  documento.addEventListener("visibilitychange", alVolver);
  return () => {
    ventana.removeEventListener("focus", alVolver);
    documento.removeEventListener("visibilitychange", alVolver);
  };
}
