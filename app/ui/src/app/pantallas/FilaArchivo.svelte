<script lang="ts">
  import Icono from "../../lib/componentes/Icono.svelte";
  import { fechaCorta, nombreMes } from "../../lib/formato";
  import type { ArchivoElegido } from "../tipos";

  let { archivo, onquitar }: { archivo: ArchivoElegido; onquitar: () => void } = $props();

  function tamano(bytes: number): string {
    return `${(bytes / 1_000_000).toFixed(1).replace(".", ",")} MB`;
  }

  const chip = $derived.by(() => {
    const f = archivo.fecha_nombre;
    if (!f) return null;
    return "periodo" in f ? nombreMes(f.periodo) : `Corte: ${fechaCorta(f.fecha_corte)}`;
  });
</script>

<div class="archivo">
  <span class="xls"><Icono nombre="excel" tamano={22} /></span>
  <div class="datos">
    <div class="nombre">{archivo.nombre}</div>
    <div class="muted detalle">{archivo.carpeta} · {tamano(archivo.bytes)}</div>
  </div>
  {#if chip}
    <span class="mes">{chip}</span>
  {:else}
    <span class="mes sin">Mes no reconocido</span>
  {/if}
  <button type="button" class="quitar" aria-label={`Quitar ${archivo.nombre}`} onclick={onquitar}>
    <Icono nombre="cerrar" />
  </button>
</div>

<style>
  .archivo {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 14px 18px;
    background: var(--color-superficie);
    border: 1px solid var(--color-borde);
    border-radius: var(--radio);
    flex-wrap: wrap;
  }
  .xls {
    width: 40px;
    height: 40px;
    border-radius: 8px;
    background: var(--color-accion-suave);
    color: var(--color-accion);
    display: flex;
    align-items: center;
    justify-content: center;
    flex: none;
  }
  .datos {
    flex: 1 1 280px;
    min-width: 0;
  }
  .nombre {
    font-weight: 700;
    overflow-wrap: anywhere;
  }
  .detalle {
    font-size: 15px;
  }
  .mes {
    display: inline-flex;
    align-items: center;
    padding: 4px 10px;
    border-radius: 999px;
    background: var(--color-accion-suave);
    color: var(--color-accion-oscura);
    font-size: 15px;
    font-weight: 700;
  }
  .mes.sin {
    background: var(--color-aviso-fondo);
    color: var(--color-aviso-texto);
  }
  .quitar {
    width: var(--alto-control);
    height: var(--alto-control);
    border-radius: 10px;
    border: 1px solid transparent;
    background: none;
    color: var(--color-texto-2);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .quitar:hover {
    border-color: var(--color-borde-fuerte);
  }
</style>
