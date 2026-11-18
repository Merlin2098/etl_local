<script lang="ts">
  // Marco común de los gráficos: título, subtítulo, alcance de filtros, «Ver como tabla» y «Descargar CSV»
  // (ADR-0005 §2: cada gráfico tiene su tabla, que es también la vía para lectores de pantalla).
  import type { Snippet } from "svelte";
  import Boton from "../lib/componentes/Boton.svelte";
  import { csvConContexto, descargar } from "../lib/csv";
  import { numero } from "../lib/formato";

  let {
    id,
    titulo,
    subtitulo,
    nota,
    columnas,
    filas,
    archivo,
    vacio = false,
    grafico,
    leyenda,
    contexto,
  }: {
    id: string;
    titulo: string;
    subtitulo?: string;
    nota?: string;
    columnas: string[];
    filas: (string | number)[][];
    archivo: string;
    vacio?: boolean;
    grafico: Snippet;
    leyenda?: Snippet;
    /** Primera línea del CSV: los filtros que produjeron estas cifras. */
    contexto: string;
  } = $props();

  let comoTabla = $state(false);
</script>

<section class="panel" aria-labelledby={id}>
  <div class="cabeza">
    <div>
      <h2 {id}>{titulo}</h2>
      {#if subtitulo}<p class="muted">{subtitulo}</p>{/if}
    </div>
    {#if !vacio}
      <Boton variante="enlace" onclick={() => (comoTabla = !comoTabla)}>
        {comoTabla ? "Ver como gráfico" : "Ver como tabla"}
      </Boton>
    {/if}
  </div>
  {#if nota}<span class="nota">{nota}</span>{/if}
  {#if vacio}
    <p class="muted vacio">Sin datos para este filtro.</p>
  {:else if comoTabla}
    <div class="tabla">
      <table>
        <caption class="visualmente-oculto">{titulo}</caption>
        <thead><tr>{#each columnas as c (c)}<th scope="col">{c}</th>{/each}</tr></thead>
        <tbody>
          {#each filas as f, k (k)}
            <tr>{#each f as v, j (j)}<td class:num={typeof v === "number"}>{typeof v === "number" ? numero(v) : v}</td>{/each}</tr>
          {/each}
        </tbody>
      </table>
    </div>
    <div><Boton onclick={() => descargar(archivo, csvConContexto(contexto, columnas, filas))}>Descargar CSV</Boton></div>
  {:else}
    {#if leyenda}{@render leyenda()}{/if}
    {@render grafico()}
  {/if}
</section>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 20px 22px;
    background: var(--color-superficie);
    border: 1px solid var(--color-borde);
    border-radius: 16px;
    min-width: 0;
  }
  .cabeza {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    flex-wrap: wrap;
  }
  h2 {
    margin: 0;
    font-size: 21px;
  }
  .cabeza p {
    margin: 4px 0 0;
    font-size: 15px;
  }
  .nota {
    font-size: 14px;
    color: var(--color-texto-2);
  }
  .vacio {
    padding: 24px 0;
    margin: 0;
  }
  .tabla {
    overflow-x: auto;
    border: 1px solid var(--color-borde);
    border-radius: 10px;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 15px;
  }
  th,
  td {
    padding: 8px 12px;
    border-bottom: 1px solid var(--color-gris-suave);
    text-align: left;
  }
  th {
    font-size: 13px;
    color: var(--color-texto-2);
    background: var(--color-gris-suave);
  }
  .num {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
</style>
