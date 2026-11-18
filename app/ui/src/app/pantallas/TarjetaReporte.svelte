<script lang="ts">
  import Icono, { type NombreIcono } from "../../lib/componentes/Icono.svelte";
  import { textoVigencia } from "../textos";
  import type { TarjetaReporte } from "../tipos";

  let { tarjeta, onelegir }: { tarjeta: TarjetaReporte; onelegir: () => void } = $props();

  const ICONOS: Record<string, NombreIcono> = { planilla: "planilla", base_personal: "personas" };
  const estado = $derived(tarjeta.estado);
  const clase = $derived({ al_dia: "ok", atrasado: "viejo", nunca: "nunca" }[estado.vigencia]);
</script>

<button type="button" class="tarjeta" onclick={onelegir}>
  <span class="icono"><Icono nombre={ICONOS[estado.reporte.fuente] ?? "planilla"} tamano={26} /></span>
  <span class="titulos">
    <span class="h2">{estado.reporte.titulo}</span>
    <span class="muted descripcion">{estado.reporte.descripcion}</span>
  </span>
  <span class="estado {clase}"><span class="punto" aria-hidden="true"></span>{textoVigencia(estado)}</span>
  <span class="ir">
    {estado.vigencia === "nunca" ? "Cargar por primera vez" : "Actualizar"}
    <Icono nombre="flecha" tamano={18} grosor={2.4} />
  </span>
</button>

<style>
  .tarjeta {
    font: inherit;
    text-align: left;
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 24px;
    background: var(--color-superficie);
    border: 1px solid var(--color-borde);
    border-radius: 16px;
    color: var(--color-texto);
    min-height: 200px;
    cursor: pointer;
  }
  .tarjeta:hover {
    border-color: var(--color-accion);
    box-shadow: 0 0 0 3px var(--color-accion-suave);
  }
  .icono {
    width: 48px;
    height: 48px;
    border-radius: 12px;
    background: var(--color-accion-suave);
    color: var(--color-accion);
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .titulos {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .h2 {
    font-family: var(--fuente-titulo);
    font-weight: 700;
    font-size: 21px;
    line-height: 1.25;
  }
  .descripcion {
    font-size: 16px;
  }
  .estado {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    font-size: 15px;
    padding: 4px 10px;
    border-radius: 999px;
    align-self: flex-start;
  }
  .ok {
    background: var(--color-accion-suave);
    color: var(--color-accion-oscura);
  }
  .viejo {
    background: var(--color-aviso-fondo);
    color: #7a4200;
  }
  .nunca {
    background: var(--color-gris-suave);
    color: #454f49;
  }
  .punto {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: currentColor;
  }
  .ir {
    margin-top: auto;
    display: flex;
    align-items: center;
    gap: 8px;
    font-weight: 700;
    color: var(--color-accion);
  }
</style>
