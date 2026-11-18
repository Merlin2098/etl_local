<script lang="ts">
  import { mesMinuscula, numero } from "../lib/formato";
  import type { Indicador } from "../lib/api/tipos.gen";
  import TarjetaGrafico from "./TarjetaGrafico.svelte";
  import { filasAusencias } from "./graficos";
  import { porcentaje } from "./textos";

  let {
    indicador,
    periodo,
    contexto,
  }: { indicador: Indicador; periodo: string | null | undefined; contexto: string } = $props();

  const filas = $derived(filasAusencias(indicador));
  const dias = $derived(filas.reduce((s, f) => s + f.dias, 0));
  const maximo = $derived(Math.max(1, ...filas.map((f) => f.dias)));
  const pct = (p: number) => `${Math.round(p)} %`;
</script>

<TarjetaGrafico
  {contexto}
  id="t-aus"
  titulo={`Días de ausencia${periodo ? ` en ${mesMinuscula(periodo)}` : ""}, por motivo`}
  subtitulo={`${numero(Math.round(dias))} días · ${porcentaje(indicador.valor)} del tiempo programado`}
  nota="Fuente: planilla · cada columna de días se cuenta una sola vez"
  columnas={["Motivo", "Días", "% del total"]}
  filas={filas.map((f) => [f.motivo, f.dias, Math.round(f.pct)])}
  archivo="dias-de-ausencia.csv"
  vacio={filas.length === 0}
>
  {#snippet grafico()}
    <div class="filas">
      {#each filas as f, k (f.motivo)}
        <div class="fila">
          <span>{f.motivo}</span>
          <span class="pista" aria-hidden="true"><span class="relleno" style:width="{(f.dias / maximo) * 100}%"></span></span>
          <span class="valor">
            {#if k === 0}<strong>{numero(f.dias)}</strong>{:else}{numero(f.dias)}{/if} · {pct(f.pct)}
          </span>
        </div>
      {/each}
    </div>
  {/snippet}
</TarjetaGrafico>

<style>
  .filas {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .fila {
    display: grid;
    grid-template-columns: minmax(150px, 38%) 1fr 92px;
    align-items: center;
    gap: 12px;
    font-size: 15px;
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
  }
</style>
