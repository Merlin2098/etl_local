<script lang="ts">
  // Un gráfico de ECharts en SVG que se adapta a su contenedor y se redibuja antes de imprimir.
  import type { EChartsCoreOption, EChartsType } from "echarts/core";
  import { onMount } from "svelte";
  import { echarts } from "./echarts";

  let { opcion, etiqueta, alto = 260 }: { opcion: EChartsCoreOption; etiqueta: string; alto?: number } = $props();

  let caja: HTMLDivElement | undefined = $state();
  let grafico: EChartsType | undefined = $state();

  onMount(() => {
    if (!caja) return;
    const g = echarts.init(caja, undefined, { renderer: "svg" });
    grafico = g;
    const ajustar = () => g.resize();
    const observador = new ResizeObserver(ajustar);
    observador.observe(caja);
    window.addEventListener("beforeprint", ajustar);
    return () => {
      observador.disconnect();
      window.removeEventListener("beforeprint", ajustar);
      g.dispose();
    };
  });

  $effect(() => {
    grafico?.setOption(opcion, { notMerge: true });
  });
</script>

<div class="grafico" bind:this={caja} role="img" aria-label={etiqueta} style:height="{alto}px"></div>

<style>
  .grafico {
    width: 100%;
  }
</style>
