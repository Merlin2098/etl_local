<script lang="ts">
  // Filtros globales: el filtrado ocurre en el servidor (ADR-0004 §2); aquí solo se eligen los valores.
  import { nombreMes } from "../lib/formato";
  import type { ClaveFiltro, EstadoTablero } from "./estado.svelte";

  let { estado }: { estado: EstadoTablero } = $props();
  const o = $derived(estado.datos?.opciones);
  const periodo = $derived(estado.filtros.periodo ?? estado.datos?.indicadores.periodo ?? "");

  function cambiar(clave: ClaveFiltro, e: Event) {
    const v = (e.currentTarget as HTMLSelectElement).value;
    void estado.cambiarFiltro(clave, v || null);
  }
</script>

{#if o}
  <div class="filtros">
    <label>
      Periodo
      <select value={periodo} onchange={(e) => cambiar("periodo", e)}>
        {#each [...o.periodos].reverse() as p (p)}<option value={p}>{nombreMes(p)}</option>{/each}
      </select>
    </label>
    <label>
      Gerencia
      <select value={estado.filtros.gerencia ?? ""} onchange={(e) => cambiar("gerencia", e)}>
        <option value="">Todas</option>
        {#each o.gerencias as g (g)}<option value={g}>{g}</option>{/each}
      </select>
    </label>
    <label>
      Sede
      <select value={estado.filtros.sede ?? ""} onchange={(e) => cambiar("sede", e)}>
        <option value="">Todas</option>
        {#each o.sedes as s (s)}<option value={s}>{s}</option>{/each}
      </select>
    </label>
    <label>
      Tipo de trabajador
      <select value={estado.filtros.tipo_trabajador ?? ""} onchange={(e) => cambiar("tipo_trabajador", e)}>
        <option value="">Todos</option>
        {#each o.tipos_trabajador as t (t)}<option value={t}>{t}</option>{/each}
      </select>
    </label>
  </div>
{/if}

<style>
  .filtros {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 12px;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 14px;
    font-weight: 700;
    color: var(--color-texto-2);
  }
  select {
    font: inherit;
    font-size: 16px;
    font-weight: 400;
    color: var(--color-texto);
    min-height: var(--alto-control);
    padding: 0 12px;
    border: 1px solid var(--color-borde-fuerte);
    border-radius: 10px;
    background: var(--color-superficie);
  }
  @media (max-width: 760px) {
    .filtros {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
</style>
