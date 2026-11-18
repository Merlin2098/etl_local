# ADR-0004: API local con DuckDB nativo para el tablero

- **Estado:** Aceptado
- **Fecha:** 2026-11-16
- **Relacionado:** [ADR-0001](0001-stack-rust-tauri.md),
  [indicadores-tablero.md](../logica-negocio/indicadores-tablero.md),
  [reglas-transversales.md](../logica-negocio/reglas-transversales.md) (TR-D3, TR-D16…TR-D18),
  [00-objetivo.md §6](../00-objetivo.md#6-dashboard-en-localhost), [diseno/](../diseno/README.md)

## Contexto

ADR-0001 dejó pendiente cómo obtiene los datos el tablero: una API local del binario que consulta DuckDB, o
DuckDB-WASM en el navegador leyendo los Parquet directamente.

Lo que ya existe tras el Plan 4:

- `etl-local procesar` publica `<salida>/actual/` con los Parquet Gold (`planilla`, `base_personal_empleados`,
  `base_personal_practicantes`, `centros_costo`) y `ejecucion.json` (fuentes, fecha de corte, etapas, avisos).
- La lógica de negocio vive en SQL de DuckDB (`config/sql/`) con umbrales en `config/parametros.yaml`, y se
  prueba en Rust contra el manifiesto del demo y contra el legacy.

Requisitos que condicionan la decisión:

- El tablero se abre desde la app **o** desde el navegador en `http://localhost:<puerto>` (00-objetivo §6).
- Filtros globales (periodo, gerencia, sede, tipo de trabajador) con *cross-filtering*: los indicadores cambian
  con cada combinación de filtros.
- Cada número explica su origen: fuente, fecha de corte e IDs de regla (TR-D17).
- Los nombres de personas solo aparecen en la vista de RR. HH. (TR-D18).
- IND-01…08 e IND-C1…C3 deben probarse contra la verdad de referencia del manifiesto (Plan 1).

## Decisión

### 1. API local en Rust sobre DuckDB nativo

Un crate nuevo, `crates/api` (lib `etl_local_api`), expone una API HTTP de **solo lectura** construida con
**axum** sobre **tokio**. Ejecuta el SQL de indicadores con el mismo DuckDB embebido que usa el motor, sobre
los Parquet de `<salida>/actual/`. El navegador y la app Tauri consumen la misma API.

- El CLI la levanta con `etl-local tablero --salida <carpeta> [--puerto <n>]`. En el Plan 7 el mismo servidor
  entrega también los archivos del frontend. La app Tauri (Planes 6–7) reutiliza el router dentro de su
  propio runtime tokio.
- Escucha **solo en `127.0.0.1`** y rechaza los pedidos cuyo `Host` no sea `127.0.0.1:<puerto>` o
  `localhost:<puerto>`. Esto evita que una página web ajena lea los datos mediante *DNS rebinding*.
- No escribe nada en `<salida>/`. Abre los Parquet de `actual/` en cada pedido, así que una nueva ejecución
  de `procesar` se ve sin reiniciar el servidor.

### 2. Indicadores en SQL, calculados al consultar

- Los indicadores y los checks viven en `config/sql/indicadores/`, al lado del SQL de Silver y Gold. Rust no
  contiene fórmulas de negocio.
- Se calculan **en cada pedido**, con los filtros recibidos. Los filtros llegan al SQL como variables de DuckDB
  (`getvariable`), igual que los parámetros del Plan 4, y un filtro ausente significa "todos". Nunca se
  concatena texto del pedido dentro del SQL.
- Las metas (indicadores-tablero §4) se agregan como una sección opcional `metas` en
  `config/parametros.yaml`. Si un indicador no tiene meta, la respuesta lo dice y el tablero muestra
  "Sin meta definida".
- La fecha de referencia sale de los datos (`periodo`, `fecha_corte`), nunca del reloj (TR-D3).

### 3. Contrato de la API (v1)

Todas las rutas van bajo `/api/v1/`, responden JSON con nombres canónicos en español y devuelven los errores
como `{ "codigo": …, "mensaje": … }`, con mensajes para el usuario final.

| Ruta | Contenido |
|---|---|
| `GET /api/v1/estado` | "Datos usados": fuentes, archivos, fecha de corte, periodos disponibles y aviso de periodos mezclados (TR-D16, IND-C2) |
| `GET /api/v1/filtros` | Valores disponibles para cada filtro |
| `GET /api/v1/indicadores` | IND-01…08 del periodo elegido: valor, valor del mes anterior, variación, sentido de "mejor", meta y origen (fuente, fecha de corte, IDs de regla; TR-D17) |
| `GET /api/v1/series` | Serie mensual de cada indicador, para los gráficos |
| `GET /api/v1/checks` | IND-C1 e IND-C3 con su explicación |
| `GET /api/v1/detalle/…` | Detalle nominal (personas en alerta, personas sin base de personal) |

El esquema exacto de cada respuesta se fija en el Plan 5 y se prueba en los tests de la API.

### 4. Privacidad por nivel (TR-D18)

Las rutas de indicadores, series, checks y estado **nunca** devuelven nombres ni números de documento. El
detalle nominal sale solo de `/api/v1/detalle/…`, que usa únicamente la pestaña "Alertas (solo RR. HH.)".

Es una **separación de presentación, no de seguridad**. Quien puede abrir la API en su equipo también puede
abrir los Parquet de la carpeta de salida. La protección real es la del sistema operativo sobre esa carpeta.

## Alternativas consideradas

| Alternativa | Por qué no |
|---|---|
| **DuckDB-WASM en el navegador** | Habría que descargar al navegador los Parquet completos, nombres incluidos, y una librería de unos 30 MB. El SQL de indicadores correría en JavaScript, fuera de los tests de Rust, y TR-D18 no podría separarse ni a nivel de presentación. |
| **Híbrido** (indicadores precalculados más WASM) | Son dos motores y dos lugares donde vive la lógica. |
| **Precalcular indicadores al procesar** | Hay una fila por cada combinación de filtros, que crecen de forma combinatoria, y el *cross-filtering* queda limitado a lo precalculado. Con DuckDB y miles de filas, calcular al consultar tarda milisegundos. |
| **Solo comandos de Tauri, sin HTTP** | No cumple el requisito de abrir el tablero en el navegador (00-objetivo §6). |
| **`tiny_http`** | Es síncrono, obliga a escribir ruteo y JSON a mano y no comparte el runtime tokio que ya trae Tauri v2. |

## Consecuencias

- **Un solo motor y un solo lugar para la lógica.** Indicadores, Silver y Gold son SQL de DuckDB en `config/sql/`
  y se prueban con el mismo arnés en Rust.
- **Dependencias nuevas:** `axum` y `tokio` (más sus transitivas). Tauri v2 ya usa tokio, así que no hay un
  segundo runtime.
- El tablero necesita que el servidor esté corriendo, ya sea con `etl-local tablero` o desde la app. Los
  Parquet sueltos no bastan para verlo.
- El contrato `/api/v1/` pasa a ser una interfaz entre el motor y el frontend del Plan 7. Un cambio
  incompatible requiere una `/api/v2/`.
- Los IDs de regla de `docs/logica-negocio/` aparecen en las respuestas (TR-D17), así que renombrarlos rompe
  la interfaz.
- Quedan fuera de este ADR y tendrán ADR propio: el framework del frontend y la librería de gráficos (Plan 7).

## Referencias

- [indicadores-tablero.md](../logica-negocio/indicadores-tablero.md): IND-01…08, IND-C1…C3, metas.
- [reglas-transversales.md](../logica-negocio/reglas-transversales.md): TR-D3, TR-D16, TR-D17, TR-D18.
- [diseno/README.md](../diseno/README.md): pantallas 6 y 7.
- Hoja de ruta: [Plan 5](../superpowers/plans/2026-11-16-hoja-de-ruta.md).

## Notas

- 2026-11-16 (Plan 5): implementado en `crates/api` y `crates/core/src/tablero.rs`. El esquema de cada respuesta
  está en `crates/core/src/tablero/respuesta.rs`; las fórmulas y las decisiones de implementación, en
  [indicadores-tablero.md §7](../logica-negocio/indicadores-tablero.md#7-implementación-plan-5).
- 2026-11-17 (Plan 7): la API también sirve los archivos del tablero (`app/ui/dist`, sin `app.html`) en la misma dirección, con CSP propia y `X-Content-Type-Options: nosniff`, sin salir de esa carpeta. `etl-local tablero --ui <carpeta>` (por defecto `app/ui/dist` o `ui/` junto al ejecutable).
