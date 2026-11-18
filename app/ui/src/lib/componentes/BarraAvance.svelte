<script lang="ts">
  let { porcentaje }: { porcentaje: number } = $props();
  const valor = $derived(Math.max(0, Math.min(100, Math.round(porcentaje))));
</script>

<div class="barra" role="progressbar" aria-valuemin={0} aria-valuemax={100} aria-valuenow={valor} aria-label="Avance">
  <div class="relleno" style:width={`${valor}%`}></div>
</div>

<style>
  .barra {
    height: 14px;
    border-radius: 999px;
    background: #e3e8e2;
    overflow: hidden;
  }
  .relleno {
    height: 100%;
    border-radius: 999px;
    background: var(--color-accion);
    transition: width 0.4s ease-out;
  }
</style>
