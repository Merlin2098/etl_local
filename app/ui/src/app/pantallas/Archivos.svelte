<script lang="ts">
  import Aviso from "../../lib/componentes/Aviso.svelte";
  import Boton from "../../lib/componentes/Boton.svelte";
  import Encabezado from "../../lib/componentes/Encabezado.svelte";
  import Icono from "../../lib/componentes/Icono.svelte";
  import Pasos from "../../lib/componentes/Pasos.svelte";
  import { fechaCorta } from "../../lib/formato";
  import type { Flujo } from "../flujo.svelte";
  import FilaArchivo from "./FilaArchivo.svelte";
  import ZonaArrastre from "./ZonaArrastre.svelte";

  let { flujo }: { flujo: Flujo } = $props();
  const reporte = $derived(flujo.reporte!);
  const n = $derived(flujo.elegidos.length);
  const complemento = $derived(reporte.complementos?.[0] ?? null);

  async function cambiarComplemento() {
    const [ruta] = await flujo.backend.elegirArchivos(false);
    if (ruta) await flujo.cambiarComplemento(ruta);
  }
</script>

<Encabezado>
  {#snippet acciones()}
    <Boton onclick={() => flujo.volverAlInicio()}><Icono nombre="volver" tamano={18} />Volver al inicio</Boton>
  {/snippet}
</Encabezado>

<main>
  <Pasos actual={1} />

  <div class="cabecera">
    <h1 tabindex="-1">{reporte.titulo}: elige los archivos de Excel</h1>
    <p class="muted">
      {#if reporte.varios_archivos}
        Puedes elegir uno o varios meses a la vez. Reconocemos el mes por el nombre del archivo.
      {:else}
        Elige un archivo. Reconocemos la fecha de corte por el nombre del archivo.
      {/if}
    </p>
  </div>

  {#if flujo.aviso}
    <Aviso alCerrar={() => (flujo.aviso = null)}>{flujo.aviso}</Aviso>
  {/if}
  {#if flujo.error}
    <Aviso tipo="error">{flujo.error.mensaje}</Aviso>
  {/if}

  <ZonaArrastre {flujo} />

  {#if n > 0}
    <section class="lista" aria-label="Archivos elegidos">
      <p class="cuenta">{n === 1 ? "1 archivo elegido" : `${n} archivos elegidos`}</p>
      {#each flujo.elegidos as archivo (archivo.ruta)}
        <FilaArchivo {archivo} onquitar={() => flujo.quitar(archivo.ruta)} />
      {/each}
    </section>
  {/if}

  {#if complemento === "licencias"}
    <div class="complemento">
      <input type="checkbox" id="complemento" bind:checked={flujo.complemento.usar} />
      <label for="complemento">
        <span class="fuerte">Agregar también el control de licencias</span>
        <span class="muted pequeno">
          {#if flujo.complemento.archivo}
            Usaremos <strong>{flujo.complemento.archivo.nombre}</strong>
            {#if flujo.complemento.archivo.modificado}(actualizado el {fechaCorta(flujo.complemento.archivo.modificado)}).{/if}
          {:else}
            Todavía no hay un control de licencias guardado.
          {/if}
        </span>
      </label>
      <Boton variante="enlace" onclick={cambiarComplemento}>
        {flujo.complemento.archivo ? "Cambiar archivo" : "Elegir archivo"}
      </Boton>
    </div>
  {/if}

  <div class="pie">
    <Boton onclick={() => flujo.volverAlInicio()}>Cancelar</Boton>
    <Boton
      variante="principal"
      deshabilitado={!flujo.puedeRevisar.ok || flujo.revisando}
      motivo={flujo.revisando ? "Revisando los archivos…" : flujo.puedeRevisar.motivo}
      onclick={() => flujo.revisar()}
    >
      Revisar archivos <Icono nombre="flecha" grosor={2.4} />
    </Boton>
  </div>
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
  .lista {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .cuenta {
    margin: 0;
    font-weight: 700;
  }
  .complemento {
    display: flex;
    gap: 12px;
    align-items: flex-start;
    flex-wrap: wrap;
    padding: 14px 18px;
    border-radius: var(--radio);
    background: var(--color-superficie);
    border: 1px solid var(--color-borde);
  }
  .complemento input {
    width: 22px;
    height: 22px;
    margin: 2px 0 0;
    accent-color: var(--color-accion);
    flex: none;
  }
  .complemento label {
    display: flex;
    flex-direction: column;
    gap: 2px;
    cursor: pointer;
    flex: 1 1 320px;
  }
  .fuerte {
    font-weight: 700;
  }
  .pequeno {
    font-size: 15px;
  }
  .pequeno strong {
    color: var(--color-texto);
  }
  .pie {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 16px;
    flex-wrap: wrap;
    padding-top: 16px;
    border-top: 1px solid var(--color-borde);
  }
</style>
