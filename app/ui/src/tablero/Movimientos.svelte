<script lang="ts">
  import { nombreMes } from "../lib/formato";
  import type { RespuestaSeries } from "../lib/api/tipos.gen";
  import Grafico from "../lib/graficos/Grafico.svelte";
  import { COLORES } from "../lib/graficos/tema";
  import TarjetaGrafico from "./TarjetaGrafico.svelte";
  import { filasMovimientos, opcionMovimientos } from "./graficos";

  let { series, contexto }: { series: RespuestaSeries; contexto: string } = $props();
  const opcion = $derived(opcionMovimientos(series));
  const filas = $derived(filasMovimientos(series).map((f) => [nombreMes(f.periodo), f.entraron, f.salieron]));
</script>

<TarjetaGrafico
  {contexto}
  id="t-mov"
  titulo="Entradas y salidas de personal por mes"
  subtitulo="Número de personas"
  nota="Siempre muestra los últimos 6 meses · sí responde a Gerencia, Sede y Tipo"
  columnas={["Mes", "Entraron", "Salieron"]}
  {filas}
  archivo="entradas-y-salidas.csv"
  vacio={!opcion}
>
  {#snippet leyenda()}
    <div class="leyenda" aria-hidden="true">
      <span><span class="punto" style:background={COLORES.entraron}></span>Entraron</span>
      <span><span class="punto" style:background={COLORES.salieron}></span>Salieron</span>
    </div>
  {/snippet}
  {#snippet grafico()}
    {#if opcion}
      <Grafico {opcion} etiqueta="Entradas y salidas de personal en los últimos meses; el detalle está en «Ver como tabla»." />
    {/if}
  {/snippet}
</TarjetaGrafico>

<style>
  .leyenda {
    display: flex;
    gap: 16px;
    font-size: 14px;
  }
  .leyenda > span {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .punto {
    width: 12px;
    height: 12px;
    border-radius: 3px;
  }
</style>
