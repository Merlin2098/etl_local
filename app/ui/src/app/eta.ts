/** Tiempo restante estimado con la duración de la última ejecución del mismo reporte. `null`: no se muestra. */
export function textoEta(previstaMs: number | null, transcurridoMs: number): string | null {
  if (previstaMs === null) return null;
  const restanteMs = previstaMs - transcurridoMs;
  if (restanteMs < 0) return "Está tardando más de lo habitual";
  if (restanteMs < 5000) return "Falta poco";
  const segundos = Math.ceil(restanteMs / 1000 / 5) * 5;
  return `Quedan unos ${segundos} segundos`;
}
