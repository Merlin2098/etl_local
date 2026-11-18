<script lang="ts">
  import Aviso from "../../lib/componentes/Aviso.svelte";
  import Boton from "../../lib/componentes/Boton.svelte";
  import Encabezado from "../../lib/componentes/Encabezado.svelte";
  import Icono from "../../lib/componentes/Icono.svelte";
  import Pasos from "../../lib/componentes/Pasos.svelte";
  import type { Flujo } from "../flujo.svelte";
  import { tituloListo } from "../textos";
  import DetallesSoporte from "./DetallesSoporte.svelte";

  let { flujo }: { flujo: Flujo } = $props();
  const reporte = $derived(flujo.reporte!);
  const r = $derived(flujo.resultado!);
  const hora = $derived.by(() => {
    const m = /^\d{8}-(\d{2})(\d{2})/.exec(r.ejecucion.marca);
    return m ? `${m[1]}:${m[2]}` : "";
  });
  const primero = $derived(r.para_revisar[0]);
  const resto = $derived(r.para_revisar.slice(1));
  let errorAbrir = $state<string | null>(null);

  async function abrir(que: Parameters<Flujo["backend"]["abrir"]>[0]) {
    errorAbrir = null;
    try {
      await flujo.backend.abrir(que);
    } catch (e) {
      errorAbrir = e instanceof Error ? e.message : String(e);
    }
  }
</script>

<Encabezado>
  {#snippet acciones()}
    <Boton onclick={() => flujo.volverAlInicio()}>Volver al inicio</Boton>
  {/snippet}
</Encabezado>

<main>
  <Pasos actual={4} />

  <div class="exito">
    <span class="sello"><Icono nombre="check" tamano={34} grosor={2.8} /></span>
    <div class="titulos">
      <h1 tabindex="-1">{tituloListo(reporte)}</h1>
      <p class="muted">{r.titulo_periodos}{hora ? ` · actualizada hoy a las ${hora}` : ""}</p>
    </div>
  </div>

  {#if r.cifras.length > 0}
    <div class="cifras">
      {#each r.cifras as c (c.etiqueta)}
        <div class="dato"><span class="cifra">{c.valor}</span><span class="muted">{c.etiqueta}</span></div>
      {/each}
    </div>
  {/if}

  {#if primero}
    <Aviso>
      <span><strong>Para revisar:</strong> {primero.mensaje}</span>
      {#if resto.length > 0}
        <details>
          <summary>y {resto.length} más</summary>
          <ul>
            {#each resto as p, i (i)}<li>{p.mensaje}</li>{/each}
          </ul>
        </details>
      {/if}
    </Aviso>
  {/if}

  <div class="panel">
    <p class="pregunta">¿Qué quieres hacer ahora?</p>
    <div class="acciones">
      <Boton variante="principal" onclick={() => flujo.verTablero()}>
        <Icono nombre="grafico" />Ver en el tablero
      </Boton>
      <Boton onclick={() => abrir({ tipo: "excel", reporte: reporte.id })}><Icono nombre="excel" />Abrir en Excel</Boton>
      <Boton onclick={() => abrir({ tipo: "carpeta" })}><Icono nombre="carpeta" />Abrir carpeta</Boton>
    </div>
    {#if errorAbrir}<Aviso tipo="error">{errorAbrir}</Aviso>{/if}
    {#if flujo.huboAnterior}
      <p class="muted pequeno">Guardamos una copia de la versión anterior por si necesitas compararla.</p>
    {/if}
  </div>

  <DetallesSoporte ejecucion={r.ejecucion} />
</main>

<style>
  main {
    max-width: 880px;
    margin: 0 auto;
    padding: 36px 40px 48px;
    display: flex;
    flex-direction: column;
    gap: 28px;
  }
  .exito {
    display: flex;
    align-items: center;
    gap: 20px;
    flex-wrap: wrap;
  }
  .sello {
    width: 64px;
    height: 64px;
    border-radius: 50%;
    background: var(--color-accion);
    color: #fff;
    display: flex;
    align-items: center;
    justify-content: center;
    flex: none;
  }
  .titulos {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .titulos p {
    margin: 0;
  }
  h1 {
    font-size: 38px;
    line-height: 1.1;
    letter-spacing: -0.01em;
  }
  .cifras {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 16px;
  }
  .dato {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 18px 20px;
    background: var(--color-superficie);
    border: 1px solid var(--color-borde);
    border-radius: 14px;
  }
  .cifra {
    font-family: var(--fuente-titulo);
    font-weight: 700;
    font-size: 34px;
    line-height: 1.1;
  }
  .panel {
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 24px;
    background: var(--color-superficie);
    border: 1px solid var(--color-borde);
    border-radius: 16px;
  }
  .panel p {
    margin: 0;
  }
  .pregunta {
    font-weight: 700;
    font-size: 19px;
  }
  .acciones {
    display: flex;
    gap: 12px;
    flex-wrap: wrap;
  }
  .pequeno {
    font-size: 15px;
  }
  summary {
    cursor: pointer;
    text-decoration: underline;
  }
</style>
