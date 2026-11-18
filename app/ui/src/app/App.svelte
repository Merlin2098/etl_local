<script lang="ts">
  import { onMount } from "svelte";
  import type { Flujo } from "./flujo.svelte";
  import Archivos from "./pantallas/Archivos.svelte";
  import Inicio from "./pantallas/Inicio.svelte";
  import Listo from "./pantallas/Listo.svelte";
  import Procesando from "./pantallas/Procesando.svelte";
  import Revision from "./pantallas/Revision.svelte";

  let { flujo }: { flujo: Flujo } = $props();

  onMount(() => {
    void flujo.cargarInicio();
  });

  // Al cambiar de pantalla, el foco va al título (lectores de pantalla) y la vista vuelve arriba.
  $effect(() => {
    void flujo.pantalla;
    queueMicrotask(() => {
      window.scrollTo(0, 0);
      document.querySelector<HTMLElement>("h1")?.focus({ preventScroll: true });
    });
  });
</script>

{#if flujo.pantalla === "inicio"}
  <Inicio {flujo} />
{:else if flujo.pantalla === "archivos"}
  <Archivos {flujo} />
{:else if flujo.pantalla === "revision"}
  <Revision {flujo} />
{:else if flujo.pantalla === "procesando"}
  <Procesando {flujo} />
{:else if flujo.pantalla === "listo" && flujo.resultado}
  <Listo {flujo} />
{/if}
