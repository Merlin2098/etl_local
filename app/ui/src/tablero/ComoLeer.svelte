<script lang="ts">
  // Pantalla 7 «Cómo leer este tablero» (texto del mock v2; ejemplos con los datos vigentes).
  import type { DatosResumen } from "./estado.svelte";
  import { ALCANCE_FILTROS, GLOSARIO, ORDEN_GLOSARIO, calculoConEjemplo, ejemplo } from "./glosario";

  let { datos }: { datos: DatosResumen | null } = $props();
  const entradas = $derived(
    ORDEN_GLOSARIO.map((id) => {
      const g = GLOSARIO[id]!;
      const ej = id === "IND-04" || id === "IND-05" ? ejemplo(id, datos) : null;
      return { id, ...g, ejemplo: ej };
    }),
  );
  const si = (v: boolean) => (v ? "Sí" : "No");
</script>

<div class="como-leer">
  <div class="intro">
    <h1 tabindex="-1">Cómo leer este tablero</h1>
    <p class="muted">
      Cada número responde una pregunta. Aquí está cuál, de dónde sale y cómo se calcula. Los marcados como
      <span class="nuevo">Nuevo</span> no existían en el reporte anterior.
    </p>
  </div>

  <section aria-labelledby="g-ind">
    <h2 id="g-ind">Indicadores</h2>
    <dl class="lista">
      {#each entradas as e (e.id)}
        <div class="def">
          <dt>{e.nombre}{#if e.nuevo}<span class="nuevo">Nuevo</span>{/if}</dt>
          <dd>
            <span>{e.pregunta}</span>
            <span class="lin"
              ><span class="et">Cálculo</span><span>{calculoConEjemplo(e.calculo, e.ejemplo)}</span></span
            >
            <span class="lin"
              ><span class="et">Fuente</span><span
                >{e.fuente} · {#each e.reglas as r, k (r)}{#if k > 0}{" "}{/if}<span class="regla">{r}</span>{/each}</span
              ></span
            >
          </dd>
        </div>
      {/each}
    </dl>
  </section>

  <section aria-labelledby="g-dos">
    <h2 id="g-dos">¿Por qué algunas personas no aparecen al filtrar por gerencia?</h2>
    <div class="caja">
      <p>
        Los números salen de la <strong>planilla</strong>, pero la gerencia, la sede y el tipo de trabajador salen de la
        <strong>base de personal</strong>. Si alguien de la planilla todavía no está en la base (por ejemplo, porque
        ingresó hace pocos días), cuenta en el total pero no aparece al filtrar.
      </p>
      <p class="muted">El tablero te dice cuántas personas están en esa situación y te deja ver quiénes son.</p>
    </div>
  </section>

  <section aria-labelledby="g-fil">
    <h2 id="g-fil">Qué filtros afectan a cada gráfico</h2>
    <div class="tabla">
      <table>
        <thead>
          <tr>
            <th scope="col">Gráfico</th>
            <th scope="col">Periodo</th>
            <th scope="col">Gerencia</th>
            <th scope="col">Sede</th>
            <th scope="col">Tipo de trabajador</th>
          </tr>
        </thead>
        <tbody>
          {#each ALCANCE_FILTROS as f (f.grafico)}
            <tr>
              <td>{f.grafico}</td>
              <td class={f.periodo === "Sí" ? "si" : "no"}>{f.periodo}</td>
              <td class={f.gerencia ? "si" : "no"}>{si(f.gerencia)}</td>
              <td class={f.sede ? "si" : "no"}>{si(f.sede)}</td>
              <td class={f.tipo ? "si" : "no"}>{si(f.tipo)}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  </section>
</div>

<style>
  .como-leer {
    max-width: 1040px;
    display: flex;
    flex-direction: column;
    gap: 28px;
  }
  .intro {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  h1 {
    margin: 0;
    font-size: 34px;
  }
  .intro p {
    margin: 0;
    max-width: 720px;
  }
  section {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  h2 {
    margin: 0;
    font-size: 21px;
  }
  .lista {
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .def {
    display: grid;
    grid-template-columns: 220px minmax(0, 1fr);
    gap: 8px 24px;
    padding: 18px 20px;
    background: var(--color-superficie);
    border: 1px solid var(--color-borde);
    border-radius: 14px;
  }
  dt {
    font-weight: 700;
    font-size: 17px;
  }
  dd {
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .lin {
    display: flex;
    gap: 10px;
    flex-wrap: wrap;
    font-size: 15px;
  }
  .et {
    font-size: 13px;
    font-weight: 700;
    color: var(--color-texto-2);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    min-width: 76px;
    padding-top: 2px;
  }
  .regla {
    font-family: var(--fuente-mono);
    font-size: 13px;
    padding: 1px 6px;
    border-radius: 5px;
    border: 1px solid var(--color-desactivado);
    background: var(--color-fondo);
  }
  .nuevo {
    font-size: 12px;
    font-weight: 700;
    padding: 1px 8px;
    border-radius: 999px;
    background: #e6eef8;
    color: #1e4e86;
    margin-left: 8px;
    vertical-align: 2px;
  }
  .intro .nuevo {
    margin: 0;
  }
  .caja {
    padding: 18px 20px;
    background: var(--color-superficie);
    border: 1px solid var(--color-borde);
    border-radius: 14px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .caja p {
    margin: 0;
  }
  .tabla {
    overflow-x: auto;
    border: 1px solid var(--color-borde);
    border-radius: 12px;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 15px;
    background: var(--color-superficie);
  }
  th,
  td {
    padding: 10px 12px;
    border-bottom: 1px solid var(--color-gris-suave);
    text-align: left;
  }
  th {
    font-size: 13px;
    color: var(--color-texto-2);
    background: var(--color-gris-suave);
  }
  .si {
    color: var(--color-accion-oscura);
    font-weight: 700;
  }
  .no {
    color: var(--color-aviso-texto);
    font-weight: 700;
  }
  @media (max-width: 720px) {
    .def {
      grid-template-columns: 1fr;
    }
  }
</style>
