<script lang="ts">
  import type { Recorte } from "../../app/tipos";
  import { letraColumna, marcas } from "../recorte";

  let {
    recorte,
    esperada = null,
    encontrada = null,
  }: { recorte: Recorte; esperada?: string | null; encontrada?: string | null } = $props();

  const m = $derived(marcas(recorte, esperada, encontrada));
  const ultimaFila = $derived(recorte.fila_inicio + recorte.celdas.length - 1);
  const columnas = $derived((recorte.celdas[0] ?? []).map((_, j) => letraColumna(recorte.columna_inicio + j)));
</script>

<figure>
  <div class="caja">
    <table
      class="grid"
      aria-label={`Vista de la hoja ${recorte.hoja}, filas ${recorte.fila_inicio} a ${ultimaFila}`}
    >
      <thead>
        <tr>
          <th scope="col"><span class="visualmente-oculto">Fila</span></th>
          {#each columnas as letra (letra)}<th scope="col">{letra}</th>{/each}
        </tr>
      </thead>
      <tbody>
        {#each recorte.celdas as fila, i (i)}
          <tr>
            <th scope="row">{recorte.fila_inicio + i}</th>
            {#each fila as texto, j (j)}
              {@const marca = m[i]?.[j] ?? null}
              <td class:esperado={marca === "esperada"} class:real={marca === "real"}>
                {#if marca === "esperada" && texto === ""}<span class="muted">(vacía)</span>{:else}{texto}{/if}
              </td>
            {/each}
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
  {#if encontrada}
    <figcaption class="muted">
      <span class="leyenda"><span class="muestra esperado"></span>Aquí deberían ir los títulos</span>
      <span class="leyenda"><span class="muestra real"></span>Aquí están ahora</span>
    </figcaption>
  {/if}
</figure>

<style>
  figure {
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-width: 0;
  }
  .caja {
    overflow-x: auto;
    border-radius: 8px;
    border: 1px solid #d3d9d2;
    max-width: 100%;
  }
  .grid {
    border-collapse: collapse;
    font-size: 14px;
    background: var(--color-superficie);
  }
  th {
    background: var(--color-gris-suave);
    color: #454f49;
    font-weight: 700;
    border: 1px solid #d3d9d2;
    padding: 4px 10px;
    font-size: 13px;
  }
  td {
    border: 1px solid #d3d9d2;
    padding: 6px 10px;
    white-space: nowrap;
    min-width: 84px;
    height: 20px;
  }
  .esperado {
    outline: 3px solid var(--color-error);
    outline-offset: -2px;
    background: var(--color-error-fondo);
  }
  .real {
    background: var(--color-real);
  }
  figcaption {
    font-size: 15px;
    display: flex;
    gap: 18px;
    flex-wrap: wrap;
  }
  .leyenda {
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }
  .muestra {
    width: 14px;
    height: 14px;
    display: inline-block;
  }
  .muestra.esperado {
    border: 3px solid var(--color-error);
    background: var(--color-error-fondo);
    outline: none;
  }
  .muestra.real {
    background: var(--color-real);
    border: 1px solid #d3d9d2;
  }
</style>
