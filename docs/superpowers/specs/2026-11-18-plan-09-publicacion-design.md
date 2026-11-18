# Plan 9 — Publicación open source: diseño

> Fecha: 2026-11-18 · Estado: **aprobado** (2026-11-18) · Opción elegida: **A (GitHub Actions con `tauri-apps/tauri-action`)**

## 1. Intención

Publicar el proyecto como open source en la cuenta personal de GitHub `merlin2098`, con licencia MIT. Debe
servir a dos públicos:

- **Quien solo quiere probarlo** descarga un instalador de Windows desde un *release* de GitHub, sin compilar
  nada.
- **Quien quiere entenderlo o contribuir** encuentra el caso de negocio, la arquitectura y las guías para
  desarrolladores, incluidos los archivos de agentes (`AGENTS.md`, `CLAUDE.md`), que se publican a propósito.

## 2. Objetivos, no objetivos y anti-objetivos

**Objetivos**

- `README.md` que estructure el caso de negocio (pedido del usuario):
  1. el problema identificado;
  2. cómo lo resuelve la plataforma;
  3. un diagrama de la arquitectura;
  4. el stack;
  5. posibles mejoras, entre ellas reemplazar el tablero en localhost por un visualizador de datos y crear
     una web donde se consuma el tablero.
- `LICENSE` MIT.
- CI en GitHub Actions:
  - **Pruebas** en cada push a `main` y en cada PR.
  - **Release** al empujar una etiqueta `v*`: compila en Windows y deja el `.msi` y el `setup.exe` en un
    *release* borrador.
- Revisión de confidencialidad antes de publicar, repetible con un script local.
- Repositorio con **historial nuevo**: `git init` en esta carpeta, sin copiar nada del `.git` del legacy.

**No objetivos**

- Instaladores de macOS y Linux. Solo Windows por ahora (pedido del usuario).
- Firma de código del instalador (requiere un certificado pagado).
- Auto-actualización de la app.
- Las mejoras futuras del README: solo se describen.

**Anti-objetivos** (sería un fracaso aunque funcione)

- Que el nombre del cliente o un dato real llegue al repositorio público, en el código, en un archivo
  generado, en el historial o en el instalador.
- Enlazar o referenciar el repositorio del legacy (su historial tiene Excel reales).
- Publicar o empujar sin un pedido explícito del usuario en ese momento.

## 3. Restricciones

- AGENTS.md: sin nombre del cliente, sin datos reales; `HR_Data_Integration_Pipeline/` nunca se versiona; sin
  commit, push, tag ni release sin pedido explícito.
- Las directrices de compilación de la hoja de ruta. El instalador local, si se compila, va solo al final y en
  segundo plano.
- Las acciones de GitHub se fijan por versión (mejor por SHA) y el workflow usa los permisos mínimos.

## 4. Enfoques considerados

| | Enfoque | Costo | Elegido |
|---|---|---|---|
| A | `tauri-apps/tauri-action` en un workflow por etiqueta, más un workflow de pruebas | Depende de una acción de terceros (la oficial de Tauri) | **Sí** |
| B | Workflow escrito a mano (`npm run tauri build` + `gh release upload`) | Más YAML propio que mantener, sin ganancia | No |
| C | Compilar en local y subir a mano | No deja el binario en el release de forma automática | No |

## 5. Diseño

### 5.1 Repositorio

- Nombre propuesto: **`etl-local`** (como los crates `etl-local-*`), público, en `github.com/merlin2098`.
  Es distinto del repositorio del legacy.
- Descripción corta: «Integrador de reportes de RR. HH.: de Excel operativos a indicadores, en tu PC».
- El usuario ya hizo `git init` y configuró `user.name`/`user.email` (correo personal). La rama se renombra a
  `main` antes del primer commit.
- `.gitignore`: se agrega `.playwright-mcp/` (capturas y logs de pruebas manuales). Se publican
  `AGENTS.md`, `CLAUDE.md` y `.vscode/` (extensions y settings), que ya están permitidos.
- Archivos que se revisan uno por uno antes del primer commit:
  - `demo/golden/` (salidas del legacy sobre datos sintéticos);
  - `docs/01-analisis-legacy.md`;
  - los documentos que citan rutas del legacy.

  Las citas `archivo:línea` del legacy se conservan: no contienen datos y son la trazabilidad de las reglas.

### 5.2 README

En español, como todo el proyecto. Secciones:

1. Título, una línea de qué es, insignias de CI y licencia, y el enlace «Descargar para Windows» al último
   release.
2. **El problema:** el proceso manual de RR. HH. con varios Excel por mes, sin trazabilidad ni alertas, y
   herramientas que obligan a mover los datos fuera del equipo.
3. **La solución:** la app de escritorio y su flujo. Primero se elige el reporte; luego la app revisa los
   archivos y avisa en términos de Excel; después procesa y abre el tablero. Todo sucede en el equipo.
4. **Arquitectura:** un diagrama Mermaid con Excel → contratos y validación (preflight) → DuckDB (capas Silver
   y Gold) → Parquet y Excel de revisión → API local → tablero, más la app Tauri que orquesta. Enlaza los ADR.
5. **Stack:** Rust (núcleo, CLI, API axum), DuckDB, Tauri 2, Svelte 5 + ECharts, y Python + uv para los datos
   de ejemplo.
6. **Probarlo:** descargar el instalador, el aviso de Windows SmartScreen y cómo seguir, y los Excel de
   ejemplo con el generador o los fixtures.
7. **Para desarrolladores:** requisitos (Rust, Node, uv, y el archivo de paginación de ≥ 16 GB o administrado
   por el sistema), los comandos principales y enlaces a `AGENTS.md`, `config/README.md` y `docs/`.
8. **Posibles mejoras:**
   - un visualizador de datos en vez del tablero en localhost;
   - una web donde se consuma el tablero;
   - instaladores para macOS y Linux;
   - firma de código;
   - los reportes del molde en el tablero.
9. Licencia MIT.

### 5.3 CI/CD (`.github/workflows/`)

- **`pruebas.yml`** corre en `windows-latest`, en cada push a `main` y en cada PR:
  - cargo fmt `--check`, clippy `-D warnings` y `cargo test` del workspace, con la caché de `Swatinem/rust-cache`;
  - `npm ci`, vitest, svelte-check y build en `app/ui`;
  - `uv run pytest` en `demo/` (los tests de paridad se saltan solos sin el legacy).
- **`release.yml`** corre al empujar una etiqueta `v*`:
  - `tauri-apps/tauri-action` compila en `windows-latest` y crea un *release* **borrador** con el `.msi` y
    el `setup.exe`;
  - el usuario lo revisa y lo publica.
- Permisos mínimos: `contents: read` en pruebas y `contents: write` solo en release.

### 5.4 Confidencialidad

- `scripts/revisar-confidencialidad.sh`:
  - obtiene los nombres a ocultar con el mismo comando de siempre (`paridad_legacy.entorno`), sin escribirlos
    en ningún archivo;
  - revisa todos los archivos que se versionarían (`git ls-files` si ya hay repo; si no, el árbol menos lo
    ignorado);
  - busca además patrones de correo electrónico;
  - sale con error si encuentra algo.
- Se corre antes del primer commit y antes de cada etiqueta. En CI no se puede correr, porque el legacy no se
  publica; el README lo dice.
- El instalador compilado también se revisa (como en el Plan 7).

### 5.5 Pasos que esperan un pedido explícito del usuario

1. `git init`, el primer commit y la rama `main`.
2. Crear el repositorio `merlin2098/etl-local` con `gh` y hacer push.
3. Empujar la etiqueta `v0.1.0`, que dispara el release.
4. Publicar el release borrador.

El plan deja todo listo hasta el paso 1 y se detiene a preguntar antes de cada uno.

## 6. Qué queda para quien construya

- La redacción del README dentro de las secciones fijadas.
- Las versiones exactas de las acciones de GitHub (fijadas).
- El título del diagrama y su nivel de detalle (uno solo, legible en GitHub).

## 7. Decisiones que tomé yo (revisables)

1. Nombre del repositorio: `etl-local`.
2. Titular del copyright en `LICENSE`: **Ricardo Uculmana** (decisión del usuario, 2026-11-18).
3. Las pruebas de CI corren en Windows, igual que el release; Linux sería más rápido pero la app Tauri necesita
   otras dependencias del sistema.
4. El release sale como **borrador** y lo publicas tú.
5. README en español.
6. Se publican `AGENTS.md`, `CLAUDE.md` y `.vscode/`; no se publican `.playwright-mcp/` ni `.superpowers/`.
