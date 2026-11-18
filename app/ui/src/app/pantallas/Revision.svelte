<script lang="ts">
  import Aviso from "../../lib/componentes/Aviso.svelte";
  import Boton from "../../lib/componentes/Boton.svelte";
  import Encabezado from "../../lib/componentes/Encabezado.svelte";
  import Icono from "../../lib/componentes/Icono.svelte";
  import Pasos from "../../lib/componentes/Pasos.svelte";
  import { numero } from "../../lib/formato";
  import type { Flujo } from "../flujo.svelte";
  import { textoBotonProcesar, textoSeguirSin, tieneError, tituloRevision } from "../textos";
  import { etiquetaArchivo } from "./etiquetas";
  import TarjetaError from "./TarjetaError.svelte";

  let { flujo }: { flujo: Flujo } = $props();
  const reporte = $derived(flujo.reporte!);
  const conError = $derived(flujo.revisiones.filter(tieneError));
  const bien = $derived(flujo.revisiones.filter((r) => !tieneError(r)));
  const puedeSeguirSin = $derived(conError.length > 0 && flujo.sinLosConError.length > 0);

  function personas(n: number | null | undefined): string {
    return n == null ? "" : ` · ${numero(n)} personas`;
  }
</script>

<Encabezado>
  {#snippet acciones()}
    <Boton onclick={() => flujo.volverAlInicio()}><Icono nombre="volver" tamano={18} />Volver al inicio</Boton>
  {/snippet}
</Encabezado>

<main>
  <Pasos actual={2} />

  <div class="cabecera">
    <h1 tabindex="-1">{tituloRevision(flujo.revisiones)}</h1>
    <p class="muted">Revisamos que cada Excel tenga la forma esperada. Así evitamos reportes con datos incompletos.</p>
  </div>

  {#if flujo.error}
    <Aviso tipo="error">{flujo.error.mensaje}</Aviso>
  {/if}

  <section aria-label="Resultado de la revisión">
    {#each bien as r (r.archivo.ruta)}
      {@const avisos = r.problemas.filter((p) => p.problema.severidad === "aviso")}
      <div class="fila">
        <span class="ico" class:av={avisos.length > 0}>
          <Icono nombre={avisos.length > 0 ? "aviso" : "check"} grosor={2.6} />
        </span>
        <div class="texto">
          <strong>{etiquetaArchivo(r.archivo, flujo.fuenteDe(r.archivo.ruta))}</strong>
          <span class="muted">· {r.archivo.nombre}</span>
          {#each avisos as a, i (i)}<div class="muted pequeno">{a.problema.mensaje}</div>{/each}
        </div>
        <span class="muted pequeno">
          {avisos.length > 0 ? "Listo" : "Todo en orden"}{personas(r.personas)}
          <span class="visualmente-oculto">{avisos.length > 0 ? "(con avisos)" : ""}</span>
        </span>
      </div>
    {/each}
    {#each conError as r (r.archivo.ruta)}
      <TarjetaError {flujo} revision={r} />
    {/each}
  </section>

  <div class="pie">
    <Boton onclick={() => (flujo.pantalla = "archivos")}>Atrás</Boton>
    <div class="derecha">
      {#if puedeSeguirSin}
        <Boton onclick={() => flujo.seguirSin()}>{textoSeguirSin(flujo.revisiones)}</Boton>
      {/if}
      <Boton
        variante="principal"
        deshabilitado={!flujo.puedeProcesar.ok}
        motivo={flujo.puedeProcesar.motivo}
        onclick={() => flujo.procesar()}
      >
        {textoBotonProcesar(reporte, flujo.elegidos.length)}
      </Boton>
    </div>
  </div>
</main>

<style>
  main {
    max-width: 920px;
    margin: 0 auto;
    padding: 36px 40px 48px;
    display: flex;
    flex-direction: column;
    gap: 24px;
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
  section {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .fila {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 14px 18px;
    background: var(--color-superficie);
    border: 1px solid var(--color-borde);
    border-radius: var(--radio);
    flex-wrap: wrap;
  }
  .ico {
    width: 36px;
    height: 36px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    flex: none;
    background: var(--color-accion-suave);
    color: var(--color-accion);
  }
  .ico.av {
    background: var(--color-aviso-fondo);
    color: #7a4200;
  }
  .texto {
    flex: 1 1 300px;
  }
  .pequeno {
    font-size: 15px;
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
  .derecha {
    display: flex;
    gap: 12px;
    align-items: flex-start;
    flex-wrap: wrap;
  }
</style>
