<script lang="ts">
  import Boton from "../../lib/componentes/Boton.svelte";
  import ChipCelda from "../../lib/componentes/ChipCelda.svelte";
  import Icono from "../../lib/componentes/Icono.svelte";
  import RecorteHoja from "../../lib/componentes/RecorteHoja.svelte";
  import type { Flujo } from "../flujo.svelte";
  import { pasosArreglo } from "../textos";
  import type { RevisionArchivo } from "../tipos";
  import { etiquetaArchivo } from "./etiquetas";

  let { flujo, revision }: { flujo: Flujo; revision: RevisionArchivo } = $props();
  const errores = $derived(revision.problemas.filter((p) => p.problema.severidad === "error"));
  const primero = $derived(errores[0]);
  const resto = $derived(errores.slice(1));
</script>

<article class="tarjeta">
  <div class="cabeza">
    <span class="ico"><Icono nombre="cerrar" grosor={2.6} /></span>
    <div class="titulos">
      <div>
        <strong>{etiquetaArchivo(revision.archivo, flujo.fuenteDe(revision.archivo.ruta))}</strong>
        <span class="muted">· {revision.archivo.nombre}</span>
      </div>
      {#if primero}
        <p class="mensaje">
          {primero.problema.codigo === "PRE-10" && primero.problema.celda_encontrada
            ? "Los títulos de las columnas no están donde deberían."
            : primero.problema.mensaje}
        </p>
        {#if primero.problema.celda && primero.problema.hoja}
          <p class="ubicacion">
            {#if primero.problema.celda_encontrada}
              Deberían empezar en <ChipCelda hoja={primero.problema.hoja} celda={primero.problema.celda} />, pero los
              encontramos en <ChipCelda celda={primero.problema.celda_encontrada} conIcono={false} />.
            {:else}
              Mira la <ChipCelda hoja={primero.problema.hoja} celda={primero.problema.celda} />.
            {/if}
          </p>
        {/if}
      {/if}
    </div>
  </div>

  {#if primero?.recorte}
    <div class="sangria">
      <RecorteHoja
        recorte={primero.recorte}
        esperada={primero.problema.celda}
        encontrada={primero.problema.celda_encontrada ?? null}
      />
    </div>
  {/if}

  {#if resto.length > 0}
    <ul class="sangria otros">
      {#each resto as p, i (i)}
        <li>
          {p.problema.mensaje}
          {#if p.problema.celda}<ChipCelda hoja={p.problema.hoja} celda={p.problema.celda} />{/if}
        </li>
      {/each}
    </ul>
  {/if}

  {#if primero}
    <div class="sangria arreglo">
      <p class="fuerte">Cómo arreglarlo</p>
      <ol>
        {#each pasosArreglo(primero.problema) as paso (paso)}<li>{paso}</li>{/each}
      </ol>
      <div class="acciones">
        <Boton onclick={() => flujo.backend.abrir({ tipo: "archivo", ruta: revision.archivo.ruta })}>
          <Icono nombre="externo" tamano={18} /><strong>Abrir en Excel</strong>
        </Boton>
        <Boton onclick={() => flujo.revisarDeNuevo()}><Icono nombre="recargar" tamano={18} />Revisar de nuevo</Boton>
      </div>
    </div>
  {/if}
</article>

<style>
  .tarjeta {
    display: flex;
    flex-direction: column;
    gap: 18px;
    padding: 22px 24px;
    background: var(--color-superficie);
    border: 2px solid var(--color-error);
    border-radius: 14px;
  }
  .cabeza {
    display: flex;
    align-items: flex-start;
    gap: 14px;
  }
  .ico {
    width: 36px;
    height: 36px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    flex: none;
    background: var(--color-error-fondo);
    color: var(--color-error);
  }
  .titulos {
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-width: 0;
  }
  .mensaje {
    margin: 0;
    font-size: 19px;
    font-weight: 700;
  }
  .ubicacion {
    margin: 0;
    line-height: 1.9;
  }
  .sangria {
    margin-left: 50px;
  }
  .otros {
    margin-top: 0;
    margin-bottom: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .arreglo {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .arreglo p {
    margin: 0;
  }
  .fuerte {
    font-weight: 700;
  }
  ol {
    margin: 0;
    padding-left: 22px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .acciones {
    display: flex;
    gap: 12px;
    flex-wrap: wrap;
    margin-top: 6px;
  }
  @media (max-width: 700px) {
    .sangria {
      margin-left: 0;
    }
  }
</style>
