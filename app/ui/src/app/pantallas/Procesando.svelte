<script lang="ts">
  import { onMount } from "svelte";
  import Aviso from "../../lib/componentes/Aviso.svelte";
  import BarraAvance from "../../lib/componentes/BarraAvance.svelte";
  import Boton from "../../lib/componentes/Boton.svelte";
  import ChipCelda from "../../lib/componentes/ChipCelda.svelte";
  import Encabezado from "../../lib/componentes/Encabezado.svelte";
  import Icono from "../../lib/componentes/Icono.svelte";
  import Pasos from "../../lib/componentes/Pasos.svelte";
  import { textoEta } from "../eta";
  import type { Flujo } from "../flujo.svelte";
  import { textoPaso, tituloProcesando } from "../textos";
  import DetallesSoporte from "./DetallesSoporte.svelte";

  let { flujo }: { flujo: Flujo } = $props();
  let ahora = $state(Date.now());

  onMount(() => {
    const reloj = setInterval(() => (ahora = flujo.ahora), 1000);
    return () => clearInterval(reloj);
  });

  const indice = $derived(flujo.avance?.indice ?? 0);
  const total = $derived(flujo.avance?.total ?? flujo.pasos.length);
  const contexto = $derived(flujo.contextoPasos);
  const eta = $derived(textoEta(flujo.duracionPrevista, ahora - flujo.inicioProceso));
</script>

<Encabezado />

<main>
  <Pasos actual={3} />

  <div class="cabecera">
    <h1 tabindex="-1">{tituloProcesando(flujo.reporte!)}</h1>
    <p class="muted">Suele tardar menos de un minuto. Puedes seguir usando la computadora; te avisaremos al terminar.</p>
  </div>

  {#if flujo.error}
    <Aviso tipo="error" titulo="No se pudo terminar.">
      <span>{flujo.error.mensaje}</span>
      {#if flujo.error.problemas.length > 0}
        <ul class="problemas">
          {#each flujo.error.problemas as p, i (i)}
            <li>
              <strong>{p.archivo}</strong>: {p.mensaje}
              {#if p.celda && p.hoja}<ChipCelda hoja={p.hoja} celda={p.celda} />{/if}
            </li>
          {/each}
        </ul>
      {/if}
    </Aviso>
    <div class="acciones">
      <Boton onclick={() => flujo.volverAlInicio()}>Volver al inicio</Boton>
      <Boton variante="principal" onclick={() => flujo.reintentar()}>Volver a intentar</Boton>
    </div>
    <DetallesSoporte error={flujo.error} />
  {:else}
    <div class="panel">
      <div class="linea">
        <span class="fuerte">{flujo.avance ? `${indice + 1} de ${total} tareas` : "Preparando…"}</span>
        {#if eta}<span class="muted pequeno">{eta}</span>{/if}
      </div>
      <BarraAvance porcentaje={total > 0 ? (indice / total) * 100 : 0} />
      <ul class="tareas">
        {#each flujo.pasos as paso, i (paso)}
          {@const estado = i < indice ? "hecha" : i === indice ? "ahora" : "pend"}
          <li class="tarea">
            <span class="chk {estado}" aria-hidden="true">
              {#if estado === "hecha"}<Icono nombre="check" tamano={16} grosor={3} />{/if}
            </span>
            <span class="nombre" class:fuerte={estado === "ahora"} class:muted={estado === "pend"}>
              {textoPaso(paso, contexto)}
            </span>
            {#if estado === "ahora"}<span class="muted pequeno">En curso…</span>{/if}
            <span class="visualmente-oculto">{estado === "hecha" ? "hecha" : estado === "ahora" ? "en curso" : "pendiente"}</span>
          </li>
        {/each}
      </ul>
    </div>
    <div class="acciones extremos">
      <span></span>
      <Boton variante="enlace" onclick={() => flujo.detener()}>Detener</Boton>
    </div>
  {/if}
</main>

<style>
  main {
    max-width: 760px;
    margin: 0 auto;
    padding: 36px 40px 48px;
    display: flex;
    flex-direction: column;
    gap: 28px;
  }
  .cabecera {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .cabecera p {
    margin: 0;
  }
  h1 {
    font-size: 34px;
    letter-spacing: -0.01em;
  }
  .panel {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 24px;
    background: var(--color-superficie);
    border: 1px solid var(--color-borde);
    border-radius: 16px;
  }
  .linea {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 12px;
  }
  .fuerte {
    font-weight: 700;
  }
  .pequeno {
    font-size: 15px;
  }
  .tareas {
    list-style: none;
    margin: 12px 0 0;
    padding: 0;
  }
  .tarea {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 12px 0;
    border-bottom: 1px solid var(--color-gris-suave);
  }
  .tarea:last-child {
    border-bottom: 0;
  }
  .nombre {
    flex: 1;
  }
  .chk {
    width: 30px;
    height: 30px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    flex: none;
  }
  .hecha {
    background: var(--color-accion);
    color: #fff;
  }
  .ahora {
    border: 3px solid var(--color-accion);
    border-right-color: #cfe3d7;
    animation: gira 1s linear infinite;
  }
  .pend {
    border: 2px solid var(--color-borde-fuerte);
  }
  @keyframes gira {
    to {
      transform: rotate(360deg);
    }
  }
  .acciones {
    display: flex;
    gap: 12px;
    flex-wrap: wrap;
  }
  .extremos {
    justify-content: space-between;
  }
  .problemas {
    margin: 8px 0 0;
    padding-left: 20px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
</style>
