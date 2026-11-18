# Hoja de ruta de planes de implementación

> Fecha: 2026-11-16 · Cada plan produce software funcional y testeable por sí solo.
> Un plan se escribe en detalle recién cuando el anterior está terminado. Así cada uno
> aprovecha lo aprendido en el previo.

| # | Plan | Entrega | Depende de | Estado |
|---|---|---|---|---|
| 1 | [Generador de data demo](2026-11-16-plan-01-generador-demo.md) | `demo/`: Excel sintéticos con layout legacy (planilla, base de personal, licencias), modos limpio/sucio, casos de error y manifiesto con la verdad de referencia | ADR-0002, `vocabulario.md`, `indicadores-tablero.md` §6 | **Completado** (2026-11-16, 67 tests) |
| 2 | [Paridad con el legacy](2026-11-16-plan-02-paridad-legacy.md) | `generar-golden` en `demo/`: corre el legacy sobre la data demo y guarda sus salidas como *fixtures* con nombres canónicos | Plan 1 | **Completado** (2026-11-16; hallazgos en `demo/README.md`) |
| — | [ADR-0003: formato unificado de contrato](../../adr/0003-formato-contrato-fuente.md) | Formato único de contrato, esquema y mapeo al vocabulario canónico | Planes 1–2 (casos reales para probarlo) | **Aceptado** (2026-11-16) |
| 3 | [Motor: lectura y validación](2026-11-16-plan-03-motor-lectura-validacion.md) | `crates/core`: lectura de Excel con `calamine`, contratos, preflight con mensajes por celda (TR-D13), CLI `crates/cli` (el generador no se migra: sus Excel se validan en los tests; ver ADR-0002) | ADR-0003 | **Completado** (2026-11-16; `etl-local validar` / `leer`) |
| 4 | [Motor: capas Silver/Gold](2026-11-16-plan-04-motor-silver-gold.md) | Planilla, base de personal y licencias a Parquet vía DuckDB; tests de paridad contra el Plan 2 | Plan 3 | **Completado** (2026-11-16; `etl-local procesar`) |
| — | [ADR-0004: API local para el tablero](../../adr/0004-api-local-tablero.md) | API local en Rust (axum) sobre DuckDB nativo; nominal solo en `/detalle/` | Plan 4 | **Aceptado** (2026-11-16) |
| 5 | [Indicadores y API local](2026-11-16-plan-05-indicadores-api.md) | IND-01…08 y checks IND-C1…C3 en SQL; API local que alimenta el tablero; tests contra el manifiesto del Plan 1 | Plan 4 | **Completado** (2026-11-16; `etl-local tablero`) |
| — | [ADR-0005: frontend Svelte 5 + ECharts](../../adr/0005-frontend-svelte-echarts.md) | Svelte 5 + Vite + TS; ECharts 6 (SVG); `app/ui/` con dos entradas; PDF por impresión del sistema | ADR-0001, ADR-0004 | **Aceptado** (2026-11-16) |
| 6 | [App Tauri: flujo de actualización](2026-11-16-plan-06-app-tauri.md) | Pantallas 1–5 de `docs/diseno/` sobre `crates/core`; entradas guardadas por fuente (actualizar un reporte a la vez) y Excel de revisión en `actual/excel/` | Planes 3–5 | **Completado** (2026-11-17; app de escritorio `npm --prefix app/ui run tauri -- dev`) |
| 7 | [Tablero](2026-11-17-plan-07-tablero.md) | Pantallas 6–7 de `docs/diseno/` (Resumen, Alertas y «Cómo leer»), resumen en PDF, tablero servido por la API y abierto desde la app | Planes 5–6 | **Completado** (2026-11-17; tablero en la app o con `etl-local tablero`) |
| 8 | [Dominios restantes](2026-11-17-plan-08-dominios.md) | Planilla régimen especial, altas de personal, practicantes y exámenes médicos de salida con un molde común (contrato + SQL + Excel), sin paridad con el legacy ni cambios en el tablero ([diseño](../specs/2026-11-17-plan-08-dominios-design.md)) | Planes 1–7 | **Completado** (2026-11-18; 4 reportes nuevos en la app con el molde de `config/README.md`) |
| 9 | [Publicación open source](2026-11-18-plan-09-publicacion.md) | README, licencia MIT, CI de pruebas, revisión de confidencialidad, repo con historial nuevo (POC: sin releases; cada developer compila) | Todo lo anterior | Publicado en GitHub; CI en curso |

## Directrices de compilación para los planes pendientes (6–9)

Aprendidas en los Planes 4 y 5 (Windows, DuckDB embebido). Cada plan nuevo las copia en sus
"Restricciones globales" y las aplica en sus pasos de verificación.

1. **Nunca `cargo clean` ni borrar `target/`.** DuckDB se compila desde C++ (feature `bundled`): recompilarlo
   desde cero tarda decenas de minutos. Si hay artefactos dañados, borrar solo los `*.rmeta`/`*.rlib` de tamaño 0
   en `target/debug/deps` y reintentar.
2. **El día a día va en debug.** DuckDB ya se optimiza también en debug (`[profile.dev.package.libduckdb-sys]`
   en `Cargo.toml`), así que los tests y el tablero corren rápido sin compilar en release.
3. **Release solo para medir rendimiento o entregar**, y nunca en paralelo con otra compilación o con los tests.
   Un plan que necesite un binario release lo pide en un paso propio, al final, en segundo plano.
4. **Paralelismo acotado:** compilar con `-j 2` (o `-j 1` si aparece `os error 1455`, archivo de paginación
   insuficiente). No lanzar dos `cargo` a la vez sobre el mismo `target/`.
5. **Medir tiempos con el binario correcto:** los objetivos de rendimiento (por ejemplo, < 500 ms por pedido del
   tablero) se verifican en release; en debug solo se compara contra la medición anterior.
6. **Frontend (Planes 6–7):** `npm ci` con `package-lock.json` versionado; `node_modules/` y `app/ui/dist/`
   fuera del control de versiones, y también excluidos del antivirus si vuelven a aparecer bloqueos.
7. **Tauri (Plan 6):** `tauri dev` compila el backend en debug sobre el mismo `target/` del workspace; no usar
   un `target/` aparte (duplicaría la compilación de DuckDB). `tauri build` cuenta como release (directriz 3).
8. **Requisito del equipo de desarrollo:** `target\` excluido del antivirus (hecho, 2026-11-16) y archivo de
   paginación administrado por el sistema o de al menos 16 GB. Se documenta en el README del Plan 9.

