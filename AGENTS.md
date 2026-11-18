# AGENTS.md

Contrato de trabajo para agentes de IA (Claude Code, Codex, Cursor, etc.) y personas que
contribuyen en este repositorio. Léelo completo antes de cualquier tarea.

## 1. Qué es este proyecto

Rework **open source** de un ETL de Recursos Humanos escrito en Python (el *legacy*) a
**Rust + Tauri**, con un **dashboard tipo BI en `localhost`** que reemplaza a Power BI.

El motor procesa Excel operativos (planillas, base de datos de personal, licencias, practicantes,
exámenes de retiro, relación de ingresos) en capas **Bronze → Silver → Gold**:

- valida las fuentes contra contratos declarativos;
- expresa las reglas de negocio en **SQL de DuckDB**;
- escribe Parquet.

Lecturas obligatorias según la tarea:

| Si vas a… | Lee primero |
|---|---|
| Entender el objetivo | [docs/00-objetivo.md](docs/00-objetivo.md) |
| Entender el legacy | [docs/01-analisis-legacy.md](docs/01-analisis-legacy.md) |
| Implementar o modificar una regla de negocio | [docs/logica-negocio/](docs/logica-negocio/README.md): el documento del dominio + [reglas-transversales.md](docs/logica-negocio/reglas-transversales.md) |
| Tomar una decisión de arquitectura | [docs/adr/](docs/adr/README.md) |

## 2. Estructura del repositorio

> La estructura marcada como *(planificada)* aún no existe. Actualiza esta sección al crearla.

```text
etl_local/
├── AGENTS.md / CLAUDE.md        # contrato para agentes (este archivo)
├── docs/                        # objetivo, análisis, lógica de negocio, ADRs
├── HR_Data_Integration_Pipeline/  # LEGACY — solo lectura, nunca se publica (ver §3)
├── crates/                      # workspace Rust
│   ├── core/                    #   motor: contratos, lectura Excel, pipelines, DuckDB, Parquet, indicadores
│   ├── api/                     #   API local del tablero (axum, solo lectura; ADR-0004)
│   └── cli/                     #   runner headless del motor (tests, CI, agentes)
├── app/                         # aplicación Tauri v2 (Plan 6)
│   ├── src-tauri/               #   backend Tauri (usa crates/core); fuera de default-members
│   └── ui/                      #   Svelte 5 + Vite + TS: app.html (pantallas 1–5) y tablero.html (6–7)
├── config/                      # contratos, SQL, parámetros y reportes; un reporte nuevo = contrato + SQL + reportes.yaml (molde, config/README.md)
├── demo/                        # Python + uv: datos sintéticos y paridad (ADR-0002)
├── scripts/                     # utilidades de desarrollo (revisión de confidencialidad)
└── .github/workflows/           # CI: pruebas y release del instalador de Windows
```

Principio de capas: **`crates/core` no depende de Tauri ni de la UI**. Toda la lógica debe poder
ejecutarse desde `crates/cli` sin interfaz gráfica.

## 3. Reglas no negociables

### 3.1 Confidencialidad (bloqueante)

- **Nunca** escribas el nombre de la empresa cliente del legacy en código, docs, tests, nombres de
  archivo, commits ni PRs. Usa `<CLIENTE>` en la documentación y nombres genéricos en el código
  (`payroll_consolidated.parquet`, no `Planilla_<CLIENTE>_...`).
- **Nunca** copies datos reales: nombres de personas, DNI/CE, códigos SAP o archivos Excel/Parquet del
  legacy. Para tests usa datos generados en `demo/` o valores obviamente ficticios (`12345678`).
- El **historial git del legacy contiene datos reales**. No hagas fork, `git subtree` ni copias de su
  `.git`. Este repo debe nacer con historial propio.
- `HR_Data_Integration_Pipeline/` está en `.gitignore` y nunca se versiona. Lo mismo aplica a los
  archivos de datos (`*.xlsx`, `*.parquet`, `*.csv`…), salvo los *fixtures* sintéticos en carpetas `fixtures/`.
  No modifiques esas reglas del `.gitignore` sin preguntar.
- Antes de proponer un commit, verifica que el diff no contiene el nombre del cliente ni datos reales.

### 3.2 Legacy de solo lectura

- No modifiques nada dentro de `HR_Data_Integration_Pipeline/`.
- Se permite ejecutarlo **solo** desde `demo/` para generar resultados esperados (paridad), y siempre
  sobre datos sintéticos.

### 3.3 Decisiones aceptadas

- Respeta los ADRs en estado *Aceptado*. Si una tarea requiere contradecir uno, detente y propón un ADR
  nuevo en lugar de desviarte en silencio.

## 4. Lógica de negocio: cómo implementarla

1. **Cada regla tiene un ID** en `docs/logica-negocio/` (`BD-FLG-03`, `NOM-GLD-07`, `TR-34`…).
   Cítalo en el código (comentario breve) y en el nombre del test que la cubre.
2. **No repliques bugs del legacy a ciegas.** Las secciones *Discrepancias* y *Preguntas abiertas* de
   cada dominio requieren una decisión explícita: `preservar` (paridad), `corregir` o `descartar`.
   Si la decisión no está tomada en [reglas-transversales.md §5](docs/logica-negocio/reglas-transversales.md#5-decisiones-pendientes-para-el-rework)
   o en un ADR, **pregunta**.
3. **Configuración sobre código.** Umbrales, columnas, layouts de Excel y reglas de limpieza van en
   `config/`, no hardcodeados en Rust.
4. **SQL para la lógica Gold.** Si una regla se puede expresar en SQL de DuckDB, va en SQL.
5. **Fechas deterministas.** Ninguna regla usa el reloj del sistema directamente; la fecha de referencia
   se recibe como parámetro (ver TR-57 / TR-D3).
6. **Fallar de forma explícita.** Ninguna etapa reporta éxito si falló; nada de `catch` que silencie
   errores (ver TR-51).
7. Si descubres que una regla documentada es incorrecta, **corrige el documento** en el mismo cambio.
8. **Vocabulario canónico.** Columnas, tablas y textos de UI usan los nombres de
   [docs/logica-negocio/vocabulario.md](docs/logica-negocio/vocabulario.md). Los nombres legacy solo aparecen
   en los mapeos de los contratos. En particular, "ingresos" significa dinero; para personas se dice "entradas".

## 5. Convenciones

| Tema | Convención |
|---|---|
| Idioma | **Español en todo**: documentación, identificadores, comentarios, logs, mensajes de UI y mensajes de commit. Puede revisarse más adelante (por ADR) si se busca mayor alcance internacional |
| Identificadores | En español, **ASCII sin tildes ni ñ** (`anio`, `periodo`, `fecha_cese`, `validar_contrato`); Rust admite identificadores no ASCII, pero se evitan para no tener problemas de tooling ni de teclado |
| Palabras clave de Conventional Commits | Se mantienen en inglés (`feat:`, `fix:`…); la descripción va en español (`feat: lectura de cabeceras desplazadas`) |
| Mensajes al usuario final (UI) | Claros para usuarios no técnicos |
| Nombres de columnas de negocio | Se conservan tal como vienen en la fuente; la normalización se declara en el contrato |
| Rust | Edition 2021+, `cargo fmt`, `cargo clippy -D warnings`; errores con `thiserror` (librería) y `anyhow` (binarios) |
| Commits | [Conventional Commits](https://www.conventionalcommits.org/) (`feat:`, `fix:`, `docs:`, `test:`, `refactor:`, `chore:`) |
| ADRs | `docs/adr/NNNN-titulo.md`, indexados en `docs/adr/README.md` |

## 6. Comandos

> Se completarán al crear cada componente. No inventes comandos que no existan todavía: si falta uno,
> agrégalo aquí cuando lo crees.

| Tarea | Comando |
|---|---|
| Formatear Rust | `cargo fmt --all` |
| Lint Rust | `cargo clippy --workspace --all-targets -j 2 -- -D warnings` (incluye la app: requiere `app/ui/dist`, ver abajo) |
| Tests Rust (motor, API, CLI) | `cargo test -j 2` (los `default-members`; no compila Tauri) |
| Validar archivos | `cargo run -p etl-local-cli -- validar --fuente planilla <archivos>` (también `leer`, `--json`) |
| Procesar (Silver/Gold) | `cargo run -p etl-local-cli -- procesar --salida <carpeta> --planilla <archivos> --licencias <archivo> --base-personal <archivo>` |
| Tablero (API + interfaz) | `npm --prefix app/ui run build` y `cargo run -p etl-local-cli -- tablero --salida <carpeta> [--puerto 8787] [--ui app/ui/dist]` → `http://127.0.0.1:8787/` |
| Tablero en desarrollo (recarga en caliente) | `etl-local tablero --salida <carpeta>` (puerto 8787) y `npm --prefix app/ui run dev` → `http://localhost:5173/tablero.html` (Vite reenvía `/api`) |
| Actualizar (lo que hace la app) | `cargo run -p etl-local-cli -- actualizar --salida <carpeta> [--planilla …] [--licencias …] [--base-personal …] [--sin licencias]` |
| UI: dependencias | `npm --prefix app/ui ci` |
| UI: tests y tipos | `npm --prefix app/ui test` · `npm --prefix app/ui run check` |
| Tipos TS derivados de Rust (tras cambiar un tipo de respuesta) | `ACTUALIZAR_ESQUEMA=1 cargo test -p etl-local-core --test esquema_api` · `ACTUALIZAR_ESQUEMA=1 cargo test -p etl-local-app --lib esquema` · `npm --prefix app/ui run tipos` |
| UI: build (necesario antes de compilar o probar la app) | `npm --prefix app/ui run build` |
| UI simulada en el navegador (sin Rust) | `npm --prefix app/ui run dev` → `http://localhost:5173/app.html?simulado` (`&error` para el caso con error) |
| Tests de la app | `cargo test -p etl-local-app --lib -j 2` (después del build de la UI) |
| App en desarrollo | `npm --prefix app/ui run tauri -- dev` (`ETL_LOCAL_SALIDA` y `ETL_LOCAL_CONFIG` cambian las carpetas) |
| Instalador | `npm --prefix app/ui run tauri -- build` (release: solo al final, sin otra compilación en paralelo) |
| Entorno demo | `uv sync --project demo` |
| Generar datos sintéticos | `uv run --project demo generar-demo --semilla 42 --modo limpio` |
| Tests del generador demo | `uv run --project demo pytest demo/tests` |
| Lint del generador demo | `uv run --project demo ruff check demo` |
| Dependencias de paridad (exactas del legacy) | `uv sync --project demo --group paridad` |
| Generar golden de paridad | `uv run --project demo generar-golden --modo limpio` (también `sucio`, `casos`) |
| Revisión de confidencialidad (nombre del cliente y correos, también dentro de los `.xlsx`) | `uv run --project demo python scripts/revisar_confidencialidad.py` (0 = limpio, 1 = hallazgos, 2 = sin legacy) |

### CI y releases

- **Pruebas** ([.github/workflows/pruebas.yml](.github/workflows/pruebas.yml)): en cada push a `main` y en
  cada PR, en Windows. Corre fmt, clippy, `cargo test --workspace`, los tests y el build de la interfaz y
  los tests del demo. Los de paridad se saltan, porque el legacy no se publica.
- **Release** ([.github/workflows/release.yml](.github/workflows/release.yml)): al empujar una etiqueta
  `v*`, compila el instalador con `tauri-apps/tauri-action` y crea un release **borrador** con el `.msi`
  y el `-setup.exe`.
- Las acciones van fijadas por SHA de commit, con la versión en un comentario. Para actualizar una:
  `gh api repos/<dueño>/<acción>/commits/<etiqueta> --jq .sha`.

Pasos para publicar una versión (cada uno con pedido explícito, §7):

1. Subir la versión en `Cargo.toml` (`[workspace.package] version`). La app toma esa versión.
2. Correr el script de confidencialidad en local, con resultado 0. En CI no se puede correr, porque
   necesita el legacy.
3. `git tag vX.Y.Z` y `git push origin vX.Y.Z`. Seguir la ejecución con `gh run watch`.
4. Descargar los instaladores del borrador (`gh release download vX.Y.Z`) y pasarles el script de
   confidencialidad.
5. Publicar el borrador.

Requisitos de desarrollo en Windows: toolchain de Rust (MSVC), Build Tools de Visual Studio (C++),
WebView2, Node.js y uv.

Compilación (DuckDB se compila desde C++ dentro del binario; ver las directrices de
[la hoja de ruta](docs/superpowers/plans/2026-11-16-hoja-de-ruta.md#directrices-de-compilación-para-los-planes-pendientes-69)):

- nunca `cargo clean` ni borrar `target/`; con artefactos dañados, borrar solo los `*.rmeta`/`*.rlib` de 0 bytes;
- trabajar en debug (DuckDB ya va optimizado en ese perfil); `--release` solo para medir o entregar, sin otra
  compilación en paralelo;
- `-j 2` por defecto, `-j 1` si aparece `os error 1455`; `target\` excluido del antivirus.

## 7. Límites de autonomía

**Puedes hacer sin preguntar:**

- leer cualquier archivo, incluido el legacy;
- crear y editar código, tests, config y docs dentro del alcance de la tarea;
- correr formateo, lint, tests y builds locales;
- generar datos sintéticos.

**Pregunta antes de:**

- agregar dependencias nuevas (crates, paquetes npm o Python), especialmente si son pesadas o nativas;
- cambiar el formato de contratos, esquemas o pipelines en `config/`;
- tomar una decisión de paridad o corrección no registrada;
- contradecir un ADR o crear uno nuevo;
- borrar archivos que no creaste en la tarea actual;
- hacer commits, push, abrir PRs, crear tags o publicar releases.

**Nunca:**

- modificar el legacy;
- introducir datos reales o el nombre del cliente;
- desactivar tests o lint para que algo "pase";
- usar `--no-verify` o saltarse hooks.

## 8. Definición de terminado

Una tarea está terminada cuando:

- [ ] el código compila y `fmt`, `clippy` y `test` pasan en lo que tocaste;
- [ ] las reglas implementadas citan su ID y tienen al menos un test (con datos sintéticos);
- [ ] los docs afectados (`docs/logica-negocio/`, ADRs, este archivo) están actualizados;
- [ ] el diff no contiene datos reales ni el nombre del cliente;
- [ ] reportaste honestamente qué se verificó y qué no.
