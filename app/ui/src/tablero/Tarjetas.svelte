<script lang="ts">
  // Las cuatro tasas de arriba (diseno §Principios 5): cifra, comparación con flecha y palabra, y contexto.
  import type { Indicador } from "../lib/api/tipos.gen";
  import type { DatosResumen } from "./estado.svelte";
  import Origen from "./Origen.svelte";
  import { buscar, cifra, lineaTarjeta, textoComparacion } from "./textos";

  let { datos, alVerQuienes }: { datos: DatosResumen; alVerQuienes: () => void } = $props();

  const IDS = ["IND-01", "IND-04", "IND-05", "IND-07"];
  const tarjetas = $derived(IDS.map((id) => buscar(datos, id)).filter((i): i is Indicador => !!i));
  const sinBase = $derived(datos.checks.checks.find((c) => c.id === "IND-C1")?.valor ?? 0);

  function flecha(i: Indicador): string {
    if (i.variacion == null || i.variacion === 0) return "→";
    return i.variacion > 0 ? "↑" : "↓";
  }

  /** IND-07 no se compara: usa la última base de personal (decisión 2 del plan). */
  function comparacion(i: Indicador): string | null {
    return i.id === "IND-07" ? null : textoComparacion(i, datos.indicadores.periodo_anterior);
  }
</script>

<section class="tarjetas" aria-label="Indicadores">
  {#each tarjetas as i (i.id)}
    {@const comp = comparacion(i)}
    {@const linea = lineaTarjeta(i.id, datos)}
    <article class="tarjeta">
      <div class="cabeza">
        <h2>{i.nombre}</h2>
        <Origen indicador={i} {sinBase} {alVerQuienes} />
      </div>
      <span class="cifra">{cifra(i.valor, i.unidad)}</span>
      {#if comp}
        <span class="comparacion" class:peor={i.comparacion === "peor"} class:mejor={i.comparacion === "mejor"}>
          <span aria-hidden="true">{flecha(i)}</span>
          {comp}
        </span>
      {/if}
      {#if linea}<span class="linea muted">{linea}</span>{/if}
    </article>
  {/each}
</section>

<style>
  .tarjetas {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 16px;
  }
  .tarjeta {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 18px 20px;
    background: var(--color-superficie);
    border: 1px solid var(--color-borde);
    border-radius: 14px;
  }
  .cabeza {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 8px;
  }
  h2 {
    margin: 0;
    font-family: var(--fuente-texto);
    font-size: 16px;
    font-weight: 700;
    color: var(--color-texto-2);
  }
  .cifra {
    font-family: var(--fuente-titulo);
    font-weight: 700;
    font-size: 38px;
    line-height: 1.1;
  }
  .comparacion {
    font-weight: 700;
    font-size: 15px;
  }
  .peor {
    color: var(--color-aviso-texto);
  }
  .mejor {
    color: var(--color-accion-oscura);
  }
  .linea {
    font-size: 14px;
  }
  @media (max-width: 1100px) {
    .tarjetas {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
  @media (max-width: 560px) {
    .tarjetas {
      grid-template-columns: 1fr;
    }
  }
</style>
