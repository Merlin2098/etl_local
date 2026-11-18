<script lang="ts">
  import Aviso from "../../lib/componentes/Aviso.svelte";
  import Boton from "../../lib/componentes/Boton.svelte";
  import Encabezado from "../../lib/componentes/Encabezado.svelte";
  import Icono from "../../lib/componentes/Icono.svelte";
  import { saludo } from "../../lib/formato";
  import type { Flujo } from "../flujo.svelte";
  import TarjetaReporte from "./TarjetaReporte.svelte";

  let { flujo }: { flujo: Flujo } = $props();
  const hora = $derived(new Date(flujo.ahora).getHours());
</script>

<Encabezado />

<main>
  {#if flujo.aviso}
    <Aviso tipo="info" alCerrar={() => (flujo.aviso = null)}>{flujo.aviso}</Aviso>
  {/if}
  {#if flujo.error}
    <Aviso tipo="error" titulo="No pudimos cargar los reportes.">{flujo.error.mensaje}</Aviso>
  {/if}

  <div class="fila-cabecera">
    <div class="cabecera">
      <p class="muted">{saludo(hora)}</p>
      <h1 tabindex="-1">¿Qué quieres actualizar hoy?</h1>
      <p class="muted">Elige el reporte. Te pediremos los archivos de Excel y nosotros hacemos el resto.</p>
    </div>
    <Boton
      variante="principal"
      deshabilitado={!flujo.puedeVerTablero.ok}
      motivo={flujo.puedeVerTablero.motivo}
      onclick={() => flujo.verTablero()}
    >
      <Icono nombre="grafico" />Ver el tablero
    </Boton>
  </div>

  {#if flujo.inicio}
    <div class="tarjetas">
      {#each flujo.inicio.tarjetas as tarjeta (tarjeta.estado.reporte.id)}
        <TarjetaReporte {tarjeta} onelegir={() => flujo.elegirReporte(tarjeta.estado.reporte.id)} />
      {/each}
    </div>

    <div class="pie">
      <Icono nombre="carpeta" tamano={22} />
      <span class="muted texto">
        Los reportes se guardan en <strong>{flujo.inicio.carpeta_visible}</strong>. Puedes abrirlos en Excel cuando
        quieras.
      </span>
      <Boton onclick={() => flujo.backend.abrir({ tipo: "carpeta" })}>Abrir carpeta</Boton>
    </div>
  {/if}
</main>

<style>
  main {
    max-width: 1120px;
    margin: 0 auto;
    padding: 48px 40px 56px;
    display: flex;
    flex-direction: column;
    gap: 36px;
  }
  .fila-cabecera {
    display: flex;
    justify-content: space-between;
    align-items: flex-end;
    gap: 24px;
    flex-wrap: wrap;
  }
  .cabecera {
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-width: 640px;
  }
  .cabecera p {
    margin: 0;
  }
  h1 {
    font-size: 40px;
    letter-spacing: -0.01em;
  }
  .tarjetas {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 20px;
  }
  .pie {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 16px 20px;
    border-radius: var(--radio);
    background: var(--color-superficie);
    border: 1px dashed var(--color-borde-fuerte);
    flex-wrap: wrap;
    color: var(--color-texto-2);
  }
  .texto {
    flex: 1 1 320px;
  }
  .texto strong {
    color: var(--color-texto);
  }
</style>
