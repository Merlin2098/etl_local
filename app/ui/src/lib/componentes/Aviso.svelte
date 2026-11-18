<script lang="ts">
  // Aviso y error nunca se distinguen solo por color: siempre llevan ícono y palabra (diseno §Lenguaje visual).
  import type { Snippet } from "svelte";
  import Icono from "./Icono.svelte";

  let {
    tipo = "aviso",
    titulo,
    children,
    alCerrar,
  }: { tipo?: "aviso" | "error" | "info"; titulo?: string; children: Snippet; alCerrar?: () => void } = $props();
</script>

<div class="caja {tipo}" role={tipo === "error" ? "alert" : "status"}>
  <Icono nombre={tipo === "info" ? "check" : "triangulo"} tamano={22} />
  <div class="texto">
    {#if titulo}<strong>{titulo}</strong>{/if}
    {@render children()}
  </div>
  {#if alCerrar}
    <button type="button" class="cerrar" aria-label="Cerrar aviso" onclick={alCerrar}>
      <Icono nombre="cerrar" />
    </button>
  {/if}
</div>

<style>
  .caja {
    display: flex;
    align-items: flex-start;
    gap: 14px;
    padding: 14px 18px;
    border-radius: var(--radio);
  }
  .caja.aviso {
    background: var(--color-aviso-fondo);
    color: var(--color-aviso-texto);
  }
  .error {
    background: var(--color-error-fondo);
    color: var(--color-error);
    border: 1px solid var(--color-error);
  }
  .info {
    background: var(--color-accion-suave);
    color: var(--color-accion-oscura);
  }
  .texto {
    flex: 1 1 320px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .cerrar {
    width: var(--alto-control);
    height: var(--alto-control);
    border: 0;
    background: none;
    color: inherit;
    cursor: pointer;
    border-radius: 10px;
    display: flex;
    align-items: center;
    justify-content: center;
  }
</style>
