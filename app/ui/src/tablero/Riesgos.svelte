<script lang="ts">
  // «Riesgos a atender»: resumen para dirección, sin nombres (TR-D18); el detalle está en «Alertas».
  import Boton from "../lib/componentes/Boton.svelte";
  import type { DatosResumen } from "./estado.svelte";
  import { buscar, textoRiesgo } from "./textos";

  let { datos, alVerDetalle }: { datos: DatosResumen; alVerDetalle: () => void } = $props();
  const riesgos = $derived(
    ["IND-07", "IND-08"].flatMap((id) => {
      const i = buscar(datos, id);
      return i && i.valor != null ? [{ id, ...textoRiesgo(i) }] : [];
    }),
  );
</script>

<section class="panel riesgos" aria-labelledby="t-riesgo">
  <div class="cabeza">
    <div>
      <h2 id="t-riesgo">Riesgos a atender</h2>
      <p class="muted">Resumen para dirección · el detalle con nombres está en la pestaña «Alertas», solo para RR. HH.</p>
    </div>
    <Boton onclick={alVerDetalle}>Ver detalle (solo RR. HH.)</Boton>
  </div>
  {#if riesgos.length === 0}
    <p class="muted">Sin datos para este filtro.</p>
  {/if}
  {#each riesgos as r (r.id)}
    <div class="riesgo">
      {#if r.detalle}<span class="cifra">{r.cifra}</span>{/if}
      <div class="texto">
        <strong>{r.titulo}</strong>
        {#if r.detalle}<span>{r.detalle}</span>{/if}
        {#if r.masCasos}<span class="muted">{r.masCasos}</span>{/if}
      </div>
    </div>
  {/each}
</section>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 20px 22px;
    background: var(--color-superficie);
    border: 1px solid var(--color-borde);
    border-radius: 16px;
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
  .riesgo {
    display: flex;
    gap: 16px;
    align-items: flex-start;
    padding: 14px 16px;
    border-radius: 12px;
    background: var(--color-fondo);
  }
  .cifra {
    font-family: var(--fuente-titulo);
    font-weight: 700;
    font-size: 34px;
    line-height: 1;
    min-width: 56px;
  }
  .texto {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 15px;
  }
  .texto strong {
    font-size: 16px;
  }
</style>
