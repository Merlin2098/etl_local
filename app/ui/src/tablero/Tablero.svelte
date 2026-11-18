<script lang="ts">
  // Pantalla 6 (Tablero) y 7 (Cómo leer este tablero), docs/diseno. Los datos vienen de la API local.
  import { onMount, tick } from "svelte";
  import { descargar } from "../lib/csv";
  import { nombreMes } from "../lib/formato";
  import Aviso from "../lib/componentes/Aviso.svelte";
  import Boton from "../lib/componentes/Boton.svelte";
  import Encabezado from "../lib/componentes/Encabezado.svelte";
  import ComoLeer from "./ComoLeer.svelte";
  import DatosUsados from "./DatosUsados.svelte";
  import Filtros from "./Filtros.svelte";
  import Alertas from "./Alertas.svelte";
  import Ausencias from "./Ausencias.svelte";
  import Movimientos from "./Movimientos.svelte";
  import PorGerencia from "./PorGerencia.svelte";
  import Riesgos from "./Riesgos.svelte";
  import Tarjetas from "./Tarjetas.svelte";
  import type { EstadoTablero, Pestana } from "./estado.svelte";
  import { csvIndicadores, lineaFiltros } from "./textos";

  let { estado }: { estado: EstadoTablero } = $props();

  function desdeHash(): Pestana {
    const h = location.hash.slice(1);
    return h === "alertas" || h === "como-leer" ? h : "resumen";
  }

  async function ir(p: Pestana) {
    if (location.hash.slice(1) !== (p === "resumen" ? "" : p)) {
      history.replaceState(null, "", `${location.pathname}${location.search}${p === "resumen" ? "" : `#${p}`}`);
    }
    await estado.irA(p);
  }

  let generado = $state(new Date());
  const dosCifras = (n: number) => String(n).padStart(2, "0");
  const pie = $derived(
    `Generado el ${dosCifras(generado.getDate())}/${dosCifras(generado.getMonth() + 1)}/${generado.getFullYear()} a las ${dosCifras(generado.getHours())}:${dosCifras(generado.getMinutes())}`,
  );

  /** «Resumen en PDF (1 página)»: el diálogo de impresión del sistema, sobre el Resumen (ADR-0005 §4). */
  async function imprimir() {
    if (estado.pestana !== "resumen") await ir("resumen");
    generado = new Date();
    await tick();
    window.print();
  }

  function datosEnExcel() {
    if (d) descargar(`tablero-${d.indicadores.periodo ?? "datos"}.csv`, csvIndicadores(d, estado.filtros));
  }

  onMount(() => {
    const alCambiarHash = () => void estado.irA(desdeHash());
    window.addEventListener("hashchange", alCambiarHash);
    void estado.cargar().then(() => estado.irA(desdeHash()));
    return () => window.removeEventListener("hashchange", alCambiarHash);
  });

  const d = $derived(estado.datos);
  const ind01 = $derived(d?.indicadores.indicadores.find((i) => i.id === "IND-01"));
  const contextoCsv = $derived(d ? `Filtros: ${lineaFiltros(estado.filtros, d.indicadores.periodo)}` : "");
  const ind05 = $derived(d?.indicadores.indicadores.find((i) => i.id === "IND-05"));
</script>

<Encabezado titulo="Tablero RR. HH.">
  {#snippet acciones()}
    {#if estado.pestana === "como-leer"}
      <Boton onclick={() => ir("resumen")}>Volver al tablero</Boton>
    {:else}
      <Boton variante="enlace" onclick={() => ir("como-leer")}>Cómo leer este tablero</Boton>
      {#if d}
        <Boton onclick={datosEnExcel}>Datos en Excel</Boton>
        <Boton onclick={imprimir}>Resumen en PDF (1 página)</Boton>
      {/if}
    {/if}
  {/snippet}
</Encabezado>

{#if estado.pestana !== "como-leer"}
  <nav class="pestanas" aria-label="Páginas del tablero">
    <button type="button" aria-current={estado.pestana === "resumen" ? "page" : undefined} onclick={() => ir("resumen")}>
      Resumen
    </button>
    <button type="button" aria-current={estado.pestana === "alertas" ? "page" : undefined} onclick={() => ir("alertas")}>
      Alertas (solo RR. HH.)
    </button>
  </nav>
{/if}

<main>
  {#if estado.aviso}
    <Aviso alCerrar={() => (estado.aviso = null)}>{estado.aviso}</Aviso>
  {/if}

  {#if estado.pestana === "como-leer"}
    <ComoLeer datos={d} />
  {:else if estado.error}
    <Aviso tipo="error" titulo="No se pudo cargar el tablero">
      <p>{estado.error.message}</p>
      <Boton onclick={() => estado.cargar()}>Volver a intentar</Boton>
    </Aviso>
  {:else if estado.vacio}
    <section class="vacio">
      <h1>Todavía no hay datos para el tablero</h1>
      <p class="muted">Procesa la planilla o la base de personal en la app y vuelve aquí.</p>
    </section>
  {:else if d}
    <div class="solo-impresion">
      <h1>Resumen RR. HH.{d.indicadores.periodo ? ` · ${nombreMes(d.indicadores.periodo)}` : ""}</h1>
      <p>{lineaFiltros(estado.filtros, d.indicadores.periodo)}</p>
    </div>
    <DatosUsados estado={d.estado} periodo={d.indicadores.periodo} />
    <Filtros {estado} />
    {#if estado.pestana === "resumen"}
      <div class="resumen" id="resumen">
        <Tarjetas datos={d} alVerQuienes={() => ir("alertas")} />
        <div class="rejilla">
          <Riesgos datos={d} alVerDetalle={() => ir("alertas")} />
          <Movimientos series={d.series} contexto={contextoCsv} />
          {#if ind01}
            <PorGerencia
              indicador={ind01}
              periodo={d.indicadores.periodo}
              sinBase={d.checks.checks.find((c) => c.id === "IND-C1")?.valor ?? 0}
              alElegir={(g) => estado.cambiarFiltro("gerencia", g)}
              contexto={contextoCsv}
            />
          {/if}
          {#if ind05}<Ausencias indicador={ind05} periodo={d.indicadores.periodo} contexto={contextoCsv} />{/if}
        </div>
      </div>
    {:else if estado.pestana === "alertas"}
      <Alertas detalle={estado.detalle} />
    {/if}
    <p class="solo-impresion pie">{pie}</p>
  {/if}
</main>

<style>
  .pestanas {
    display: flex;
    gap: 4px;
    padding: 0 40px;
    background: var(--color-superficie);
    border-bottom: 1px solid var(--color-borde);
    overflow-x: auto;
  }
  .pestanas button {
    font: inherit;
    font-size: 16px;
    min-height: var(--alto-control);
    padding: 0 16px;
    border: 0;
    border-bottom: 3px solid transparent;
    background: none;
    color: var(--color-texto-2);
    cursor: pointer;
    white-space: nowrap;
  }
  .pestanas button[aria-current="page"] {
    color: var(--color-texto);
    font-weight: 700;
    border-bottom-color: var(--color-accion);
  }
  main {
    max-width: 1360px;
    margin: 0 auto;
    padding: 24px 40px 48px;
    display: flex;
    flex-direction: column;
    gap: 20px;
  }
  .resumen {
    display: flex;
    flex-direction: column;
    gap: 20px;
  }
  .rejilla {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 20px;
    align-items: start;
  }
  @media (max-width: 1000px) {
    .rejilla {
      grid-template-columns: 1fr;
    }
  }
  .vacio {
    padding: 48px 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .vacio h1 {
    font-size: 30px;
    margin: 0;
  }
  @media (max-width: 760px) {
    .pestanas {
      padding: 0 16px;
    }
    main {
      padding: 16px 16px 40px;
    }
  }
</style>
