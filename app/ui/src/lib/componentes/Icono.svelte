<script lang="ts" module>
  // Íconos de trazo de los mocks v2 (docs/diseno). Decorativos: siempre con texto al lado.
  const TRAZOS = {
    ayuda: '<circle cx="12" cy="12" r="10"/><path d="M9.1 9a3 3 0 0 1 5.8 1c0 2-3 3-3 3"/><path d="M12 17h.01"/>',
    flecha: '<path d="M5 12h14M13 6l6 6-6 6"/>',
    volver: '<path d="M19 12H5M11 18l-6-6 6-6"/>',
    planilla: '<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M3 9h18M8 4v16"/>',
    personas:
      '<circle cx="9" cy="8" r="3.5"/><path d="M2.5 20c.8-3.5 3.4-5.5 6.5-5.5s5.7 2 6.5 5.5"/><circle cx="17.5" cy="9" r="2.5"/><path d="M17 14c2.4.2 4 1.8 4.5 4.5"/>',
    carpeta: '<path d="M3 7h5l2 2h11v10a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2Z"/>',
    subir: '<path d="M12 15V4M7 9l5-5 5 5"/><path d="M4 15v3a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-3"/>',
    excel: '<path d="M14 3H6a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9Z"/><path d="M14 3v6h6M9 13l4 5M13 13l-4 5"/>',
    cerrar: '<path d="M6 6l12 12M18 6 6 18"/>',
    check: '<path d="M5 12l5 5 9-10"/>',
    aviso: '<path d="M12 8v5M12 16.5h.01"/><circle cx="12" cy="12" r="9"/>',
    triangulo:
      '<path d="M12 8v5M12 16.5h.01"/><path d="M10.3 3.9 1.8 18a2 2 0 0 0 1.7 3h17a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0Z"/>',
    externo: '<path d="M14 3h7v7M21 3l-9 9"/><path d="M19 14v5a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V7a2 2 0 0 1 2-2h5"/>',
    recargar: '<path d="M21 12a9 9 0 1 1-3-6.7L21 8"/><path d="M21 3v5h-5"/>',
    celda: '<rect x="3" y="3" width="18" height="18" rx="2"/><path d="M3 9h18M9 3v18"/>',
    grafico:
      '<path d="M3 3v18h18"/><rect x="7" y="12" width="3" height="5"/><rect x="12" y="8" width="3" height="9"/><rect x="17" y="5" width="3" height="12"/>',
  } as const;
  export type NombreIcono = keyof typeof TRAZOS;
  export function trazo(nombre: NombreIcono): string {
    return TRAZOS[nombre];
  }
</script>

<script lang="ts">
  let { nombre, tamano = 20, grosor = 2 }: { nombre: NombreIcono; tamano?: number; grosor?: number } = $props();
</script>

<svg
  width={tamano}
  height={tamano}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width={grosor}
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  {@html trazo(nombre)}
</svg>
