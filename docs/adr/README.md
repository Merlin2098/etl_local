# Architecture Decision Records

| ADR | Título | Estado | Fecha |
|---|---|---|---|
| [ADR-0001](0001-stack-rust-tauri.md) | Rust + Tauri como stack del producto | Aceptado | 2026-11-14 |
| [ADR-0002](0002-demo-python-uv.md) | Python con uv en `demo/` para datos sintéticos y paridad | Aceptado | 2026-11-14 |
| [ADR-0003](0003-formato-contrato-fuente.md) | Formato unificado de contrato de fuente | Aceptado | 2026-11-16 |
| [ADR-0004](0004-api-local-tablero.md) | API local con DuckDB nativo para el tablero | Aceptado | 2026-11-16 |
| [ADR-0005](0005-frontend-svelte-echarts.md) | Frontend con Svelte 5 y gráficos con Apache ECharts | Aceptado | 2026-11-16 |

## Pendientes previstos

- Decisiones de paridad y corrección ([reglas-transversales.md §5](../logica-negocio/reglas-transversales.md#5-decisiones-pendientes-para-el-rework))

## Cómo agregar un ADR

1. Crear `NNNN-titulo-corto.md` con el siguiente número libre.
2. Incluir las secciones: Contexto, Decisión, Alternativas consideradas, Consecuencias y Referencias.
3. Agregarlo a la tabla de este índice.
4. Si reemplaza a otro, marcar el anterior como `Reemplazado por ADR-NNNN`.
