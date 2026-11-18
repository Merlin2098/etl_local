<script lang="ts">
  // «Alertas (solo RR. HH.)» (TR-D18): las listas con nombres. Nunca se imprime ni entra en el resumen.
  import Aviso from "../lib/componentes/Aviso.svelte";
  import Boton from "../lib/componentes/Boton.svelte";
  import { descargar } from "../lib/csv";
  import { numero } from "../lib/formato";
  import type { RespuestaDetalle } from "../lib/api/tipos.gen";
  import { COLUMNAS_ALERTAS, COLUMNAS_SIN_BASE, celdasAlerta, celdasSinBase, csvDetalle } from "./alertas";

  let { detalle }: { detalle: { alertas: RespuestaDetalle; sinBase: RespuestaDetalle } | null } = $props();

  const secciones = $derived(
    detalle
      ? [
          {
            id: "s-alertas",
            titulo: `Personas con alertas (${numero(detalle.alertas.filas.length)})`,
            columnas: COLUMNAS_ALERTAS,
            filas: detalle.alertas.filas.map(celdasAlerta),
            archivo: "personas-con-alertas.csv",
          },
          {
            id: "s-sin-base",
            titulo: `Personas de la planilla sin base de personal (${numero(detalle.sinBase.filas.length)})`,
            columnas: COLUMNAS_SIN_BASE,
            filas: detalle.sinBase.filas.map(celdasSinBase),
            archivo: "personas-sin-base-de-personal.csv",
          },
        ]
      : [],
  );
</script>

<div class="alertas">
  <h1>Alertas (solo RR. HH.)</h1>
  <Aviso>Esta pestaña muestra nombres. Es solo para RR. HH.: no la compartas.</Aviso>
  {#if !detalle}
    <p class="muted">Cargando…</p>
  {/if}
  {#each secciones as s (s.id)}
    <section class="panel" aria-labelledby={s.id}>
      <div class="cabeza">
        <h2 id={s.id}>{s.titulo}</h2>
        {#if s.filas.length > 0}
          <Boton onclick={() => descargar(s.archivo, csvDetalle(s.columnas, s.filas))}>Descargar CSV</Boton>
        {/if}
      </div>
      {#if s.filas.length === 0}
        <p class="muted">Sin datos para este filtro.</p>
      {:else}
        <div class="tabla">
          <table>
            <thead><tr>{#each s.columnas as c (c)}<th scope="col">{c}</th>{/each}</tr></thead>
            <tbody>
              {#each s.filas as f, k (k)}
                <tr>{#each f as v, j (j)}<td>{v}</td>{/each}</tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}
    </section>
  {/each}
</div>

<style>
  .alertas {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  h1 {
    margin: 0;
    font-size: 28px;
  }
  .panel {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 20px 22px;
    background: var(--color-superficie);
    border: 1px solid var(--color-borde);
    border-radius: 16px;
  }
  .cabeza {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
  }
  h2 {
    margin: 0;
    font-size: 20px;
  }
  .tabla {
    max-height: 520px;
    overflow: auto;
    border: 1px solid var(--color-borde);
    border-radius: 10px;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 15px;
  }
  th,
  td {
    padding: 8px 12px;
    border-bottom: 1px solid var(--color-gris-suave);
    text-align: left;
    vertical-align: top;
  }
  th {
    position: sticky;
    top: 0;
    font-size: 13px;
    color: var(--color-texto-2);
    background: var(--color-gris-suave);
  }
  @media print {
    .alertas {
      display: none;
    }
  }
</style>
