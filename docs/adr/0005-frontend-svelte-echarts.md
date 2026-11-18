# ADR-0005: Frontend con Svelte 5 y gráficos con Apache ECharts

- **Estado:** Aceptado
- **Fecha:** 2026-11-16
- **Relacionado:** [ADR-0001](0001-stack-rust-tauri.md), [ADR-0004](0004-api-local-tablero.md),
  [diseno/](../diseno/README.md), [indicadores-tablero.md](../logica-negocio/indicadores-tablero.md),
  [reglas-transversales.md](../logica-negocio/reglas-transversales.md) (TR-D16…TR-D19)

## Contexto

ADR-0001 eligió Tauri v2 con un frontend web. ADR-0001 y ADR-0004 dejaron abiertos el framework del frontend y
la librería de gráficos. Lo que ya está decidido:

- **Dos contextos, una misma interfaz visual** ([diseno/](../diseno/README.md)):
  - pantallas 1–5 (elegir archivos, revisar, procesar, listo), solo dentro de la app de escritorio, con acceso
    a archivos;
  - pantallas 6–7 (tablero y "Cómo leer este tablero"), en la app **y** en el navegador, en
    `http://127.0.0.1:<puerto>`.
- **Los datos del tablero vienen de la API local** (ADR-0004), que calcula todo en el servidor. El frontend no
  agrega datos: muestra, filtra y explica.
- **Requisitos del tablero:**
  - *cross-filtering* entre gráficos, tooltips y tablas exportables a CSV;
  - botón "¿De dónde sale?" (TR-D17);
  - barra "Datos usados" (TR-D16);
  - resumen ejecutivo en PDF de una página (TR-D19).
- **Lenguaje visual aprobado:**
  - tipografías Atkinson Hyperlegible, Bricolage Grotesque y JetBrains Mono;
  - paleta validada para daltonismo;
  - controles de 44 px como mínimo;
  - modo oscuro pendiente.
- **Funciona sin internet:** nada se carga desde un CDN.
- **Proyecto open source:** el código debe ser fácil de leer para quien contribuya, y los paquetes, livianos.

## Decisión

### 1. Svelte 5 con Vite y TypeScript, como SPA estática

- **Svelte 5** (con *runes*), **Vite** y **TypeScript** en modo estricto. Sin SvelteKit: no hace falta servidor,
  SSR ni enrutador de archivos.
- El resultado son archivos estáticos (`dist/`). Tauri los empaqueta en la app y la API axum los sirve para el
  navegador. En ejecución no hay servidor Node.
- **Calidad:** `svelte-check` (tipos) y **Vitest** (lógica: cliente de la API, estado de filtros, formatos).
- **Gestor de paquetes:** npm (Node ya figura en los requisitos de AGENTS.md).
- **Tipografías:** las tres son de licencia OFL y van empaquetadas como `woff2`.

### 2. Apache ECharts para los gráficos

- **ECharts 6**, importado por partes (`echarts/core` más los tipos de gráfico y componentes que se usen), con
  el **renderizador SVG**:
  - texto nítido y accesible;
  - impresión vectorial para el PDF;
  - con los pocos puntos del tablero, sin costo de rendimiento.
- **Un tema propio** generado desde los *tokens* de `docs/diseno/`, en versión clara y oscura. Los colores
  salen de la paleta validada; el aviso y el error nunca se distinguen solo por color.
- **Accesibilidad:**
  - se activa la opción `aria` de ECharts, que da una descripción textual del gráfico;
  - cada gráfico tiene al lado su tabla de datos (exportable a CSV), que es además la vía para lectores de
    pantalla.
- **Cross-filtering:**
  1. un clic o una selección en un gráfico actualiza el **estado global de filtros**, un *store* de Svelte;
  2. el estado vuelve a pedir los datos a la API (`/api/v1/indicadores`, `/series`…).
  El filtrado ocurre en el servidor (ADR-0004 §2), nunca en el navegador.

### 3. Un proyecto con dos entradas: `app/ui/`

```text
app/ui/
├── package.json
├── app.html          # pantallas 1–5: solo dentro de Tauri (usa @tauri-apps/api)
├── tablero.html      # pantallas 6–7: Tauri y navegador (usa fetch a /api/v1/)
└── src/
    ├── app/          # pantallas del flujo de actualización
    ├── tablero/      # tablero, "Cómo leer este tablero", vista Resumen
    └── lib/          # componentes, estilos y tokens, tema de ECharts, cliente tipado de la API
```

- Vite construye las dos páginas en un solo `dist/`, y las dos comparten `src/lib/`.
- **En la app de escritorio**, el tablero se abre en una ventana de Tauri que apunta a la API local. Así hay un
  solo camino de código, el mismo que en el navegador.
- **Tipos de la API:** los tipos TypeScript de las respuestas se derivan de los tipos Rust
  (`crates/core/src/tablero/respuesta.rs`) y no se escriben a mano. Un test detecta si quedan desfasados. El
  mecanismo exacto se elige en el Plan 7; la opción natural es un esquema JSON con `schemars`, que ya está en el
  workspace.

### 4. Resumen ejecutivo en PDF con la impresión del sistema (TR-D19)

- La vista "Resumen" de `tablero.html` tiene CSS de impresión: A4, una página, sin controles.
- El botón "Guardar resumen en PDF" abre el diálogo de impresión del sistema (`window.print()`, también dentro
  de la ventana de Tauri), donde el usuario elige "Guardar como PDF".
- Los gráficos SVG se imprimen como vectores.
- No hay generación de PDF en el servidor ni gráficos duplicados.

### Versiones de referencia (npm, 2026-11-16)

| Paquete | Versión |
|---|---|
| `svelte` | 5.57.3 |
| `vite` | 8.3.8 |
| `@sveltejs/vite-plugin-svelte` | 7.3.1 |
| `typescript` | 7.0.2 |
| `echarts` | 6.1.0 |
| `@tauri-apps/api` / `@tauri-apps/cli` | 2.12.5 |
| `vitest` (dev) | 5.1.0 |
| `svelte-check` (dev) | 4.7.8 |

Al crear `app/ui/` se fijan versiones exactas en `package-lock.json`.

## Alternativas consideradas

| Alternativa | Por qué no |
|---|---|
| **React** | Es el ecosistema más grande, pero pide más código de soporte (estado, efectos, memoización) y genera paquetes más pesados para una UI con pocas pantallas. |
| **Vue 3** | Es un punto medio razonable, con menos presencia que React en proyectos de datos y sin ventajas claras sobre Svelte aquí. |
| **Sin framework** (Web Components) | Sin dependencias, pero el estado de filtros, pantallas y formularios habría que escribirlo y mantenerlo a mano. |
| **SvelteKit** | Su enrutador y su SSR no aportan nada a archivos estáticos servidos por Tauri y axum. |
| **Observable Plot** | Da gráficos muy legibles, pero el *cross-filtering* y la interacción se programan a mano. |
| **Vega-Lite** | Trae las selecciones incorporadas, pero es más pesada y menos flexible para el diseño aprobado. |
| **Chart.js** | Ofrece poca interacción entre gráficos y renderiza en canvas, peor para accesibilidad e impresión. |
| **Una sola SPA para todo** | Mezcla el contexto con acceso a archivos (Tauri) con el de solo lectura (navegador) y obliga a ocultar pantallas. |
| **Dos proyectos separados** | Duplica configuración, dependencias y estilos. |
| **PDF generado en Rust** | Obliga a rehacer los gráficos en el servidor y suma dependencias. Un PDF idéntico siempre no compensa ese costo para un resumen de una página. |

## Consecuencias

- Hay **dos ecosistemas en el repo**, Rust y TypeScript, previstos en ADR-0001. Los comandos de `app/ui/` se
  agregan a AGENTS.md §6 cuando se creen.
- **Dependencias nuevas** (frontend): `svelte`, `vite`, `@sveltejs/vite-plugin-svelte`, `typescript`,
  `echarts` y `@tauri-apps/api`. De desarrollo: `vitest` y `svelte-check`.
- **Servir el frontend desde axum** requiere entregar `dist/` como archivos estáticos (Plan 7). Puede ser desde
  disco o embebido en el binario; se decide en ese plan sin cambiar este ADR.
- La **Content Security Policy** de Tauri debe permitir conexiones a `127.0.0.1:<puerto>` y nada externo.
- El **contrato `/api/v1/`** (ADR-0004) queda atado a los tipos TypeScript derivados. Un cambio en
  `respuesta.rs` que no se refleje en el frontend hace fallar un test.
- **El PDF depende del diálogo de impresión** de cada sistema operativo. Lo que se garantiza es el diseño de la
  página (CSS de impresión), no un archivo byte a byte idéntico.

## Notas

- **Plan 6 (2026-11-16):**
  - TypeScript quedó en **6.0.3** y no en 7.0.2: `svelte-check` 4.7.8, la última versión, solo acepta TypeScript 5 o 6. Se sube cuando lo soporte.
  - Los tipos de los comandos de la app de escritorio (`app/ui/src/app/tipos.ts`) están escritos a mano. Un test compara sus claves con ejemplos JSON que genera Rust (`app/src-tauri/tests/ejemplos/`). El mecanismo derivado del Plan 7 los reemplaza.
  - Ninguna de estas notas cambia la decisión.
- **Plan 7 (2026-11-17):** los tipos TypeScript se derivan de Rust. `schemars` (feature `chrono04`) escribe
  `app/ui/src/lib/api/esquema.json` (API) y `app/ui/src/app/esquema.json` (app de escritorio) con
  `ACTUALIZAR_ESQUEMA=1 cargo test …`, y `npm run tipos` (`json-schema-to-typescript` 16.0.0) los convierte en
  `tipos.gen.ts`. Un test de Rust y otro de Vitest fallan si quedan desfasados. Reemplaza los ejemplos JSON y
  los tipos escritos a mano del Plan 6.

## Referencias

- [ADR-0001](0001-stack-rust-tauri.md): stack Rust + Tauri.
- [ADR-0004](0004-api-local-tablero.md): API local del tablero.
- [diseno/README.md](../diseno/README.md): pantallas, principios y lenguaje visual.
- [reglas-transversales.md](../logica-negocio/reglas-transversales.md): TR-D16 (frescura), TR-D17 (trazabilidad),
  TR-D18 (privacidad por nivel) y TR-D19 (resumen en PDF).
