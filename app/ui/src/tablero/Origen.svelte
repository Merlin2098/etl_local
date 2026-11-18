<script lang="ts">
  // «¿De dónde sale este número?» (TR-D17): fuente, qué cuenta y regla de cada indicador.
  import { onMount } from "svelte";
  import Icono from "../lib/componentes/Icono.svelte";
  import { fechaCorta, nombreMes, numero } from "../lib/formato";
  import type { Indicador } from "../lib/api/tipos.gen";
  import { GLOSARIO } from "./glosario";

  let {
    indicador,
    sinBase = 0,
    alVerQuienes,
  }: { indicador: Indicador; sinBase?: number; alVerQuienes?: () => void } = $props();

  let abierto = $state(false);
  let caja: HTMLDivElement | undefined = $state();
  const id = `origen-${Math.random().toString(36).slice(2, 9)}`;

  const o = $derived(indicador.origen);
  const fuente = $derived(
    o.fuente === "planilla"
      ? `Planilla${o.periodo ? ` de ${nombreMes(o.periodo).toLowerCase()}` : ""}`
      : `Base de personal${o.fecha_corte ? ` con corte al ${fechaCorta(o.fecha_corte)}` : ""}`,
  );

  onMount(() => {
    const fuera = (e: MouseEvent) => {
      if (abierto && caja && !caja.contains(e.target as Node)) abierto = false;
    };
    const tecla = (e: KeyboardEvent) => {
      if (abierto && e.key === "Escape") abierto = false;
    };
    document.addEventListener("click", fuera);
    document.addEventListener("keydown", tecla);
    return () => {
      document.removeEventListener("click", fuera);
      document.removeEventListener("keydown", tecla);
    };
  });
</script>

<div class="origen" bind:this={caja}>
  <button
    type="button"
    class="abrir"
    aria-label="¿De dónde sale este número?"
    aria-expanded={abierto}
    aria-controls={id}
    onclick={() => (abierto = !abierto)}
  >
    <Icono nombre="ayuda" tamano={18} />
  </button>
  {#if abierto}
    <div class="panel" role="dialog" aria-label={`Origen de ${indicador.nombre}`} {id}>
      <strong>¿De dónde sale este número?</strong>
      <dl>
        <dt>Fuente</dt>
        <dd>{fuente}</dd>
        <dt>Qué cuenta</dt>
        <dd>{GLOSARIO[indicador.id]?.calculo ?? indicador.pregunta}</dd>
        <dt>Regla</dt>
        <dd class="reglas">{#each o.reglas as r (r)}<span class="regla">{r}</span>{/each}</dd>
      </dl>
      {#if indicador.id === "IND-01" && sinBase > 0}
        <p class="sin-base">
          <strong>{numero(sinBase)} {sinBase === 1 ? "persona" : "personas"}</strong> de la planilla no
          {sinBase === 1 ? "está" : "están"} en la base de personal, así que no aparecen al filtrar por gerencia o sede.
          <button type="button" class="enlace" onclick={alVerQuienes}>Ver quiénes</button>
        </p>
      {/if}
    </div>
  {/if}
</div>

<style>
  .origen {
    position: relative;
  }
  .abrir {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    border-radius: 50%;
    border: 1px solid var(--color-borde-fuerte);
    background: var(--color-superficie);
    color: var(--color-texto-2);
    cursor: pointer;
  }
  .abrir::after {
    /* Área táctil de 44 px sin agrandar el ícono. */
    content: "";
    position: absolute;
    inset: -6px;
  }
  .panel {
    position: absolute;
    right: 0;
    top: 40px;
    z-index: 5;
    width: min(360px, 80vw);
    padding: 14px 16px;
    background: var(--color-superficie);
    border: 1px solid var(--color-borde-fuerte);
    border-radius: var(--radio);
    box-shadow: 0 8px 24px rgb(27 36 32 / 0.14);
    display: flex;
    flex-direction: column;
    gap: 8px;
    font-size: 15px;
  }
  dl {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 4px 12px;
    margin: 0;
  }
  dt {
    font-weight: 700;
    color: var(--color-texto-2);
  }
  dd {
    margin: 0;
  }
  .reglas {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  .regla {
    font-family: var(--fuente-mono);
    font-size: 13px;
    padding: 1px 6px;
    border-radius: 5px;
    border: 1px solid var(--color-desactivado);
    background: var(--color-fondo);
  }
  .sin-base {
    margin: 0;
    padding: 10px 12px;
    border-radius: 10px;
    background: var(--color-aviso-fondo);
    color: var(--color-aviso-texto);
  }
  .enlace {
    font: inherit;
    font-weight: 700;
    color: var(--color-accion);
    background: none;
    border: 0;
    padding: 0;
    text-decoration: underline;
    cursor: pointer;
  }
</style>
