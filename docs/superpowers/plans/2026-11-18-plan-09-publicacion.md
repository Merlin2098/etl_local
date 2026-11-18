# Plan 9: Publicación open source — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Dejar el proyecto listo para publicarse en `github.com/merlin2098/etl-local`. Eso incluye el README con el caso de negocio, la licencia MIT, CI de pruebas, un release automático del instalador de Windows y una revisión de confidencialidad repetible. Los pasos visibles hacia afuera se hacen solo con permiso explícito.

**Architecture:**
- Higiene del repositorio: `.gitignore`, `.gitattributes`, `LICENSE` y la rama `main`.
- Un script de confidencialidad en Python que recorre exactamente lo que se versionaría, incluido el interior de los `.xlsx`.
- El README.
- Dos workflows de GitHub Actions (`pruebas.yml` y `release.yml` con `tauri-apps/tauri-action`).
- Una verificación en una copia limpia del árbol versionable.
- Al final, los pasos de publicación, uno por uno y con permiso.

**Tech Stack:** Git, GitHub Actions, `tauri-apps/tauri-action`, Python + uv (script y tests), Markdown + Mermaid.

**Spec:** [docs/superpowers/specs/2026-11-18-plan-09-publicacion-design.md](../specs/2026-11-18-plan-09-publicacion-design.md)

## Global Constraints

- Nunca escribir el nombre del cliente en ningún archivo, mensaje de commit ni descripción. Se verifica con `nombres_a_ocultar(ubicar_legacy())` en tiempo de ejecución.
- `HR_Data_Integration_Pipeline/` nunca se versiona. No se enlaza ni menciona el repositorio del legacy en GitHub.
- Sin commit, push, creación de repositorio, etiqueta ni publicación de release sin pedido explícito del usuario **en ese momento** (Tarea 5).
- Licencia MIT: `Copyright (c) 2026 Ricardo Uculmana`.
- Solo Windows. Release como **borrador**.
- Todo en español, con identificadores ASCII. Sin dependencias nuevas de Rust, npm ni Python. Las acciones de GitHub no son dependencias del producto.
- Compilación local: `-j 2`, nunca `cargo clean`, nunca dos `cargo` a la vez. Si este plan necesita un release local, va solo, en segundo plano.

## Review Focus

1. **Un clon nuevo no compila o sus tests fallan** porque algo necesario está ignorado o solo existe en local (fixtures, esquemas generados, `package-lock.json`). Se pinea en la Tarea 4: `copia_limpia_pasa_las_pruebas`.
2. **CI en Windows hace checkout con CRLF** y fallan los tests que comparan archivos generados (esquemas, golden). Se pinea en la Tarea 1 (`.gitattributes` con `eol=lf`) y en la Tarea 4 (la copia limpia se hace con `git checkout-index`, que aplica `.gitattributes`).
3. **El nombre del cliente dentro de un `.xlsx`**, que es un zip y que `grep` no ve. Se pinea en la Tarea 2: `revisa_dentro_de_los_xlsx`.
4. **Un correo personal o de trabajo en archivos o en el historial.** Se pinea en la Tarea 2 (`encuentra_correos`) y en la Tarea 5 (verificación de `git config` antes del commit).
5. **El workflow de release falla en GitHub** porque no encuentra la app o el frontend. Se mitiga en la Tarea 4 corriendo en local exactamente el mismo comando de build que usará la acción. Se confirma en la Tarea 5 al ver la ejecución.

---

### Task 1: Higiene del repositorio

**Files:**
- Create: `LICENSE`, `.gitattributes`
- Modify: `.gitignore`, `Cargo.toml` (`rust-version`)

**Interfaces:**
- Produces: la lista de lo versionable = `git ls-files --cached --others --exclude-standard` (la usan las Tareas 2 y 4).

- [ ] **Step 1: Comprobar el estado (RED)**

  Run: `git status --porcelain | grep -E "\.playwright-mcp" ; test -f LICENSE ; test -f .gitattributes`

  Expected: aparece `.playwright-mcp/` y faltan los dos archivos.

- [ ] **Step 2: Implementar**
  - `.gitignore`: agregar `.playwright-mcp/` en la sección de agentes.
  - `LICENSE`: texto MIT estándar con `Copyright (c) 2026 Ricardo Uculmana`.
  - `.gitattributes`:
    - `* text=auto eol=lf`;
    - `binary` para `*.xlsx *.xlsm *.xls *.parquet *.png *.ico *.icns *.woff *.woff2 *.pdf`.
  - `rust-version`: ponerlo en la versión mínima real. El código usa *let chains*, que piden 1.88; confirmarlo con `rustc --version` y con un `cargo check` que no falle por la versión.
  - Rama: `git branch -m master main` (el repositorio no tiene commits; es local y reversible).

- [ ] **Step 3: Verificar (GREEN)**

  Run: `git status --porcelain --ignored | grep -E "playwright|HR_Data|\.superpowers|target/|node_modules"` y `git branch --show-current`

  Expected: todas esas rutas aparecen como `!!` (ignoradas) y la rama es `main`.

### Task 2: Revisión de confidencialidad

**Files:**
- Create: `scripts/revisar_confidencialidad.py`, `demo/tests/test_revisar_confidencialidad.py`

**Interfaces:**
- Produces:
  - `revisar(raiz: Path, archivos: list[str], nombres: list[str]) -> list[Hallazgo]`;
  - `Hallazgo(archivo: str, motivo: Literal["nombre", "correo"], linea: int | None)`;
  - CLI: `uv run --project demo python scripts/revisar_confidencialidad.py [--raiz .]`, que sale con 0 si no hay hallazgos y con 1 si los hay.
- Reglas:
  - **Nombres:** se obtienen de `paridad_legacy.entorno` y se buscan sin distinguir mayúsculas. **Nunca se imprimen**: el hallazgo dice «nombre» y el archivo.
  - **Correos:** se busca la regex `[\w.+-]+@[A-Za-z][\w-]*(?:\.[\w-]+)*\.[A-Za-z]{2,}\b` (el dominio empieza con letra, para no confundir versiones como `paquete@1.2.3`). Se permiten `*@users.noreply.github.com`, `*@example.com` y `noreply@anthropic.com`.
  - **Formatos:** dentro de `.xlsx`/`.xlsm` se leen las entradas `.xml` con `zipfile`; los demás binarios se leen como bytes decodificados en latin-1.
  - **Archivos:** sin argumento, la lista es `git ls-files --cached --others --exclude-standard`.
  - **Sin legacy:** sale con 2 y el mensaje «No se encontró el legacy: no se puede revisar el nombre del cliente».

- [ ] **Step 1: Escribir los tests.** Usan `tmp_path` y un nombre de prueba ficticio, `"Empresa Ficticia"`; nunca el real.
  - `test_encuentra_nombre_sin_mostrarlo`: un archivo con «empresa ficticia» da un hallazgo `nombre`, y el texto del hallazgo no contiene el nombre.
  - `test_revisa_dentro_de_los_xlsx`: un `.xlsx` creado con openpyxl con el nombre en una celda da un hallazgo.
  - `test_encuentra_correos`: un correo de un dominio cualquiera da un hallazgo y `x@users.noreply.github.com` no.
  - `test_limpio_sin_hallazgos`.
- [ ] **Step 2: Run** `uv run --project demo pytest demo/tests/test_revisar_confidencialidad.py -q`. Expected: FAIL (módulo inexistente).
- [ ] **Step 3: Implementar.** El test importa el script con `importlib` desde su ruta.
- [ ] **Step 4: Run** el test. Expected: PASS.
- [ ] **Step 5: Correr el script sobre el árbol real.** Expected: 0 hallazgos. Si aparece algo, se corrige el archivo (no el script) y se anota en el ledger.
- [ ] **Step 6: Revisión manual** de lo que el script no puede juzgar:
  - `demo/golden/` y `docs/01-analisis-legacy.md`: buscar datos que parezcan reales (personas, montos, RUC) y referencias al repositorio del legacy;
  - cada `docs/**` que diga «github.com».

  Se anota el resultado en el ledger. Si algo no puede publicarse, se pregunta antes de borrarlo.

### Task 3: README

**Files:**
- Create: `README.md`

- [ ] **Step 1: Escribir** con las secciones de la §5.2 del spec, en este orden:
  1. título, línea de qué es, insignias de `pruebas.yml` y licencia, y el enlace de descarga a `https://github.com/merlin2098/etl-local/releases/latest`;
  2. «El problema»;
  3. «La solución»;
  4. «Arquitectura» (Mermaid `flowchart LR`, enlaces a `docs/adr/`);
  5. «Stack»;
  6. «Probarlo» (instalador, aviso de SmartScreen y cómo seguir, datos de ejemplo: `uv run --project demo generar-demo --modo limpio` o `crates/core/tests/fixtures/demo/limpio/`);
  7. «Para desarrolladores» (requisitos, incluido el archivo de paginación de ≥ 16 GB o administrado por el sistema y `target\` fuera del antivirus; comandos principales; enlaces a `AGENTS.md`, `CLAUDE.md`, `config/README.md` y `docs/`);
  8. «Posibles mejoras» (visualizador de datos en lugar del tablero en localhost, web que consuma el tablero, macOS/Linux, firma de código, reportes del molde en el tablero);
  9. «Licencia».

  Las cifras (reportes, indicadores) salen de `config/reportes.yaml` y `config/indicadores.yaml`, no de memoria.
- [ ] **Step 2: Verificar:**
  - que cada enlace relativo exista: un script corto en Python que extraiga `](ruta)` y compruebe `Path.exists()`, con 0 rotos;
  - que el bloque Mermaid tenga sintaxis válida: revisión visual del código, porque no hay renderizador local;
  - que el script de confidencialidad siga en 0.

### Task 4: CI/CD y copia limpia

**Files:**
- Create: `.github/workflows/pruebas.yml`, `.github/workflows/release.yml`
- Modify: `AGENTS.md` (sección de comandos: CI y release), `README.md` (insignia ya apuntando a `pruebas.yml`)

**Interfaces:**
- Consumes: la lista de lo versionable (Tarea 1) y el script (Tarea 2).
- Produces:
  - **`pruebas.yml`**:
    - `on: push (main), pull_request`; `runs-on: windows-latest`; `permissions: contents: read`; `concurrency` por rama con `cancel-in-progress`.
    - Pasos:
      1. checkout;
      2. `dtolnay/rust-toolchain` stable con `clippy` y `rustfmt`;
      3. `Swatinem/rust-cache`;
      4. `actions/setup-node` (versión del `engines` de `app/ui/package.json` o la LTS actual; cache npm con `app/ui/package-lock.json`);
      5. `astral-sh/setup-uv`;
      6. `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` y `cargo test --workspace`;
      7. `npm ci`, `npm test`, `npm run check` y `npm run build` en `app/ui`;
      8. `uv run --project demo pytest demo/tests -q`.
  - **`release.yml`**:
    - `on: push: tags: ['v*']`; `permissions: contents: write`; `windows-latest`.
    - Los mismos pasos de toolchain, `npm ci` en `app/ui` y luego `tauri-apps/tauri-action` con `projectPath: app/ui`, `tagName: ${{ github.ref_name }}`, `releaseName: "Integrador RR. HH. ${{ github.ref_name }}"`, `releaseDraft: true` y un `releaseBody` en español con el aviso de SmartScreen.
  - Cada acción se fija por SHA de commit, con su versión en un comentario. Los SHA se obtienen con `gh api repos/<dueño>/<acción>/commits/<tag> --jq .sha` (solo lectura).

- [ ] **Step 1: Escribir el test de la copia limpia (RED):** `copia_limpia_pasa_las_pruebas`.
  - Un script en el scratchpad copia lo versionable a un directorio temporal con `git add -A --dry-run` → lista, y luego `git checkout-index` sobre un índice temporal (`GIT_INDEX_FILE`), para que se apliquen `.gitattributes` y `eol=lf`.
  - En esa copia, con `CARGO_TARGET_DIR=<target del repo>` (así no recompila DuckDB), corre:
    - `cargo test --workspace -j 2`;
    - `npm ci` + `npm test` + `npm run check` + `npm run build` en `app/ui`;
    - `uv run --project demo pytest demo/tests -q`.
  - Antes de tener todo en su lugar se espera ver si algo falta. Si pasa a la primera, se anota en el ledger que el test confirma el estado sin un RED previo.
- [ ] **Step 2: Escribir `pruebas.yml` y `release.yml`.**
- [ ] **Step 3: Validar los YAML:** que carguen con `python -c "import yaml…"` (PyYAML viene con el entorno del demo; si no está, con `uv run --with pyyaml`, sin agregarlo al proyecto). Además, que cada `uses:` tenga un SHA de 40 caracteres.
- [ ] **Step 4: Correr la copia limpia.** Expected: todo PASS. Cargo, app, vitest y build deben dar los mismos totales que en el árbol de trabajo (226, 22, 79). En pytest, los tests de paridad se saltan, porque la copia no incluye el legacy, igual que en CI.
- [ ] **Step 5: Documentar en `AGENTS.md`:** cómo se publica un release (etiqueta `v*` → borrador → revisar → publicar) y que el script de confidencialidad se corre antes de cada etiqueta.

### Task 5: Publicación (cada paso con permiso explícito)

Cada paso se detiene y **pregunta** antes de ejecutarse. Una respuesta negativa o la falta de respuesta deja el plan en ese punto.

- [ ] **Step 1: Antes del commit:**
  - mostrar `git config user.name` y el dominio del correo (no el correo completo);
  - correr el script de confidencialidad, con resultado 0;
  - mostrar `git status --short | wc -l` y las carpetas de primer nivel incluidas.
- [ ] **Step 2: Con permiso**, el primer commit en `main`. Mensaje: «Primera versión pública: motor, app de escritorio, tablero y documentación». Termina con la línea de atribución que pide el sistema.
- [ ] **Step 3: Con permiso:**
  - `gh repo create merlin2098/etl-local --public --source . --remote origin --description "Integrador de reportes de RR. HH.: de Excel operativos a indicadores, en tu PC"`;
  - `git push -u origin main`.
- [ ] **Step 4: Seguir `pruebas.yml`** con `gh run watch`. Si falla, se diagnostica con `superpowers:systematic-debugging` y la corrección va en un commit nuevo (que también pide permiso).
- [ ] **Step 5: Con permiso:** `git tag v0.1.0` + `git push origin v0.1.0`. Se sigue `release.yml`.
- [ ] **Step 6: Verificar el release borrador:**
  - tiene `.msi` y `setup.exe`;
  - se descargan con `gh release download` a la scratchpad;
  - el script de confidencialidad sobre esos binarios da 0 hallazgos;
  - se avisa al usuario para que lo publique (o lo publico yo si me lo pide).
- [ ] **Step 7: Cierre:**
  - en la hoja de ruta, el Plan 9 pasa a **Completado**;
  - el README del repositorio ya enlaza `releases/latest`.
