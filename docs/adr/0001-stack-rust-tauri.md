# ADR-0001: Rust + Tauri como stack del producto

- **Estado:** Aceptado
- **Fecha:** 2026-11-14
- **Reemplaza a:** —
- **Relacionado:** [ADR-0002](0002-demo-python-uv.md), [00-objetivo.md §5](../00-objetivo.md#5-decisión-de-stack-pendiente), [01-analisis-legacy.md §9](../01-analisis-legacy.md#9-implicancias-para-el-rework)

## Contexto

El legacy `HR_Data_Integration_Pipeline` es una app de escritorio Python (PySide6 + Polars + DuckDB)
distribuida con PyInstaller. El rework busca un producto open source, moderno, con binario único,
y con un dashboard interactivo en `localhost` que reemplace a Power BI.

Del análisis del legacy ([logica-negocio/](../logica-negocio/README.md)) se desprende:

- La lógica de negocio cabe mayormente en **SQL de DuckDB + configuración declarativa**
  (contratos, esquemas, rule packs). El lenguaje nativo hará sobre todo plomería: leer Excel,
  validar contratos, orquestar etapas, emitir progreso, escribir Parquet y servir el dashboard.
- El problema central es **leer Excel operativos desordenados**: cabeceras desplazadas o
  combinadas, filas de totales, valores de error, `.xlsx`, `.xlsm` y (declarado) `.xls`.
- Se necesitan un frontend web (UI de escritorio + dashboard) y empaquetado multiplataforma.

Se evaluaron dos opciones: **Go + Wails** y **Rust + Tauri**.

## Decisión

Usar **Rust** para el núcleo y **Tauri v2** para la aplicación de escritorio.

Componentes base:

| Necesidad | Elección |
|---|---|
| Lectura de Excel | `calamine` |
| Motor SQL / reglas Gold | DuckDB embebido (`duckdb` crate, feature `bundled`) |
| Formato analítico | Parquet (escrito por DuckDB o vía Arrow) |
| Escritura de Excel (revisión opcional) | `rust_xlsxwriter` |
| Configuración | YAML (`serde` + `serde_yaml` o equivalente) con validación de esquema propia |
| Desktop / UI | Tauri v2 + frontend web |
| Dashboard | Frontend web servido en `localhost` sobre los Parquet Gold |

Quedan **fuera de esta decisión** y tendrán ADR propio: el framework del frontend, la librería de
gráficos, la estrategia del dashboard (API local con DuckDB o DuckDB-WASM en el navegador) y el
formato unificado de contratos.

## Alternativas consideradas

### Go + Wails

- **A favor:** curva de aprendizaje baja, iteración rápida, `duckdb-go` con librerías
  precompiladas y `excelize` muy completo para escribir Excel.
- **En contra:** `excelize` no lee `.xls`; el ecosistema de datos (Arrow, Polars, DataFusion) es
  más limitado; Wails tiene una comunidad y un ecosistema de plugins más pequeños que Tauri.
- **Motivo del descarte:** la lectura robusta de Excel es el núcleo del producto, y el valor de
  portafolio en data engineering es menor.

### Mantener Python y modernizar

Descartado por el objetivo explícito de cambiar de stack y eliminar la distribución con PyInstaller.
Python se conserva solo como herramienta de demo y paridad ([ADR-0002](0002-demo-python-uv.md)).

## Consecuencias

### Positivas

- `calamine` es la mejor opción disponible para leer Excel sucios, y además lee `.xls` y `.ods`.
- Binarios pequeños y rápidos; Tauri resuelve diálogos de archivos, empaquetado multiplataforma y auto-update.
- Continuidad con el ecosistema de datos moderno (Arrow, Polars, DataFusion) por si se necesitan más adelante.
- Los SQL del legacy se reutilizan sobre el mismo motor (DuckDB).

### Negativas y mitigaciones

| Costo | Mitigación |
|---|---|
| Curva de aprendizaje de Rust | Mantener la lógica en SQL y configuración; el código Rust es acotado y con tipos simples |
| Compilación lenta de DuckDB `bundled` | Cachear `target/` en CI; usar `cargo check` en el ciclo local; evaluar enlazar una librería DuckDB precompilada si se vuelve un problema |
| Requiere toolchain C/C++ en Windows (MSVC) | Documentarlo en el README de desarrollo |
| Dos ecosistemas en el repo (Rust + JS/TS del frontend) | Workspace claro y comandos unificados (ver `AGENTS.md`) |

## Referencias

- [00-objetivo.md](../00-objetivo.md)
- [01-analisis-legacy.md §9](../01-analisis-legacy.md#9-implicancias-para-el-rework)
- [logica-negocio/reglas-transversales.md](../logica-negocio/reglas-transversales.md)
