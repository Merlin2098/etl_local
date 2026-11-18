<script lang="ts">
  // «Datos usados» (TR-D16): de qué archivos sale lo que se ve y si hay periodos mezclados (IND-C2).
  import Icono from "../lib/componentes/Icono.svelte";
  import type { Estado } from "../lib/api/tipos.gen";
  import { fuentesUnicas, textoFuente } from "./textos";

  let { estado, periodo }: { estado: Estado; periodo: string | null | undefined } = $props();
  const fuentes = $derived(fuentesUnicas(estado.fuentes));
</script>

<div class="datos" role="status">
  <strong>Datos usados:</strong>
  {#each fuentes as f (f.fuente)}<span class="fuente">{textoFuente(f, periodo)}</span>{/each}
  {#if estado.periodos_mezclados && estado.aviso_periodos}
    <span class="mezcla"><Icono nombre="triangulo" tamano={18} /><strong>Aviso:</strong> {estado.aviso_periodos}</span>
  {/if}
</div>

<style>
  .datos {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px 16px;
    padding: 12px 16px;
    background: var(--color-superficie);
    border: 1px solid var(--color-borde);
    border-radius: var(--radio);
    font-size: 15px;
  }
  .fuente {
    color: var(--color-texto-2);
  }
  .mezcla {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 2px 10px;
    border-radius: 8px;
    background: var(--color-aviso-fondo);
    color: var(--color-aviso-texto);
  }
</style>
