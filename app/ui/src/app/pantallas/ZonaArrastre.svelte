<script lang="ts">
  import { onMount } from "svelte";
  import Boton from "../../lib/componentes/Boton.svelte";
  import Icono from "../../lib/componentes/Icono.svelte";
  import type { Flujo } from "../flujo.svelte";

  let { flujo }: { flujo: Flujo } = $props();

  onMount(() => {
    let desuscribir: (() => void) | null = null;
    let vivo = true;
    void flujo.backend.alSoltarArchivos((rutas) => void flujo.agregar(rutas)).then((f) => {
      if (vivo) desuscribir = f;
      else f();
    });
    return () => {
      vivo = false;
      desuscribir?.();
    };
  });

  async function buscar() {
    const rutas = await flujo.backend.elegirArchivos(flujo.reporte?.varios_archivos ?? true);
    await flujo.agregar(rutas);
  }
</script>

<div class="zona">
  <Icono nombre="subir" tamano={44} grosor={1.8} />
  <p class="h2">Arrastra aquí tus archivos</p>
  <p class="muted">o</p>
  <Boton onclick={buscar}><strong>Buscar en mi computadora</strong></Boton>
  <p class="muted pequeno">Archivos de Excel (.xlsx, .xlsm, .xls)</p>
</div>

<style>
  .zona {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    text-align: center;
    padding: 36px 24px;
    border: 2px dashed #8fb8a1;
    border-radius: 18px;
    background: var(--color-superficie);
    color: var(--color-accion);
  }
  p {
    margin: 0;
  }
  .h2 {
    font-family: var(--fuente-titulo);
    font-weight: 700;
    font-size: 21px;
    color: var(--color-texto);
  }
  .pequeno {
    font-size: 15px;
  }
</style>
