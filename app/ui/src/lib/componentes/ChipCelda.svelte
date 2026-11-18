<script lang="ts">
  // Elemento distintivo del diseño: la referencia de celda como en Excel.
  import Icono from "./Icono.svelte";

  let { hoja = null, celda, conIcono = true }: { hoja?: string | null; celda: string; conIcono?: boolean } = $props();
</script>

<span class="celda">
  {#if conIcono}<Icono nombre="celda" tamano={14} />{/if}
  {#if hoja}Hoja «{hoja}» · celda {celda}{:else}{celda}{/if}
</span>

<style>
  .celda {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-family: var(--fuente-mono);
    font-size: 14px;
    font-weight: 500;
    padding: 2px 8px;
    border-radius: 6px;
    background: var(--color-superficie);
    border: 1px solid #b9c4bc;
    color: var(--color-texto);
    white-space: nowrap;
    vertical-align: baseline;
  }
</style>
