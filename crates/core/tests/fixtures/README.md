# Fixtures de los tests del motor

`demo/` contiene Excel **sintéticos** generados por `generar-demo` (Plan 1). Sirven para verificar que los
contratos de `config/fuentes/` describen el layout del generador:

- `limpio` y `sucio` no deben dar errores;
- cada `CASO-xx` debe dar exactamente su error.

Se regeneran **solo** con el generador, nunca a mano. Desde la raíz del repositorio, después de borrar
`crates/core/tests/fixtures/demo/`:

```bash
uv run --project demo generar-demo --modo limpio --meses 3 --salida crates/core/tests/fixtures/demo
uv run --project demo generar-demo --modo sucio  --meses 3 --salida crates/core/tests/fixtures/demo
uv run --project demo generar-demo --modo casos  --meses 1 --salida crates/core/tests/fixtures/demo
```

`limpio` y `sucio` usan la misma semilla y los mismos parámetros, así que tienen la misma población: los tests
comparan sus lecturas fila por fila. Con 3 meses aparece la columna dinámica `DIAS SUB. MATERNIDAD`, que se
genera desde el tercer mes.
