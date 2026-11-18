# Plan 6: App Tauri, flujo de actualización (pantallas 1–5) — Plan de implementación

> **Para agentes:** SUB-SKILL REQUERIDA: usar superpowers:subagent-driven-development (recomendado) o
> superpowers:executing-plans para implementar este plan tarea por tarea. Los pasos usan casillas (`- [ ]`).
> Este repo **no usa git**: donde la skill dice «commit», se registra la tarea en el ledger
> `.superpowers/sdd/2026-11-16-plan-06-app-tauri/progress.md`.

**Objetivo:** una persona de RR. HH. abre la app de escritorio, elige un reporte, arrastra sus Excel, ve los
errores explicados por celda (con un recorte de la hoja), procesa con avance visible y termina con cifras clave,
avisos y su reporte en Excel. Sin escribir un comando.

**Arquitectura:**
- **Núcleo (`crates/core`).** Gana lo que la UI necesita y debe poder correrse sin interfaz (AGENTS.md §2):
  - revisión por archivo con recorte de hoja;
  - avance y cancelación del proceso;
  - un catálogo de reportes;
  - un almacén de entradas que permite actualizar un reporte a la vez;
  - Excel de revisión en `actual/excel/`.
- **App (`app/src-tauri`).** Crate Tauri v2 delgado: ajustes, comandos y estado. Toda la lógica que se prueba
  vive en `servicio.rs`, sin Tauri.
- **UI (`app/ui`).** Svelte 5 + Vite + TypeScript estricto (ADR-0005), con la entrada `app.html`. El estado del
  flujo es una clase con *runes* que recibe un `Backend` inyectado: el real usa `invoke`, el simulado sirve
  para los tests y la revisión visual.

**Stack:**
- Rust: Tauri 2, `tauri-plugin-dialog` 2, `tauri-plugin-opener` 2 y `rust_xlsxwriter` 0.102 (ya en el workspace).
- Frontend: Svelte 5.57, Vite 8.3, TypeScript 7.0, Vitest 5.1, svelte-check 4.7, `@tauri-apps/api` y `cli` 2.12.5.
- Tipografías: `@fontsource/*`, licencia OFL.

**Spec:**
- [docs/diseno/README.md](../../diseno/README.md): pantallas 1–5, principios y lenguaje visual. Los mocks v2
  están en un lienzo privado; los textos que usa este plan están copiados en la sección «Textos».
- [ADR-0005](../../adr/0005-frontend-svelte-echarts.md): frontend.
- [ADR-0001](../../adr/0001-stack-rust-tauri.md): stack.
- [reglas-transversales.md](../../logica-negocio/reglas-transversales.md): TR-D6, TR-D7, TR-D13 y TR-D16.
- [validacion-preflight.md](../../logica-negocio/validacion-preflight.md): PRE-07 y PRE-10.
- [00-objetivo.md](../../00-objetivo.md): «Excel opcional para revisión».

**Decisiones del usuario (2026-11-16):**
1. **Actualización parcial copiando entradas.** Cada archivo aceptado se copia a `<salida>/entradas/<fuente>/`
   (la planilla se reemplaza por periodo) y cada ejecución reprocesa todo desde ahí.
2. **Excel de revisión en este plan.** Un `.xlsx` por tabla Gold en `actual/excel/`; `rust_xlsxwriter` pasa a
   ser dependencia normal de `crates/core`.
3. **Solo los reportes disponibles.** Las tarjetas salen de `config/reportes.yaml`, que referencia contratos
   de `config/fuentes/`. El Plan 8 agrega las suyas sin tocar la UI.

## Restricciones globales

- **Confidencialidad:**
  - nunca escribir el nombre del cliente (usar `<CLIENTE>`) ni copiar datos reales;
  - `HR_Data_Integration_Pipeline/` es solo lectura (AGENTS.md §3.1);
  - las pruebas usan solo los fixtures sintéticos de `crates/core/tests/fixtures/demo/`.
- **Idioma:** español en UI, docs, mensajes y nombres; identificadores ASCII (sin ñ ni tildes), tipos en
  `PascalCase` y funciones en `snake_case`/`camelCase`.
- **UI sin jerga** (diseno §Principios 3):
  - la interfaz nunca muestra Bronze, Silver, Gold, ETL, Parquet ni preflight;
  - los códigos de regla (`PRE-10`) y los nombres de etapa solo aparecen en «Ver detalles para soporte».
- **Un botón desactivado siempre dice por qué** (diseno §Principios 2), con `aria-describedby`.
- **Lenguaje visual exacto de diseno §Lenguaje visual:**
  - colores: fondo `#F5F6F2`, superficie `#FFFFFF`, texto `#1B2420`, secundario `#55605A`, acción `#1E6B47`;
  - aviso: fondo `#FDF1DC`, texto `#6B3A00`; error: `#A3261B` sobre `#FBE7E4`;
  - controles de 44 px como mínimo; botón principal de 52 px;
  - tipografías: Atkinson Hyperlegible (texto, 17 px), Bricolage Grotesque 700 (títulos) y JetBrains Mono
    (celdas).
- **Modo oscuro:** fuera de alcance (pendiente de diseño). Los tokens van en `:root` para poder agregarlo después.
- **Sin internet en ejecución:** nada desde un CDN; las tipografías van empaquetadas.
- **Núcleo sin reloj** (AGENTS.md §4.5): `crates/core` recibe `marca` y `hoy`. La app y la UI sí pueden leer el
  reloj.
- **Dependencias nuevas:** solo las de «Stack». Al aprobar este plan quedan aprobadas. Cualquier otra se
  consulta antes.
- **Contrato de datos (aditivo):**
  - `Problema` gana `celda_encontrada: Option<String>`, que no se serializa si es `None`;
  - `Ejecucion.salidas` incluye las rutas `excel/*.xlsx`;
  - `ErrorProceso` gana `Cancelado`.
  No cambia nada existente. Se anota en la Tarea 11.
- **Directrices de compilación** (hoja de ruta §Directrices):
  1. nunca `cargo clean` ni borrar `target/`; con artefactos dañados, borrar solo los `*.rmeta`/`*.rlib` de
     0 bytes en `target/debug/deps` y reintentar;
  2. todo en debug;
  3. el release (`tauri build`) solo en la Tarea 11, en segundo plano y sin otra compilación en paralelo;
  4. `-j 2` (o `-j 1` ante `os error 1455`), nunca dos `cargo` a la vez;
  5. `tauri dev` usa el `target/` del workspace;
  6. `npm ci` con `package-lock.json` versionado; `node_modules/` y `dist/` no se versionan.
- **Comandos de verificación:**
  - Rust: `cargo test -p <crate> --test <archivo> -j 2`;
  - suite Rust por defecto: `cargo test -j 2`, que con `default-members` excluye la app;
  - app: `npm --prefix app/ui run build` y luego `cargo test -p etl-local-app --lib -j 2`;
  - UI: `npm --prefix app/ui test` (Vitest) y `npm --prefix app/ui run check` (svelte-check);
  - lint: `cargo clippy --workspace --all-targets -j 2 -- -D warnings` (requiere `app/ui/dist`) y
    `cargo fmt --all --check`.

## Textos (de los mocks v2; exactos salvo los marcados ✎)

| Pantalla | Texto |
|---|---|
| Todas | Marca «Integrador RR. HH.» (provisional). Pasos: «Elegir archivos», «Revisar», «Procesar», «Listo». «Volver al inicio» |
| 1 Inicio | Saludo («Buenos días», «Buenas tardes» o «Buenas noches» ✎), «¿Qué quieres actualizar hoy?», «Elige el reporte. Te pediremos los archivos de Excel y nosotros hacemos el resto.» Estado de la tarjeta: «Último mes: octubre 2025», «Al día · 12 nov 2025», «Aún no se ha cargado». Enlace: «Actualizar» o «Cargar por primera vez». Pie: «Los reportes se guardan en **{carpeta}**. Puedes abrirlos en Excel cuando quieras.» y «Abrir carpeta» |
| 2 Elegir | «{Título}: elige los archivos de Excel». Planilla: «Puedes elegir uno o varios meses a la vez. Reconocemos el mes por el nombre del archivo.» Base ✎: «Elige un archivo. Reconocemos la fecha de corte por el nombre del archivo.» «Arrastra aquí tus archivos», «o», «Buscar en mi computadora», «Archivos de Excel (.xlsx, .xlsm, .xls)». «{n} archivos elegidos». Chip del mes («Septiembre 2025») o de la fecha («Corte: 12 nov 2025» ✎); sin fecha: «Mes no reconocido» ✎. «Quitar {archivo}» (aria). Complemento: «Agregar también el control de licencias», «Usaremos **{archivo}** (actualizado el {fecha}).», «Cambiar archivo». «Cancelar» y «Revisar archivos» |
| 3 Revisión | Con errores: «Hay que arreglar {n} archivo(s) antes de seguir». Sin errores ✎: «Todo listo para procesar». «Revisamos que cada Excel tenga la forma esperada. Así evitamos reportes con datos incompletos.» Fila OK: «Todo en orden · {n} personas». Fila con aviso: «Listo · {n} personas». Chip: «Hoja «{hoja}» · celda {celda}». Leyenda: «Aquí deberían ir los títulos» / «Aquí están ahora». «Cómo arreglarlo» con 3 pasos, «Abrir en Excel», «Revisar de nuevo». Pie: «Atrás», «Seguir sin {mes(es)}», «Procesar los {n} meses» (base ✎: «Procesar la base de personal»), «Se activa cuando todos los archivos estén en orden.» |
| 4 Procesando | «Estamos preparando tu {reporte en minúscula}», «Suele tardar menos de un minuto. Puedes seguir usando la computadora; te avisaremos al terminar.» «{i} de {n} tareas», «Quedan unos {s} segundos», «En curso…», «Ver detalles para soporte», «Detener» |
| 5 Listo | «Tu {reporte} está lista/o» ✎ (concordancia en `reportes.yaml`), «{periodos} · actualizada hoy a las {hh:mm}», «**Para revisar:** {mensaje}», «¿Qué quieres hacer ahora?», «Abrir en Excel», «Abrir carpeta», «Guardamos una copia de la versión anterior por si necesitas compararla.» |

**Fuera de alcance de este plan:**
- «Ver el tablero», «Ver en el tablero» y «Ver quiénes son»: se ocultan hasta el Plan 7.
- «¿Necesitas ayuda?»: no tiene contenido todavía.
- La pantalla de ajustes: es un pendiente de diseño; la carpeta de salida tiene un valor por defecto y una
  variable de entorno.

## Foco de revisión

1. **El Excel de salida está abierto en Excel y se vuelve a procesar.** Windows no deja reemplazar `actual/`. Se
   espera el mensaje de `ErrorProceso::Publicar` (el que dice «¿Hay un archivo abierto…?»), con
   `entradas/` sin cambios. Test en la Tarea 4.
2. **Se elige otra vez un archivo que ya está en `<salida>/entradas/`** (copiarlo sobre sí mismo), o la carpeta
   de salida tiene tildes y ñ («Reportes RR. HH./año»). No se trunca el archivo y todo funciona. Test en la
   Tarea 4.
3. **Llega un mes que ya estaba guardado.** El nuevo reemplaza al guardado. Si son **dos nuevos del mismo mes**,
   la UI bloquea «Revisar archivos» con su motivo. Tests en las Tareas 4 y 7.
4. **«Detener» a mitad del proceso.** No cambia `actual/` ni `entradas/` y no quedan carpetas `.en-curso-*`.
   Test en la Tarea 3.
5. **Se suelta un PDF o una carpeta, o el nombre no trae el mes.** El archivo que no es Excel se ignora con un
   mensaje; el que no trae mes muestra «Mes no reconocido» y no se puede procesar. Tests en las Tareas 1 y 7.

---

### Tarea 1: Revisión de archivos en el núcleo (celda encontrada, fecha del nombre, personas y recorte)

**Archivos:**
- Modificar: `crates/core/src/informe.rs` (campo nuevo y constructor), `crates/core/src/preflight.rs` (PRE-10
  llena `celda_encontrada`; `fecha_en_nombre` pública), `crates/core/src/proceso.rs` (los problemas de
  `controles` llevan `celda_encontrada: None`), `crates/core/src/libro.rs` (lectura de rango) y
  `crates/core/src/lib.rs`.
- Crear: `crates/core/src/revision.rs`.
- Tests: `crates/core/tests/revision.rs` (nuevo) y `crates/core/tests/preflight.rs`.

**Interfaces:**
- Produce:
  - `Problema.celda_encontrada: Option<String>` con `#[serde(default, skip_serializing_if = "Option::is_none")]`;
  - `pub fn preflight::fecha_en_nombre(contrato: &Contrato, nombre_archivo: &str) -> Option<FechaNombre>`
    (solo mira el nombre, sin abrir el archivo);
  - `pub struct revision::Revision { pub archivo: String, pub fecha_nombre: Option<FechaNombre>, pub personas: Option<u64>, pub informe: Informe }`
    con `Serialize` y `Deserialize`;
  - `pub fn revision::revisar_archivo(contrato: &Contrato, ruta: &Path) -> Revision`;
  - `pub struct revision::Recorte { pub hoja: String, pub fila_inicio: u32, pub columna_inicio: u32, pub celdas: Vec<Vec<String>> }`
    (Serialize/Deserialize; `celdas[f][c]` es el texto de la fila `fila_inicio + f`);
  - `pub fn revision::recorte(ruta: &Path, hoja: &str, desde: Celda, hasta: Celda) -> Result<Recorte, String>`:
    rango inclusivo, a lo sumo 12 filas × 8 columnas (si se pide más, se recorta desde `desde`), textos
    recortados a 40 caracteres con «…» y celdas vacías como `""`.

- [ ] **Paso 1: escribir los tests que fallan.**
  - En `tests/preflight.rs`, `caso_01_informa_celda_encontrada`: `validar_archivo` sobre
    `fixtures/demo/casos/CASO-01/Planilla 2025-11 Empleados.xlsx` da un error `PRE-10` con
    `celda == Some("A6")` y `celda_encontrada == Some("A7")`.
  - En `tests/revision.rs`:
    - `fecha_del_nombre`: `fecha_en_nombre(planilla, "Planilla 2025-09 Empleados.xlsx") == Some(Periodo("2025-09"))`;
      `fecha_en_nombre(base_personal, "BD. 12.11.2025..xlsx") == Some(FechaCorte(2025-11-12))`;
      `fecha_en_nombre(planilla, "planilla sin mes.xlsx") == None`.
    - `revisar_planilla_limpia`: sobre `planilla("limpio", "2025-09")` da `informe.tiene_errores() == false`,
      `fecha_nombre == Some(Periodo("2025-09"))` y
      `personas == Some(manifiesto("limpio")["periodos"]["2025-09"]["personas_planilla"])` (1226).
    - `revisar_con_error_no_cuenta_personas`: sobre CASO-01 da `personas == None`, `informe.tiene_errores()` y
      `fecha_nombre == Some(Periodo("2025-11"))`.
    - `recorte_de_cabecera_desplazada`: `recorte(CASO-01, "Planilla", A5, D8)` da `fila_inicio == 5`,
      `columna_inicio == 1`, 4×4 celdas, `celdas[1][0] == ""` (A6) y `celdas[2][0]` no vacía (A7).
    - `recorte_limita_el_tamano`: `recorte(…, A1, Z99)` da 12 filas × 8 columnas.
    - `recorte_hoja_inexistente`: devuelve `Err`.
    - `problema_sin_encontrada_no_serializa_el_campo`: `serde_json::to_string` de un `Problema` con `None` no
      contiene `"celda_encontrada"`.
- [ ] **Paso 2:** `cargo test -p etl-local-core --test revision --test preflight -j 2`.
  Esperado: no compila (`revision`, `celda_encontrada` y `fecha_en_nombre` no existen).
- [ ] **Paso 3: implementar.**
  - `fecha_en_nombre` reutiliza la lógica de `revisar_nombre` sin `Registro`: extraer una función común que
    devuelva `Result<Option<FechaNombre>, String>` y que `revisar_nombre` siga usando para sus PRE-07.
  - `Registro` gana un constructor para PRE-10 desplazada que llena `celda_encontrada`.
  - `personas` cuenta los valores distintos no vacíos de la columna `clave()` de la primera hoja del contrato
    leída por `leer_archivo`. Sin clave o con errores, `None`.
  - `recorte` abre con `Libro::abrir`, `cargar` y `texto`.
- [ ] **Paso 4:** el mismo comando del Paso 2 y luego `cargo test -p etl-local-core -j 2`.
  Esperado: todo pasa, incluidas las suites existentes (que solo cambian para construir `Problema` con el campo
  nuevo).
- [ ] **Paso 5:** registrar la tarea en el ledger.

### Tarea 2: Excel de revisión en `actual/excel/`

**Archivos:**
- Modificar: `Cargo.toml` del workspace (nada que agregar: `rust_xlsxwriter` ya está en
  `[workspace.dependencies]`), `crates/core/Cargo.toml` (`rust_xlsxwriter` pasa a `[dependencies]` y se
  conserva en dev), `crates/core/src/almacen.rs` y `crates/core/src/proceso.rs`.
- Test: `crates/core/tests/proceso.rs`.

**Interfaces:**
- Produce:
  - `Almacen::exportar_excel(&self, tabla: &str, ruta: &Path) -> Result<u64, ErrorProceso>`:
    - una hoja con el nombre de la tabla, sin el sufijo `_gold`;
    - fila 1 con los nombres de columna;
    - números como número, fechas como fecha de Excel con formato `dd/mm/yyyy`, booleanos como booleano, texto
      como texto y nulos como celda vacía;
    - devuelve las filas de datos;
    - si la tabla supera 1 048 575 filas, devuelve `ErrorProceso::Datos` («demasiadas filas para Excel»).
  - `procesar` escribe en `.en-curso-<marca>/excel/` un archivo por tabla Gold exportada:
    `planilla.xlsx`, `base_personal_empleados.xlsx`, `base_personal_practicantes.xlsx` y `centros_costo.xlsx`,
    los que existan en esa ejecución. Cada uno se agrega a `Ejecucion.salidas` como `("excel/<nombre>.xlsx", filas)`.

- [ ] **Paso 1: escribir el test que falla.** En `tests/proceso.rs`, `escribe_excel_de_revision`: procesar
  limpio completo y verificar:
  - existen los 4 `.xlsx` en `actual/excel/`;
  - al abrir `planilla.xlsx` con `calamine`, la hoja `planilla` tiene como fila 1 los nombres de columna de
    `gold/planilla.parquet` (leídos con DuckDB) y tantas filas de datos como `Ejecucion.salidas` informa para
    `gold/planilla.parquet`;
  - una columna de fecha conocida de la Gold de base de personal se lee como `Data::DateTime`.
  Agregar además `solo_base_no_escribe_excel_de_planilla`.
- [ ] **Paso 2:** `cargo test -p etl-local-core --test proceso -j 2`.
  Esperado: FALLA (no existe `actual/excel/`).
- [ ] **Paso 3:** implementar `exportar_excel` leyendo con `SELECT * FROM <tabla>`, recorriendo filas con
  `duckdb::types::ValueRef` y escribiendo con `rust_xlsxwriter::Workbook`. Llamarla dentro del bloque de
  escritura atómica, después de los Parquet y antes de `ejecucion.json`.
- [ ] **Paso 4:** `cargo test -p etl-local-core -j 2`.
  Esperado: pasa todo. El Plan 5 lee solo `gold/*.parquet` y no se ve afectado.
- [ ] **Paso 5:** registrar la tarea en el ledger.

### Tarea 3: Avance y cancelación del proceso

**Archivos:**
- Modificar: `crates/core/src/proceso.rs` y `crates/core/src/error.rs`.
- Test: `crates/core/tests/proceso.rs`.

**Interfaces:**
- Produce:
  - `#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)] #[serde(rename_all = "snake_case")] pub enum Paso { Leer, Licencias, Planilla, BasePersonal, Controles, Guardar }`;
  - `#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)] pub struct Avance { pub paso: Paso, pub indice: usize, pub total: usize }`,
    donde `indice` es la posición del paso en `pasos(entradas)`, desde 0;
  - `pub fn pasos(entradas: &Entradas) -> Vec<Paso>`. El orden es `Leer`, `Licencias` (si hay planilla o
    licencias), `Planilla` (si hay planilla), `BasePersonal` (si hay base), `Controles` y `Guardar`;
  - `pub fn procesar_con(config: &Path, entradas: &Entradas, salida: &Path, marca: &str, avance: &mut dyn FnMut(Avance), cancelar: &AtomicBool) -> Result<Ejecucion, ErrorProceso>`;
  - `procesar` sigue con la misma firma y llama a `procesar_con` con un `avance` vacío y
    `&AtomicBool::new(false)`;
  - `ErrorProceso::Cancelado`, con el mensaje «Se detuvo el proceso; no se cambió ningún reporte.».

- [ ] **Paso 1: escribir los tests que fallan.**
  - `avance_recorre_los_pasos_en_orden`: procesar limpio completo, recolectar los `Avance` y verificar que los
    `paso` son exactamente `pasos(&entradas)` en orden, con `indice` de 0 a `total - 1` y `total == 6`.
  - `pasos_solo_base`: da `[Leer, BasePersonal, Controles, Guardar]`.
  - `cancelar_antes_de_guardar_no_cambia_nada`:
    1. procesar una vez (para tener `actual/`);
    2. procesar de nuevo con un `avance` que activa `cancelar` al recibir `Paso::Controles`;
    3. verificar `Err(ErrorProceso::Cancelado)`, `actual/ejecucion.json` igual al de antes, ninguna entrada
       `.en-curso-*` en la salida y ninguna carpeta nueva en `historico/`.
- [ ] **Paso 2:** `cargo test -p etl-local-core --test proceso -j 2`.
  Esperado: no compila.
- [ ] **Paso 3: implementar.**
  - Emitir el `Avance` **al empezar** cada paso.
  - Revisar `cancelar` (`Ordering::Relaxed`) antes de cada paso y justo antes de `promover`. Después de
    `promover` ya no se cancela.
  - Al cancelar, limpiar `.en-curso-<marca>` igual que en el camino de error.
- [ ] **Paso 4:** `cargo test -p etl-local-core -j 2`.
  Esperado: pasa todo.
- [ ] **Paso 5:** registrar la tarea en el ledger.

### Tarea 4: Catálogo de reportes, entradas guardadas, `actualizar` y comando `actualizar` del CLI

**Archivos:**
- Crear: `config/reportes.yaml`, `crates/core/src/reportes.rs` y `crates/core/src/entradas.rs`.
- Modificar: `crates/core/src/lib.rs`, `crates/cli/src/main.rs`, `crates/cli/src/procesar.rs` (salida humana
  compartida) y `config/README.md`.
- Crear: `crates/cli/src/actualizar.rs`.
- Tests: `crates/core/tests/reportes.rs` y `crates/core/tests/entradas.rs` (nuevos), y `crates/cli/tests/cli.rs`.

**Interfaces:**
- `config/reportes.yaml`:
  ```yaml
  # Reportes que ofrece la app (pantalla 1). fuente y complementos: contratos de config/fuentes/.
  # excel: archivo de actual/excel/ que abre «Abrir en Excel». genero: concordancia de «Tu … está lista/o».
  reportes:
    - id: planilla
      titulo: Planilla mensual
      descripcion: Horas extra, subsidios, licencias y salidas
      fuente: planilla
      varios_archivos: true
      complementos: [licencias]
      excel: planilla
      genero: femenino
    - id: base_personal
      titulo: Base de personal
      descripcion: Quién trabaja hoy y alertas de contrato y edad
      fuente: base_personal
      varios_archivos: false
      complementos: []
      excel: base_personal_empleados
      genero: femenino
  ```
- Produce en `reportes.rs`:
  - `pub struct Reporte { id, titulo, descripcion, fuente, varios_archivos: bool, complementos: Vec<String>, excel, genero: Genero }`
    (Deserialize con `deny_unknown_fields` y Serialize);
  - `pub enum Genero { Femenino, Masculino }`;
  - `pub fn cargar_reportes(config: &Path) -> Result<Vec<Reporte>, ErrorContrato>`. Valida:
    - `id` único y con formato `[a-z_]+`;
    - `fuente` y cada complemento existen en `config/fuentes/`;
    - `excel` con formato `[a-z_]+`.
  - `pub enum Vigencia { Nunca, AlDia, Atrasado }` (`snake_case`);
  - `pub struct EstadoReporte { pub reporte: Reporte, pub vigencia: Vigencia, pub ultimo: Option<FechaNombre> }`;
  - `pub fn estado_reportes(config: &Path, salida: &Path, hoy: NaiveDate) -> Result<Vec<EstadoReporte>, ErrorProceso>`.
    Reglas:
    - `ultimo` es el mayor `fecha_en_nombre` de los archivos de `entradas/<fuente>/`;
    - un `Periodo` está al día si es igual o posterior al mes anterior al de `hoy`;
    - una `FechaCorte` está al día si `hoy - fecha <= 31` días;
    - sin archivos, `Nunca`.
- Produce en `entradas.rs`:
  - `pub fn guardadas(config: &Path, salida: &Path) -> Result<Entradas, ErrorProceso>`: lee
    `<salida>/entradas/{planilla,licencias,base_personal}/`;
  - `pub fn combinar(config: &Path, guardadas: &Entradas, nuevas: &Entradas) -> Result<Entradas, ErrorProceso>`:
    - una planilla nueva reemplaza a la guardada del mismo periodo;
    - una licencia o base nueva reemplaza a la guardada;
    - dos planillas nuevas del mismo periodo, o una planilla sin periodo en el nombre, dan
      `ErrorProceso::Datos` con el nombre de los archivos;
  - `pub fn actualizar(config: &Path, salida: &Path, nuevas: &Entradas, excluir: &[String], marca: &str, avance: &mut dyn FnMut(Avance), cancelar: &AtomicBool) -> Result<Ejecucion, ErrorProceso>`:
    1. `combinar(guardadas, nuevas)`;
    2. quita las fuentes de `excluir` (por ejemplo, `licencias` si el usuario desmarca «Agregar también el
       control de licencias») **solo de esta ejecución**: sus archivos guardados no se borran;
    3. `procesar_con`;
    4. **solo si sale bien**, sincroniza `entradas/` con el conjunto usado: copia los archivos nuevos con su
       nombre original y borra los reemplazados. Las fuentes excluidas no se tocan.
    Si la copia falla después de publicar, devuelve `ErrorProceso::Salida` (ruling: el reporte ya quedó
    publicado; el mensaje lo dice).
    - Copiar un archivo sobre sí mismo (misma ruta canónica) no hace nada.
- CLI: `etl-local actualizar --salida <carpeta> [--config config] [--planilla <archivos>…] [--licencias <archivo>] [--base-personal <archivo>] [--sin <fuente>…] [--json]`.
  La salida y los códigos de salida son los de `procesar`. Sin archivos nuevos, reprocesa lo guardado; si
  tampoco hay nada guardado, da el error «No hay archivos para procesar.» con código 2.

- [ ] **Paso 1: escribir los tests que fallan.**
  - En `tests/reportes.rs`:
    - `catalogo_del_repo_es_valido`: `cargar_reportes(config)` da los ids `["planilla", "base_personal"]`;
    - `fuente_inexistente_falla`: un yaml temporal con `fuente: nomina` da `Err` cuyo mensaje nombra «nomina»;
    - `vigencia`: con `hoy = 2025-12-05`, el periodo `2025-11` es `AlDia` y `2025-10` es `Atrasado`; la fecha
      de corte `2025-11-12` es `AlDia` y `2025-10-01` es `Atrasado`; sin `entradas/`, `Nunca`.
  - En `tests/entradas.rs`, con `tempfile` y una salida llamada `Reportes RR. HH./año ñ`:
    - `primera_actualizacion_guarda_las_entradas`: `actualizar` con las 3 planillas limpias, licencias y base
      deja copias en `entradas/planilla/` (3), `entradas/licencias/` (1) y `entradas/base_personal/` (1), con
      los mismos bytes que los originales;
    - `actualizar_solo_base_conserva_la_planilla`: tras la primera, `actualizar` con solo la base da una
      `Ejecucion` cuyos `periodos` son los 3 de la planilla y que trae `gold/planilla.parquet` en `salidas`;
    - `mes_nuevo_reemplaza_al_guardado`: copiar `Planilla 2025-11 Empleados.xlsx` a un temporal como
      `Planilla 2025-11 Empleados Final.xlsx` y actualizar con él deja en `entradas/planilla/` el archivo
      «Final» y no el anterior;
    - `dos_nuevas_del_mismo_mes_fallan`: `combinar` da `Err(Datos)` y el mensaje nombra ambos archivos;
    - `elegir_un_archivo_ya_guardado_no_lo_trunca`: actualizar pasando como nueva la ruta
      `entradas/base_personal/BD. 12.11.2025..xlsx` termina bien y el archivo conserva su tamaño;
    - `excluir_licencias_no_las_borra`: tras la primera, `actualizar` con `excluir = ["licencias"]` da una
      `Ejecucion` sin `silver/licencias.parquet` en `salidas`, y `entradas/licencias/` conserva su archivo;
    - `falla_no_toca_entradas`: actualizar con CASO-01 da `Err(Entradas)` y `entradas/` queda igual (mismos
      nombres);
    - `#[cfg(windows)] excel_abierto_bloquea_y_no_toca_entradas`: con un `File::open` de
      `actual/excel/planilla.xlsx` vivo, actualizar con la base da `Err(ErrorProceso::Publicar { .. })` y
      `entradas/base_personal/` sigue con el archivo anterior. Si en esta máquina Windows permite el rename
      con el archivo abierto, se ledgera un ruling y el test se reemplaza por un handle con
      `share_mode(0)`.
  - En `crates/cli/tests/cli.rs`:
    - `actualizar_reprocesa_lo_guardado`: `actualizar` con todo y luego `actualizar --base-personal <base>`
      salen con 0, y el JSON de la segunda trae los 3 periodos;
    - `actualizar_sin_nada_sale_2`.
- [ ] **Paso 2:** `cargo test -p etl-local-core --test reportes --test entradas -j 2`.
  Esperado: no compila.
- [ ] **Paso 3:** implementar `reportes.rs`, `entradas.rs` y el comando del CLI. En el CLI, la marca sale del
  reloj, igual que en `procesar`; extraer `humana` a un lugar común.
- [ ] **Paso 4:** `cargo test -j 2`.
  Esperado: pasa todo (core, api y cli).
- [ ] **Paso 5:** agregar a `config/README.md` una fila para `reportes.yaml` y registrar la tarea en el ledger.

### Tarea 5: Esqueleto de la app (`app/ui` + `app/src-tauri`) con estilos y tipografías

**Archivos:**
- Crear en `app/ui/`: `package.json`, `package-lock.json` (generado con `npm install`), `vite.config.ts`,
  `svelte.config.js`, `tsconfig.json`, `app.html`, `src/app/main.ts`, `src/app/App.svelte`,
  `src/lib/estilos/tokens.css`, `src/lib/estilos/base.css`, `src/lib/estilos/fuentes.css`,
  `src/lib/formato.ts` y `src/lib/formato.test.ts`.
- Crear en `app/src-tauri/`: `Cargo.toml` (paquete `etl-local-app`), `build.rs`, `tauri.conf.json`,
  `capabilities/principal.json`, `icons/` (generados), `src/main.rs` y `src/lib.rs`.
- Modificar: `Cargo.toml` del workspace (miembro `app/src-tauri` y
  `default-members = ["crates/core", "crates/api", "crates/cli"]`) y `.gitignore`, si existe (si no, crearlo
  con `node_modules/`, `dist/` y `target/`).

**Interfaces:**
- `package.json`:
  - scripts: `dev` (`vite`), `build` (`vite build`), `test` (`vitest run`), `check` (`svelte-check --fail-on-warnings`)
    y `tauri` (`tauri`);
  - dependencias: `svelte`, `@tauri-apps/api`, `@tauri-apps/plugin-dialog`, `@tauri-apps/plugin-opener`,
    `@fontsource/atkinson-hyperlegible`, `@fontsource/bricolage-grotesque` y `@fontsource/jetbrains-mono`;
  - de desarrollo: `vite`, `@sveltejs/vite-plugin-svelte`, `typescript`, `vitest`, `svelte-check` y
    `@tauri-apps/cli`;
  - versiones de ADR-0005 para las que figuran ahí; las demás, la última 2.x o 5.x que resuelva npm, fijada
    en el lock.
- Vite: multipágina con `build.rollupOptions.input = { app: "app.html" }` (el Plan 7 agrega `tablero.html`),
  `server.port = 5173` y `strictPort: true`.
- `tsconfig.json`: `strict: true`, `noUncheckedIndexedAccess: true` y `verbatimModuleSyntax: true`.
- `tauri.conf.json`:
  - `productName` «Integrador RR. HH.» e `identifier` `dev.etl-local.integrador`;
  - `build.devUrl` `http://localhost:5173/app.html`, `build.frontendDist` `../ui/dist`,
    `beforeDevCommand` `npm --prefix ../ui run dev` y `beforeBuildCommand` `npm --prefix ../ui run build`
    (si la CLI resuelve rutas distintas, se ledgera un ruling con lo que funcionó);
  - una ventana `principal`: `url` `app.html`, 1280×860, mínimo 960×700, título «Integrador RR. HH.»;
  - `app.security.csp`: `default-src 'self'; style-src 'self' 'unsafe-inline'; font-src 'self'; img-src 'self' data:; connect-src ipc: http://ipc.localhost`;
  - `bundle.resources`: `{ "../../config/": "config/" }`;
  - `bundle.icon` con los generados.
- `capabilities/principal.json`: ventana `principal` y permisos `core:default` y `dialog:allow-open`, nada más.
  Abrir archivos y carpetas pasa por comandos propios (Tarea 6), no por permisos del *opener* en JS.
- `src/lib.rs` expone los módulos (Tarea 6). `src/main.rs` arma el `tauri::Builder` con los plugins `dialog` y
  `opener` y `tauri::generate_context!()` (en `main.rs`, para que `cargo test --lib` no necesite `dist/`).
- `src/lib/formato.ts`:
  - `nombreMes(periodo: string): string`: `"2025-09"` da `"Septiembre 2025"`;
  - `fechaCorta(iso: string): string`: `"2025-11-12"` da `"12 nov 2025"`;
  - `numero(n: number): string`: separador de miles `.` también con 4 cifras, decimales con `,`;
  - `saludo(hora: number): string`: «Buenos días» (< 12), «Buenas tardes» (< 19) o «Buenas noches»;
  - `listaMeses(periodos: string[]): string`: `["2025-09","2025-10","2025-11"]` da
    `"septiembre, octubre y noviembre"`.
- `App.svelte`, de momento: encabezado con la marca y el texto «¿Qué quieres actualizar hoy?», para probar el
  arranque.

- [ ] **Paso 1: escribir el test que falla.** `formato.test.ts` con los casos de arriba, además de
  `numero(1227) === "1.227"`, `numero(86) === "86"`, `numero(3715) === "3.715"`, `numero(7.29) === "7,29"` y
  `nombreMes("2025-01") === "Enero 2025"`.
- [ ] **Paso 2:** `npm --prefix app/ui install`, luego `npm --prefix app/ui test`.
  Esperado: FALLA (no existe `formato.ts`).
- [ ] **Paso 3:** implementar:
  - `formato.ts`, sin `Intl` para los miles: `es-ES` no agrupa los números de 4 cifras;
  - `tokens.css` con las variables `--color-fondo`, `--color-superficie`, `--color-texto`, `--color-texto-2`,
    `--color-accion`, `--color-aviso-fondo`, `--color-aviso-texto`, `--color-error`, `--color-error-fondo`,
    `--color-borde` (`#DDE2DC`) y `--alto-control` (44px) / `--alto-principal` (52px), tomadas de
    diseno §Lenguaje visual;
  - `fuentes.css`, que importa solo los pesos usados: Atkinson 400 y 700, Bricolage 700 y JetBrains Mono 500,
    subconjuntos `latin` y `latin-ext`;
  - el esqueleto Tauri;
  - los íconos, con `npm --prefix app/ui run tauri -- icon <svg>` desde `app/`, a partir de un SVG del logo
    del mock (cuadrado verde `#1E6B47` con 4 cuadrados blancos) guardado en `app/src-tauri/icons/logo.svg`.
- [ ] **Paso 4: verificar.**
  - `npm --prefix app/ui test` y `npm --prefix app/ui run check`. Esperado: pasan, con 0 errores y 0 avisos.
  - `npm --prefix app/ui run build`. Esperado: genera `app/ui/dist/app.html`.
  - `cargo build -p etl-local-app -j 2`, en segundo plano (compila Tauri la primera vez). Esperado: compila;
    `libduckdb-sys` no se recompila desde cero.
  - `npm --prefix app/ui run tauri -- info` desde `app/`. Esperado: sin avisos de versiones desparejas entre
    crates y paquetes npm de Tauri.
- [ ] **Paso 5:** registrar la tarea en el ledger, con las versiones exactas que quedaron en `Cargo.lock` y
  `package-lock.json`.

### Tarea 6: Backend Tauri (ajustes, servicio y comandos)

**Archivos:**
- Crear: `app/src-tauri/src/ajustes.rs`, `app/src-tauri/src/servicio.rs`, `app/src-tauri/src/comandos.rs`,
  `app/src-tauri/src/error.rs` y `app/src-tauri/tests/ejemplos/*.json` (JSON de referencia).
- Modificar: `app/src-tauri/src/lib.rs` y `app/src-tauri/src/main.rs` (registrar comandos y estado).
- Tests: los `#[cfg(test)]` de `servicio.rs` y `ajustes.rs`.

**Interfaces:**
- `ajustes.rs`:
  - `pub struct Ajustes { pub carpeta_salida: Option<PathBuf>, pub duraciones_ms: BTreeMap<String, u64> }`
    (serde, `default`);
  - `pub fn cargar(ruta: &Path) -> Ajustes`: un archivo inexistente o ilegible da `Ajustes::default()`;
  - `pub fn guardar(ruta: &Path, a: &Ajustes) -> std::io::Result<()>`;
  - el archivo es `app_config_dir()/ajustes.json`.
- `servicio.rs` (sin tipos de Tauri):
  - `pub struct Rutas { pub config: PathBuf, pub salida: PathBuf }`;
  - `pub fn resolver_rutas(env_config: Option<PathBuf>, env_salida: Option<PathBuf>, recursos: &Path, documentos: &Path, ajustes: &Ajustes) -> Rutas`:
    - la config se elige en este orden: `ETL_LOCAL_CONFIG`, luego `recursos/config` si existe
      `recursos/config/fuentes`; en debug, `CARGO_MANIFEST_DIR/../../config`;
    - la salida se elige en este orden: `ETL_LOCAL_SALIDA`, luego `ajustes.carpeta_salida`, luego
      `documentos/Reportes RR. HH.`;
  - `pub fn carpeta_visible(salida: &Path, documentos: &Path) -> String`: `documentos/Reportes RR. HH.` da
    `"Documentos › Reportes RR. HH."`; fuera de Documentos, la ruta completa;
  - tipos serializables, con campos en `snake_case` como el resto del contrato:
    - `ArchivoElegido { ruta: String, nombre: String, carpeta: String, bytes: u64, modificado: Option<String> /* ISO fecha */, fecha_nombre: Option<FechaNombre> }`;
    - `ProblemaVista { problema: Problema, recorte: Option<Recorte> }`;
    - `RevisionArchivo { archivo: ArchivoElegido, personas: Option<u64>, problemas: Vec<ProblemaVista> }`;
    - `TarjetaReporte { estado: EstadoReporte, duracion_prevista_ms: Option<u64> }`;
    - `Inicio { tarjetas: Vec<TarjetaReporte>, carpeta_visible: String }`;
    - `Cifra { valor: String, etiqueta: String }`;
    - `Resultado { ejecucion: Ejecucion, titulo_periodos: String, cifras: Vec<Cifra>, para_revisar: Vec<Problema> }`;
  - `pub fn rango_recorte(p: &Problema) -> Option<(Celda, Celda)>`:
    - con `celda` y `celda_encontrada`, de la fila `min - 1` (≥ 1) a la `max + 1`, y de la columna de la
      celda esperada a esa columna + 3;
    - con solo `celda`, de la fila `celda - 1` a la `celda + 2`, y de la columna de la celda a esa columna + 3;
    - sin `hoja` o sin `celda`, `None`;
  - `pub fn reconocer(config: &Path, reporte: &Reporte, rutas: &[PathBuf]) -> Vec<ArchivoElegido>`: solo
    nombre y metadatos, sin abrir el Excel;
  - `pub fn revisar(config: &Path, reporte: &Reporte, rutas: &[PathBuf]) -> Result<Vec<RevisionArchivo>, ErrorApp>`:
    el contrato de cada ruta es el de la fuente del reporte, salvo que su nombre coincida con el
    `patron_nombre` de un complemento;
  - `pub fn resultado(config: &Path, salida: &Path, reporte: &Reporte, ej: Ejecucion) -> Result<Resultado, ErrorApp>`.
    Las cifras dependen del reporte:
    - **planilla:** IND-01 del último periodo (`Tablero::indicadores(&Filtros::default())`) con la etiqueta
      «personas en la planilla de {mes}»; `{n} meses` (o «1 mes») con la etiqueta de los meses abreviados
      («sep · oct · nov»); y el número de avisos con la etiqueta «avisos para revisar»;
    - **base_personal:** filas de `gold/base_personal_empleados.parquet` según `ej.salidas` («empleados»),
      IND-07 («contratos por superar su plazo») e IND-08 («cumplen la edad de jubilación»);
    - `para_revisar` son los problemas `Aviso` de `ej.problemas`;
    - `titulo_periodos`: «Septiembre a noviembre de 2025», «Noviembre 2025» o «Corte: 12 nov 2025».
- `error.rs`: `#[derive(Serialize)] pub struct ErrorApp { pub codigo: String, pub mensaje: String, pub problemas: Vec<Problema> }`
  con `From<ErrorProceso>`:
  - `Entradas` da `codigo` `"entradas"` y aplana los problemas;
  - `Cancelado` da `"cancelado"`;
  - `Publicar` da `"archivo_abierto"`;
  - el resto da `"interno"`.
- `comandos.rs` (todos `#[tauri::command]`, con el estado `Estado { rutas: Rutas, ajustes_ruta: PathBuf, cancelar: Arc<AtomicBool>, en_curso: AtomicBool, revisados: Mutex<HashSet<PathBuf>> }`):
  - `inicio() -> Result<Inicio, ErrorApp>`, que usa la fecha local como `hoy`;
  - `reconocer(reporte: String, rutas: Vec<String>) -> Result<Vec<ArchivoElegido>, ErrorApp>`;
  - `complemento_guardado(fuente: String) -> Result<Option<ArchivoElegido>, ErrorApp>`;
  - `async revisar(reporte: String, rutas: Vec<String>) -> Result<Vec<RevisionArchivo>, ErrorApp>`, en
    `spawn_blocking`, que agrega las rutas a `revisados`;
  - `async procesar(reporte: String, rutas: Vec<String>, complementos: Vec<String>, excluir: Vec<String>, avance: Channel<Avance>) -> Result<Resultado, ErrorApp>`:
    - con `en_curso` activo, devuelve `ErrorApp { codigo: "en_curso", mensaje: "Ya hay un proceso en curso." }`;
    - si no, pone `cancelar` en falso, arma las `Entradas` nuevas (`rutas` en la fuente del reporte,
      `complementos` en su fuente), toma la marca del reloj y llama a `actualizar(…, &excluir, …)` en
      `spawn_blocking`;
    - al terminar bien, guarda `duraciones_ms[reporte]` y llama a `window.request_user_attention`
      (Informational).
  - `detener()`: pone `cancelar` en verdadero;
  - `abrir(que: Abrir) -> Result<(), ErrorApp>`, con
    `#[serde(tag = "tipo", rename_all = "snake_case")] enum Abrir { Carpeta, Excel { reporte: String }, Archivo { ruta: String } }`:
    - `Carpeta` crea la salida si no existe;
    - `Excel` abre `<salida>/actual/excel/<reporte.excel>.xlsx` y, si no existe, devuelve
      «Todavía no hay un Excel de este reporte. Procésalo primero.»;
    - `Archivo` solo abre rutas que estén en `revisados` y, si no, devuelve `codigo` `"no_permitido"`;
    - abre con `tauri_plugin_opener::OpenerExt`.

- [ ] **Paso 1: escribir los tests que fallan** (en `servicio.rs` y `ajustes.rs`):
  - `ajustes_inexistentes_dan_defecto` y `ajustes_ida_y_vuelta`;
  - `rutas_prioridad`: la variable de entorno gana a los ajustes, que ganan al valor por defecto;
  - `carpeta_visible_en_documentos` y `carpeta_visible_fuera`;
  - `rango_recorte_desplazada`: A6 y A7 dan A5–D8; `rango_recorte_solo_celda`: C10 da C9–F12;
    `rango_recorte_sin_hoja` da `None`;
  - `reconocer_planilla`: las 3 planillas limpias dan los periodos `2025-09`, `2025-10` y `2025-11`, y `bytes > 0`;
  - `revisar_caso_01_trae_recorte`: el único problema trae un `recorte` con `fila_inicio == 5`;
  - `revisar_planilla_reconoce_licencias_como_complemento`: el archivo de licencias limpio, pasado en `rutas`
    del reporte planilla, se revisa con el contrato `licencias` y no da errores;
  - `resultado_planilla`: tras `actualizar` limpio completo en un temporal, la primera cifra es
    `numero(IND-01)` con la etiqueta «personas en la planilla de noviembre» y `titulo_periodos` es
    «Septiembre a noviembre de 2025»;
  - `error_app_desde_entradas`: un `ErrorProceso::Entradas` con 2 informes da `codigo == "entradas"` y los
    problemas aplanados;
  - `ejemplos_json_al_dia`: serializar un valor fijo de `Inicio`, `RevisionArchivo` (con recorte),
    `Avance`, `Resultado` y `ErrorApp`, y comparar con `tests/ejemplos/<tipo>.json`. Si la variable
    `ACTUALIZAR_EJEMPLOS=1` está definida, el test reescribe los archivos.
- [ ] **Paso 2:** `npm --prefix app/ui run build`, luego `cargo test -p etl-local-app --lib -j 2`.
  Esperado: no compila.
- [ ] **Paso 3:** implementar. Generar los ejemplos con `ACTUALIZAR_EJEMPLOS=1` una vez y revisarlos a mano:
  no deben tener rutas reales de la máquina, solo rutas ficticias como `C:/Usuarios/demo/...`.
- [ ] **Paso 4:** `cargo test -p etl-local-app --lib -j 2` y `cargo clippy --workspace --all-targets -j 2 -- -D warnings`.
  Esperado: pasan.
- [ ] **Paso 5:** registrar la tarea en el ledger.

### Tarea 7: Estado del flujo, textos y backend simulado (TypeScript puro)

**Archivos:**
- Crear en `app/ui/src/app/`: `tipos.ts`, `backend.ts` (interfaz y backend real con `invoke`, `Channel`, el
  diálogo y `onDragDropEvent`), `simulado.ts` (backend falso con datos sintéticos), `flujo.svelte.ts`,
  `textos.ts`, `eta.ts` y sus tests (`flujo.test.ts`, `textos.test.ts`, `eta.test.ts` y `tipos.test.ts`).

**Interfaces:**
- `tipos.ts`: los tipos TS de la Tarea 6 (`ArchivoElegido`, `RevisionArchivo`, `ProblemaVista`, `Problema`,
  `Recorte`, `Inicio`, `TarjetaReporte`, `EstadoReporte`, `Reporte`, `Avance`, `Paso`, `Resultado`, `Cifra`,
  `Ejecucion`, `ErrorApp`, `Abrir`, `FechaNombre`), escritos a mano. Se suma
  `export const CAMPOS: Record<string, string[]>` con las claves de primer nivel de cada tipo. El mecanismo
  derivado llega en el Plan 7 (ADR-0005 §3).
- `backend.ts`:
  ```ts
  export interface Backend {
    inicio(): Promise<Inicio>;
    reconocer(reporte: string, rutas: string[]): Promise<ArchivoElegido[]>;
    complementoGuardado(fuente: string): Promise<ArchivoElegido | null>;
    revisar(reporte: string, rutas: string[]): Promise<RevisionArchivo[]>;
    procesar(reporte: string, rutas: string[], complementos: string[], excluir: string[], alAvanzar: (a: Avance) => void): Promise<Resultado>;
    detener(): Promise<void>;
    abrir(que: Abrir): Promise<void>;
    elegirArchivos(multiple: boolean): Promise<string[]>;
    alSoltarArchivos(cb: (rutas: string[]) => void): Promise<() => void>;
  }
  export function backendTauri(): Backend;
  ```
  Los errores de `invoke` llegan como `ErrorApp`. El backend real los relanza como `class FalloApp extends Error { app: ErrorApp }`.
- `simulado.ts`: `export function backendSimulado(opciones?: { conError?: boolean; demoraMs?: number }): Backend`.
  - Sin `conError`, 3 planillas de 2025-09 a 2025-11 sin errores, la segunda con un aviso.
  - Con `conError`, la tercera trae PRE-10 en A6, encontrada en A7, con un recorte de 4×4. Los textos de las
    celdas son inventados («APELLIDO PATERNO», «Pérez»…).
  - `procesar` emite los 6 avances con `demoraMs` entre cada uno.
- `flujo.svelte.ts`:
  ```ts
  export type Pantalla = "inicio" | "archivos" | "revision" | "procesando" | "listo";
  export class Flujo {
    pantalla: Pantalla; inicio: Inicio | null; reporte: Reporte | null;
    elegidos: ArchivoElegido[]; complemento: { usar: boolean; archivo: ArchivoElegido | null };
    revisiones: RevisionArchivo[]; avance: Avance | null; pasos: Paso[]; resultado: Resultado | null;
    error: ErrorApp | null; aviso: string | null; // banner (p. ej. tras «Detener» o archivos ignorados)
    constructor(backend: Backend, reloj?: () => number);
    cargarInicio(): Promise<void>;
    elegirReporte(id: string): Promise<void>;          // → "archivos"; carga complemento guardado
    agregar(rutas: string[]): Promise<void>;            // filtra .xlsx/.xlsm/.xls; ignora el resto con aviso;
                                                        // base: un solo archivo (el último), con aviso
    quitar(ruta: string): void;
    get puedeRevisar(): { ok: boolean; motivo: string | null };
    revisar(): Promise<void>;                           // → "revision"
    revisarDeNuevo(): Promise<void>;
    get puedeProcesar(): { ok: boolean; motivo: string | null };
    get sinLosConError(): ArchivoElegido[];             // para «Seguir sin …»
    seguirSin(): Promise<void>;                         // procesa excluyendo los que tienen error
    procesar(): Promise<void>;                          // → "procesando" → "listo" | error
    detener(): Promise<void>;                           // al volver Cancelado → "inicio" con aviso
    volverAlInicio(): Promise<void>;
  }
  ```
  - `pasos` en la UI replica `pasos()` del núcleo con lo elegido; el orden está en la Tarea 3.
  - Complemento:
    - `complemento.usar` arranca en verdadero si hay un archivo guardado o si se eligió uno;
    - con `usar` verdadero y un archivo nuevo elegido, su ruta va en `complementos` (y también se revisa);
    - con `usar` verdadero y solo el guardado, no se pasa nada: el núcleo ya lo usa;
    - con `usar` falso, la fuente va en `excluir`;
    - test: `desmarcar licencias las excluye sin borrarlas`, que verifica los argumentos de `procesar`.
  - Motivos de `puedeRevisar`:
    - «Elige al menos un archivo.»;
    - «Hay dos archivos de {Mes}; quita uno.»;
    - «No reconocemos el mes de «{archivo}». Revisa el nombre del archivo.» (para la base: «la fecha de corte»).
  - Motivo de `puedeProcesar`: «Se activa cuando todos los archivos estén en orden.».
- `textos.ts`:
  - `textoPaso(paso: Paso, ctx: { archivos: number; periodos: string[] }): string`:
    - `Leer`: «Leer los {n} archivos de Excel» (o «Leer el archivo de Excel»);
    - `Licencias`: «Agregar las licencias de cada persona»;
    - `Planilla`: «Juntar {listaMeses}» (con un solo mes: «Preparar {mes en minúscula}»);
    - `BasePersonal`: «Preparar la base de personal y sus alertas»;
    - `Controles`: «Revisar que los datos cuadren»;
    - `Guardar`: «Guardar el reporte»;
  - `tituloRevision(revs: RevisionArchivo[]): string`;
  - `textoBotonProcesar(reporte: Reporte, n: number): string`;
  - `textoSeguirSin(revs): string`: «Seguir sin noviembre» o «Seguir sin octubre y noviembre»;
  - `pasosArreglo(p: Problema): string[]`:
    - para PRE-10 con `celda_encontrada`: [«Abre el archivo y deja los títulos de las columnas empezando en {celda}.», «Guarda y cierra el archivo.», «Vuelve aquí y pulsa «Revisar de nuevo».»];
    - para el resto: [«Abre el archivo y corrige lo indicado.», «Guarda y cierra el archivo.», «Vuelve aquí y pulsa «Revisar de nuevo».»];
  - `tituloListo(reporte: Reporte): string`: «Tu planilla mensual está lista».
- `eta.ts`: `textoEta(previstaMs: number | null, transcurridoMs: number): string | null`:
  - sin previsión, `null`;
  - si quedan 5 s o más: «Quedan unos {s} segundos», con `s` redondeado hacia arriba a múltiplos de 5;
  - si quedan entre 0 y 5 s: «Falta poco»;
  - si ya se pasó: «Está tardando más de lo habitual».

- [ ] **Paso 1: escribir los tests que fallan** (Vitest, entorno `node`, con `backendSimulado`):
  - `flujo.test.ts`:
    - `elegir planilla pasa a archivos y carga el complemento guardado`;
    - `agregar ignora un PDF y avisa` (el aviso nombra el archivo);
    - `base de personal se queda con un solo archivo`;
    - `dos archivos del mismo mes bloquean la revisión con su motivo`;
    - `mes no reconocido bloquea la revisión`;
    - `revisión con error desactiva procesar y ofrece seguir sin ese mes`;
    - `seguirSin procesa solo los archivos sin error` (verificar con un espía las rutas que recibe `procesar`);
    - `procesar recorre los avances y termina en listo`;
    - `detener vuelve al inicio con el aviso de cancelado` (el simulado devuelve `FalloApp` con `cancelado`);
    - `error de entradas al procesar muestra los problemas`.
  - `textos.test.ts`: cada rama de `textoPaso`, `tituloRevision` (0, 1 y 2 archivos con error),
    `textoSeguirSin` y `pasosArreglo`.
  - `eta.test.ts`: `textoEta(null, 0) === null`, `textoEta(30000, 8000) === "Quedan unos 25 segundos"`,
    `textoEta(30000, 27000) === "Falta poco"` y `textoEta(30000, 40000) === "Está tardando más de lo habitual"`.
  - `tipos.test.ts`: para cada `app/src-tauri/tests/ejemplos/<tipo>.json` (importado como JSON), las claves de
    primer nivel son iguales a `CAMPOS[<tipo>]`.
- [ ] **Paso 2:** `npm --prefix app/ui test`.
  Esperado: FALLA (no existen los módulos).
- [ ] **Paso 3:** implementar. `Flujo` usa `$state` en sus campos y `$derived` donde aplique; los getters
  calculan desde el estado.
- [ ] **Paso 4:** `npm --prefix app/ui test` y `npm --prefix app/ui run check`.
  Esperado: pasan, con 0 errores y 0 avisos.
- [ ] **Paso 5:** registrar la tarea en el ledger.

### Tarea 8: Pantallas Inicio y Elegir archivos

**Archivos:**
- Crear:
  - en `app/ui/src/lib/componentes/`: `Encabezado.svelte`, `Pasos.svelte`, `Boton.svelte`, `Aviso.svelte` y
    `Icono.svelte` (SVG de trazo de los mocks);
  - en `app/ui/src/app/pantallas/`: `Inicio.svelte`, `TarjetaReporte.svelte`, `Archivos.svelte`,
    `ZonaArrastre.svelte` y `FilaArchivo.svelte`.
- Modificar: `App.svelte` (elige la pantalla según `flujo.pantalla`) y `main.ts`:
  - usa `backendSimulado({ conError })` si `import.meta.env.DEV` y la URL trae `?simulado`, con `conError`
    verdadero cuando también trae `error`;
  - usa `backendTauri()` si no;
  - el código simulado no debe quedar en el build de producción: verificar con `grep` sobre `dist/` que no
    aparece «APELLIDO PATERNO».

**Interfaces:**
- `Boton.svelte`:
  - props `variante: "principal" | "secundario" | "enlace"`, `deshabilitado?: boolean`, `motivo?: string`,
    `onclick` y `children`;
  - con `deshabilitado` y `motivo`, muestra el motivo debajo y lo enlaza con `aria-describedby`.
- `Pasos.svelte`: prop `actual: 1 | 2 | 3 | 4`. Los pasos anteriores llevan check y la clase «hecho»; el actual
  lleva `aria-current="step"`.
- **Inicio:**
  - las tarjetas se arman desde `flujo.inicio.tarjetas`, cada una como `<button>` con ícono, título,
    descripción, estado y enlace;
  - según la vigencia: `AlDia` usa la clase `ok` y el texto «Al día · {mes o fecha}»; `Atrasado` usa `viejo`
    y «Último mes: {mes}» (para la base: «Corte: {fecha}» ✎); `Nunca` usa `nunca` y «Aún no se ha cargado»;
  - el pie trae la carpeta visible y «Abrir carpeta», que llama a `abrir({tipo: "carpeta"})`;
  - el banner `flujo.aviso` aparece arriba si existe.
- **Archivos:**
  - zona de arrastre con `alSoltarArchivos` (se suscribe al montar y se desuscribe al desmontar) y «Buscar en
    mi computadora», que usa `elegirArchivos(reporte.varios_archivos)`;
  - lista de elegidos con su chip y «Quitar»;
  - el bloque de complemento aparece solo si el reporte tiene complementos;
  - pie con «Cancelar» y «Revisar archivos», con su `motivo` cuando está desactivado.
- Ver la sección «Textos» para los textos exactos.

- [ ] **Paso 1: revisión visual que falla.**
  - `npm --prefix app/ui run dev` en segundo plano.
  - Con la herramienta Playwright (MCP), abrir `http://localhost:5173/app.html?simulado`. Esperado: no está la
    pantalla Inicio con las tarjetas.
- [ ] **Paso 2: implementar los componentes y pantallas.**
- [ ] **Paso 3: verificar.**
  - `npm --prefix app/ui run check` y `npm --prefix app/ui test`. Esperado: pasan.
  - Con Playwright en `?simulado`:
    1. captura de Inicio a 1280×860;
    2. clic en «Planilla mensual»: captura de Archivos sin elegidos, con «Revisar archivos» desactivado y su
       motivo visible;
    3. en la consola, elegir archivos simulados vía `window.__flujo.agregar([...])` (expuesto solo en
       `?simulado`): captura con 3 elegidos.
  - Las capturas se guardan en el workspace del plan y se comparan a ojo contra los textos y colores de este
    plan.
  - Verificar con `browser_snapshot` que cada botón tiene nombre accesible y que la navegación con Tab
    llega a las tarjetas.
- [ ] **Paso 4:** registrar la tarea en el ledger, con las rutas de las capturas.

### Tarea 9: Pantalla Revisión (chip de celda, recorte de hoja y cómo arreglarlo)

**Archivos:**
- Crear: `app/ui/src/lib/componentes/ChipCelda.svelte`, `app/ui/src/lib/componentes/RecorteHoja.svelte`,
  `app/ui/src/app/pantallas/Revision.svelte` y `app/ui/src/app/pantallas/TarjetaError.svelte`.
- Crear: `app/ui/src/lib/recorte.ts` y `app/ui/src/lib/recorte.test.ts`.

**Interfaces:**
- `recorte.ts`:
  - `marcas(r: Recorte, esperada: string | null, encontrada: string | null): ("esperada" | "real" | null)[][]`;
  - la celda `esperada` se marca como `"esperada"`;
  - con `encontrada`, la **fila** completa de la celda encontrada (dentro del recorte) se marca como `"real"`;
  - `letraColumna(n: number): string` (1 da `"A"`, 27 da `"AA"`).
- `RecorteHoja.svelte`:
  - `<table>` con `aria-label` «Vista de la hoja {hoja}, filas {a} a {b}», cabeceras de columna (letras) y de
    fila (números) como `<th scope>`;
  - clases `esperado` (borde de 3 px `#A3261B` sobre `#FBE7E4`) y `real` (`#FFF8E6`);
  - la celda esperada vacía muestra «(vacía)»;
  - leyenda «Aquí deberían ir los títulos» / «Aquí están ahora», que solo aparece con `celda_encontrada`;
  - desplazamiento horizontal dentro de su caja.
- `Revision.svelte`:
  - una fila por archivo sin error (OK o aviso, con su texto);
  - una `TarjetaError` por archivo con error, que trae:
    - mes y nombre;
    - `problema.mensaje` en negrita;
    - el chip «Hoja «{hoja}» · celda {celda}» si hay celda;
    - el recorte si existe;
    - «Cómo arreglarlo» con `pasosArreglo`;
    - «Abrir en Excel» (`abrir({tipo: "archivo", ruta})`) y «Revisar de nuevo»;
  - si un archivo trae varios problemas, se muestran todos, con el primero destacado;
  - pie con «Atrás», «Seguir sin…» (solo si hay archivos con y sin error) y el botón principal con su motivo.

- [ ] **Paso 1: escribir el test que falla.** `recorte.test.ts`:
  - con un recorte de A5 a D8, esperada `A6` y encontrada `A7`: `marcas[1][0] === "esperada"`, toda la fila 2
    es `"real"` y el resto es `null`;
  - sin encontrada, ninguna celda es `"real"`;
  - `letraColumna(1) === "A"`, `letraColumna(26) === "Z"` y `letraColumna(27) === "AA"`.
- [ ] **Paso 2:** `npm --prefix app/ui test`.
  Esperado: FALLA.
- [ ] **Paso 3:** implementar.
- [ ] **Paso 4: verificar.**
  - `npm --prefix app/ui test` y `npm --prefix app/ui run check`. Esperado: pasan.
  - Con Playwright en `?simulado&error`: captura de Revisión con error. Verificar que «Procesar los 3 meses»
    está desactivado con su motivo, que «Seguir sin noviembre» está visible y que la tabla del recorte tiene
    su `aria-label`.
  - Con `?simulado`: captura de Revisión sin errores («Todo listo para procesar»).
- [ ] **Paso 5:** registrar la tarea en el ledger.

### Tarea 10: Pantallas Procesando y Listo

**Archivos:**
- Crear: `app/ui/src/app/pantallas/Procesando.svelte`, `app/ui/src/app/pantallas/Listo.svelte`,
  `app/ui/src/lib/componentes/BarraAvance.svelte` y `app/ui/src/app/pantallas/DetallesSoporte.svelte`.

**Interfaces:**
- **Procesando:**
  - tareas de `flujo.pasos` con `textoPaso`; los estados son hecha (check), ahora (anillo que gira; sin giro
    con `prefers-reduced-motion`) y pendiente;
  - `BarraAvance` con `role="progressbar"` y `aria-valuenow` igual a `round(indice / total * 100)`;
  - «{indice + 1} de {total} tareas»;
  - texto de ETA con `textoEta(tarjeta.duracion_prevista_ms, transcurrido)`, actualizado cada segundo; si da
    `null`, no se muestra;
  - «Detener» llama a `flujo.detener()`;
  - si `flujo.error` existe, en lugar de la lista se muestra:
    - el mensaje del error (para `archivo_abierto`, el mensaje del núcleo tal cual);
    - la lista de problemas con chip de celda, si hay;
    - «Volver a intentar» (que reprocesa) y «Volver al inicio».
- **DetallesSoporte:**
  - un `<details>` «Ver detalles para soporte» con marca, versión del motor, etapas (nombre técnico y estado)
    y problemas con `codigo`;
  - el texto se puede seleccionar;
  - aparece en Procesando (con error) y en Listo.
- **Listo:**
  - ícono de éxito, `tituloListo(reporte)` y «{titulo_periodos} · actualizada hoy a las {hh:mm}» (la hora sale
    de `ejecucion.marca`);
  - cifras de `resultado.cifras`, en 3 columnas que se apilan si la ventana es angosta;
  - «Para revisar»: el primer aviso, más «y {n} más» como `<details>` con la lista completa;
  - acciones: «Abrir en Excel», que es aquí el botón **principal** porque el tablero llega en el Plan 7
    (`abrir({tipo: "excel", reporte: id})`), y «Abrir carpeta»;
  - nota de la copia anterior;
  - «Volver al inicio» en el encabezado.

- [ ] **Paso 1: revisión visual que falla.** Con Playwright en `?simulado`, recorrer hasta procesar.
  Esperado: no existen las pantallas 4 y 5.
- [ ] **Paso 2:** implementar.
- [ ] **Paso 3: verificar.**
  - `npm --prefix app/ui test` y `npm --prefix app/ui run check`. Esperado: pasan.
  - Con Playwright en `?simulado` (con `demoraMs` de 800):
    1. captura de Procesando a mitad de camino: barra, «3 de 6 tareas» y tarea en curso;
    2. captura de Listo;
    3. clic en «Detener» durante otro proceso: vuelve a Inicio con el aviso «Se detuvo el proceso; no se cambió
       ningún reporte.».
- [ ] **Paso 4:** registrar la tarea en el ledger.

### Tarea 11: Verificación integral, empaquetado y documentación

**Archivos:**
- Modificar:
  - `AGENTS.md`: §2 marca `app/` como existente; §6 con los comandos de la app y el cambio de «Tests Rust» a
    `cargo test` (default-members) más el comando de la app;
  - `docs/superpowers/plans/2026-11-16-hoja-de-ruta.md`: Plan 6 **Completado**, con enlace;
  - `docs/diseno/README.md`: nota «Implementado (Plan 6)» bajo las pantallas 1–5, con los textos marcados ✎;
  - `config/README.md`;
  - `docs/logica-negocio/reglas-transversales.md`: nota en TR-D13 de que la UI muestra el recorte con
    `celda_encontrada`;
  - `docs/adr/0005-frontend-svelte-echarts.md`: solo en **Notas**, sin cambiar la decisión: los tipos de la
    app (Plan 6) están escritos a mano con ejemplos JSON de referencia hasta el mecanismo del Plan 7.

- [ ] **Paso 1: suite completa, en serie.**
  1. `cargo fmt --all --check`;
  2. `cargo test -j 2`;
  3. `npm --prefix app/ui run build`;
  4. `cargo test -p etl-local-app --lib -j 2`;
  5. `cargo clippy --workspace --all-targets -j 2 -- -D warnings`;
  6. `npm --prefix app/ui test`;
  7. `npm --prefix app/ui run check`.
  Esperado: todo en verde. Anotar el número de tests en el ledger.
- [ ] **Paso 2: prueba real con `tauri dev` sobre datos demo.**
  - Desde `app/`, con `ETL_LOCAL_SALIDA` apuntando a un temporal del scratchpad, ejecutar
    `npm --prefix ui run tauri -- dev` en segundo plano.
  - Esperado: abre la ventana sin errores en la consola de Rust y la consola web no tiene violaciones de CSP.
  - Pedir al usuario que recorra los siguientes casos (el agente no puede operar la ventana nativa):
    1. Planilla con las 3 planillas limpias de `crates/core/tests/fixtures/demo/limpio/planilla/` más las
       licencias;
    2. Base de personal;
    3. CASO-01, que muestra el recorte;
    4. «Abrir en Excel» y «Abrir carpeta».
  - Registrar su respuesta. Si no está disponible, ledgerar «prueba manual pendiente» y seguir.
- [ ] **Paso 3: chequeo de confidencialidad.**
  - Ejecutar el comando de AGENTS.md (nombres a ocultar de `paridad_legacy.entorno`, piped a `grep -f`) sobre
    `app/`, `config/reportes.yaml`, `crates/` y `docs/`, excluyendo `node_modules/` y `dist/`.
    Esperado: sin coincidencias.
  - Verificar que `app/src-tauri/tests/ejemplos/*.json` no tienen rutas reales.
- [ ] **Paso 4: empaquetado (release).** Último paso, en segundo plano y sin otra compilación en paralelo:
  `npm --prefix ui run tauri -- build` desde `app/`.
  - Esperado: un instalador en `target/release/bundle/` (NSIS o MSI).
  - Anotar el tamaño del instalador y el tiempo de compilación.
  - Instalar y abrir no forma parte de este plan.
- [ ] **Paso 5:** actualizar la documentación listada arriba y registrar la tarea en el ledger.
