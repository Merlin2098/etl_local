<script lang="ts">
  import type { Snippet } from "svelte";

  let { acciones, titulo = "Integrador RR. HH." }: { acciones?: Snippet; titulo?: string } = $props();
</script>

<header class="top">
  <div class="marca">
    <span class="logo" aria-hidden="true"><span></span><span></span><span></span><span></span></span>
    {titulo}
  </div>
  {#if acciones}<div class="acciones">{@render acciones()}</div>{/if}
</header>

<style>
  .top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    flex-wrap: wrap;
    padding: 16px 40px;
    background: var(--color-superficie);
    border-bottom: 1px solid var(--color-borde);
  }
  .marca {
    display: flex;
    align-items: center;
    gap: 12px;
    font-family: var(--fuente-titulo);
    font-weight: 700;
    font-size: 20px;
  }
  .logo {
    width: 36px;
    height: 36px;
    border-radius: 8px;
    background: var(--color-accion);
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 3px;
    padding: 7px;
  }
  .logo span {
    background: #fff;
    border-radius: 2px;
    opacity: 0.9;
  }
  .acciones {
    display: flex;
    gap: 12px;
    flex-wrap: wrap;
  }
</style>
