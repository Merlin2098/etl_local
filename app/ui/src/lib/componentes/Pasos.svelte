<script lang="ts">
  import Icono from "./Icono.svelte";

  let { actual }: { actual: 1 | 2 | 3 | 4 } = $props();
  const PASOS = ["Elegir archivos", "Revisar", "Procesar", "Listo"];
</script>

<ol class="pasos" aria-label="Pasos">
  {#each PASOS as nombre, i (nombre)}
    {@const n = i + 1}
    {#if i > 0}<li class="sep" aria-hidden="true"></li>{/if}
    <li class="paso" class:activo={n === actual} class:hecho={n < actual} aria-current={n === actual ? "step" : undefined}>
      <span class="num">
        {#if n < actual || (n === 4 && actual === 4)}<Icono nombre="check" tamano={14} grosor={3} />{:else}{n}{/if}
      </span>
      {nombre}
      {#if n < actual}<span class="visualmente-oculto">(hecho)</span>{/if}
    </li>
  {/each}
</ol>

<style>
  .pasos {
    display: flex;
    gap: 8px;
    align-items: center;
    flex-wrap: wrap;
    list-style: none;
    margin: 0;
    padding: 0;
    font-size: 15px;
  }
  .paso {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--color-texto-2);
  }
  .num {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    font-weight: 700;
    border: 2px solid var(--color-borde-fuerte);
    background: var(--color-superficie);
    font-size: 14px;
  }
  .activo {
    color: var(--color-texto);
    font-weight: 700;
  }
  .activo .num {
    background: var(--color-accion);
    border-color: var(--color-accion);
    color: #fff;
  }
  .hecho .num {
    background: var(--color-accion-suave);
    border-color: var(--color-accion);
    color: var(--color-accion);
  }
  .sep {
    width: 32px;
    height: 2px;
    background: var(--color-borde);
  }
</style>
