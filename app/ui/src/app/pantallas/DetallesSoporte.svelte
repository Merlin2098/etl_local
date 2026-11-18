<script lang="ts">
  // Único lugar donde se ven códigos de regla y nombres técnicos de etapa (diseno §Principios 3).
  import type { ErrorApp, Ejecucion } from "../tipos";

  let { ejecucion = null, error = null }: { ejecucion?: Ejecucion | null; error?: ErrorApp | null } = $props();
  const problemas = $derived([...(error?.problemas ?? []), ...(ejecucion?.problemas ?? [])]);
</script>

<details>
  <summary>Ver detalles para soporte</summary>
  <div class="contenido">
    {#if error}
      <p><strong>Error:</strong> {error.codigo} — {error.mensaje}</p>
    {/if}
    {#if ejecucion}
      <p><strong>Ejecución:</strong> {ejecucion.marca} · motor {ejecucion.version_motor}</p>
      <ul>
        {#each ejecucion.etapas as e (e.nombre)}
          <li>{e.nombre}: {e.estado}{e.filas !== null ? ` (${e.filas} filas)` : ""}</li>
        {/each}
      </ul>
    {/if}
    {#if problemas.length > 0}
      <ul>
        {#each problemas as p, i (i)}
          <li>
            <code>{p.codigo}</code> [{p.severidad}] {p.archivo}{p.hoja ? ` · ${p.hoja}` : ""}{p.celda ? ` · ${p.celda}` : ""}:
            {p.mensaje}{p.detalle ? ` — ${p.detalle}` : ""}
          </li>
        {/each}
      </ul>
    {/if}
  </div>
</details>

<style>
  summary {
    color: var(--color-accion);
    text-decoration: underline;
    cursor: pointer;
    min-height: var(--alto-control);
    display: inline-flex;
    align-items: center;
  }
  .contenido {
    user-select: text;
    font-size: 15px;
    color: var(--color-texto-2);
    padding: 8px 0;
  }
  code {
    font-family: var(--fuente-mono);
  }
</style>
