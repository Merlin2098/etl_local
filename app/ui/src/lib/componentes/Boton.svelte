<script lang="ts">
  // Un botón desactivado siempre dice por qué (diseno §Principios 2): el motivo va debajo, enlazado.
  import type { Snippet } from "svelte";

  let {
    variante = "secundario",
    deshabilitado = false,
    motivo = null,
    onclick,
    children,
    etiqueta,
  }: {
    variante?: "principal" | "secundario" | "enlace";
    deshabilitado?: boolean;
    motivo?: string | null;
    onclick?: () => void;
    children: Snippet;
    etiqueta?: string;
  } = $props();

  const id = `motivo-${Math.random().toString(36).slice(2, 9)}`;
  const conMotivo = $derived(deshabilitado && !!motivo);
</script>

<span class="envoltura" class:con-motivo={conMotivo}>
  <button
    type="button"
    class={variante}
    disabled={deshabilitado}
    aria-describedby={conMotivo ? id : undefined}
    aria-label={etiqueta}
    {onclick}
  >
    {@render children()}
  </button>
  {#if conMotivo}
    <span {id} class="motivo">{motivo}</span>
  {/if}
</span>

<style>
  .envoltura {
    display: inline-flex;
    position: relative;
  }
  /* El motivo cuelga debajo, alineado a la derecha, sin ensanchar el botón. */
  .con-motivo {
    padding-bottom: 30px;
  }
  button {
    font: inherit;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    cursor: pointer;
    text-decoration: none;
  }
  .principal {
    font-weight: 700;
    font-size: 17px;
    min-height: var(--alto-principal);
    padding: 0 26px;
    border-radius: var(--radio);
    border: 0;
    background: var(--color-accion);
    color: #fff;
  }
  .principal:hover:not(:disabled) {
    background: var(--color-accion-oscura);
  }
  .secundario {
    font-size: 16px;
    min-height: var(--alto-control);
    padding: 0 18px;
    border-radius: 10px;
    border: 1px solid var(--color-borde-fuerte);
    background: var(--color-superficie);
    color: var(--color-texto);
  }
  .secundario:hover:not(:disabled) {
    border-color: var(--color-accion);
  }
  .enlace {
    font-size: 16px;
    min-height: var(--alto-control);
    padding: 0 6px;
    background: none;
    border: 0;
    color: var(--color-accion);
    text-decoration: underline;
  }
  button:disabled {
    cursor: not-allowed;
  }
  .principal:disabled {
    background: var(--color-desactivado);
  }
  .secundario:disabled {
    color: var(--color-texto-2);
  }
  .motivo {
    position: absolute;
    right: 0;
    bottom: 0;
    white-space: nowrap;
    font-size: 15px;
    color: var(--color-texto-2);
    text-align: right;
  }
</style>
