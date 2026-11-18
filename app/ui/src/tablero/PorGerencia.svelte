<script lang="ts">
  // Barras HTML (decisión 1 del plan): cada barra es un botón que filtra todo el tablero (cross-filtering).
  import { nombreMes, numero } from "../lib/formato";
  import type { Indicador } from "../lib/api/tipos.gen";
  import TarjetaGrafico from "./TarjetaGrafico.svelte";
  import { filasGerencia } from "./graficos";

  let {
    indicador,
    periodo,
    sinBase,
    alElegir,
    contexto,
  }: {
    indicador: Indicador;
    periodo: string | null | undefined;
    sinBase: number;
    alElegir: (g: string) => void;
    contexto: string;
  } = $props();

  const filas = $derived(filasGerencia(indicador));
  const maximo = $derived(Math.max(1, ...filas.map((f) => f.valor)));
  const nota = $derived(
    [periodo ? nombreMes(periodo) : null, sinBase > 0 ? `${numero(sinBase)} personas sin gerencia en la base de personal no se muestran` : null]
      .filter(Boolean)
      .join(" · "),
  );
</script>

<TarjetaGrafico
  {contexto}
  id="t-ger"
  titulo="Personas en planilla por gerencia"
  subtitulo="Haz clic en una barra para filtrar todo el tablero"
  {nota}
  columnas={["Gerencia", "Personas"]}
  filas={filas.map((f) => [f.gerencia, f.valor])}
  archivo="personas-por-gerencia.csv"
  vacio={filas.length === 0}
>
  {#snippet grafico()}
    <div class="barras">
      {#each filas as f (f.gerencia)}
        <button type="button" class="barra" aria-label={`${f.gerencia}: ${numero(f.valor)} personas`} onclick={() => alElegir(f.gerencia)}>
          <span class="nombre">{f.gerencia}</span>
          <span class="pista"><span class="relleno" style:width="{(f.valor / maximo) * 100}%"></span></span>
          <span class="valor">{numero(f.valor)}</span>
        </button>
      {/each}
    </div>
  {/snippet}
</TarjetaGrafico>

<style>
  .barras {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .barra {
    font: inherit;
    font-size: 15px;
    display: grid;
    grid-template-columns: minmax(110px, 30%) 1fr 56px;
    align-items: center;
    gap: 12px;
    min-height: var(--alto-control);
    padding: 0 8px;
    border: 0;
    border-radius: 8px;
    background: none;
    color: var(--color-texto);
    text-align: left;
    cursor: pointer;
  }
  .barra:hover {
    background: var(--color-accion-suave);
  }
  .nombre {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .pista {
    height: 14px;
    border-radius: 4px;
    background: var(--color-gris-suave);
  }
  .relleno {
    display: block;
    height: 100%;
    border-radius: 4px;
    background: var(--color-accion);
  }
  .valor {
    text-align: right;
    font-variant-numeric: tabular-nums;
    font-weight: 700;
  }
</style>
