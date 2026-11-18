# Plan 8: Dominios restantes — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Sumar a la app los reportes Planilla régimen especial, Altas de personal, Control de practicantes y Exámenes médicos de salida con un molde común (contrato + SQL + Excel), sin paridad con el legacy y sin tocar el tablero.

**Architecture:** `proceso::procesar_con` gana un camino genérico: toda fuente de `Entradas.otras` se lee con su contrato, se carga en `<fuente>_lectura`, pasa por `config/sql/<fuente>/*.sql` (que deja `<fuente>_gold`) y se exporta a `gold/<fuente>.parquet` y `excel/<fuente>.xlsx`. Los cuatro dominios son solo configuración (contrato, SQL, parámetros, `reportes.yaml`). El generador `demo/` produce sus Excel y su manifiesto.

**Tech Stack:** Rust (crates/core, DuckDB), Python + uv (`demo/`), Tauri 2 + Svelte 5 (solo ajustes genéricos).

**Spec:** [docs/superpowers/specs/2026-11-17-plan-08-dominios-design.md](../specs/2026-11-17-plan-08-dominios-design.md)

## Global Constraints

- Nunca escribir el nombre del cliente; nunca copiar datos reales; `HR_Data_Integration_Pipeline/` es de solo lectura.
- Todo en español con identificadores ASCII.
- Sin commits, push ni PR sin pedido explícito (el repo no tiene git: no hay pasos de commit).
- Sin dependencias nuevas (Rust, Python ni npm).
- Formato de contrato (ADR-0003) sin cambios; `parametros.yaml` solo crece con secciones nuevas.
- Compilación: `-j 2` (o `-j 1` con menos de ~4 GB libres), nunca `cargo clean`, nunca dos `cargo` a la vez, sin release en este plan.
- Ningún Gold lleva montos de dinero (TR-D22).
- Meses en español en todos los Gold (`nombre_mes`: Enero…Diciembre).
- Chequeo de confidencialidad al final de cada tarea (comando de AGENTS.md con `grep -f`).

## Review Focus

1. Un reporte nuevo que trae su archivo con errores no debe borrar ni reemplazar `actual/` ni las entradas guardadas de los demás reportes (Task 2: `otra_fuente_con_errores_no_publica_nada`).
2. Guardar exámenes de salida (sin fecha en el nombre) debe mostrarse en Inicio como cargado, no como «Aún no se ha cargado» (Task 5: `reporte_sin_fecha_en_nombre_cargado_esta_al_dia`).
3. «Datos usados» del tablero no debe listar las fuentes nuevas con su id crudo (Task 6: `fuentes_del_tablero_ignora_otras`).
4. Procesar solo la planilla, cuando ya hay practicantes guardados, debe seguir publicando el Gold de practicantes (Task 2: `actualizar_conserva_otras_guardadas`).
5. Dos archivos del mismo mes de una fuente con periodo (altas, régimen especial) deben detener el proceso con un mensaje, como la planilla (Task 4: `dos_altas_del_mismo_mes_detienen`).

---

### Task 1: Generador demo de los cuatro dominios

**Files:**
- Create: `demo/src/generador_demo/excel_regimen_especial.py`, `excel_altas.py`, `excel_practicantes.py`, `excel_examenes.py`
- Modify: `demo/src/generador_demo/cli.py` (función `generar`), `manifiesto.py`, `casos.py`, `poblacion.py` (grupo de régimen especial), `demo/README.md`
- Test: `demo/tests/test_excel_dominios.py`, `demo/tests/test_sucio_casos.py`

**Interfaces:**
- Produces (layout demo que los contratos de las Tareas 2–5 describen; rutas relativas a la carpeta del modo):
  - `planilla_regimen_especial/Planilla Regimen Especial AAAA-MM.xlsx`: una por periodo. Hoja `Planilla`, cabecera en A6, datos desde la fila 7. Columnas: `CÓDIGO CORPORATIVO, DNI/CEX, SEXO, F. NACIMIENTO, PUESTO, SEDE, ÁREA, CENTRO DE COSTOS, DESC. CENTRO DE COSTO, FECHA DE INGRESO, DIAS TRABAJADOS, DIAS DESC MEDICO (P), DIAS GOCE HABER(D), DIAS DIST.LIC.S.GH, DIAS LIC. PATE (P), # HRS EXTRAS SIMP.25%, # HRS EXTRAS SIMP.35%, #HRS EXTRAS DOBLES, TOTAL INGRESOS, TOTAL DESCUENTOS, TOTAL APORTACIONES, NETO`. Al final lleva una fila de totales con `TOTALES` en la columna A y sin DNI. El grupo propio tiene 40 personas (`Poblacion.regimen_especial`, que no aparece en `en_planilla`); todas trabajan todos los periodos.
  - `altas_personal/Altas de personal AAAA-MM.xlsx`: una por periodo. Hojas `EMPLEADOS` y `PRACTICANTES`, cabecera en A1.
    - EMPLEADOS: `DNI, APELLIDOS Y NOMBRES, SEXO, FECHA NACIMIENTO, CARGO, AREA, SEDE, FECHA INICIO, TIPO DE CONTRATO, PROYECTO, CODIGO SAP`.
    - PRACTICANTES: `Numero Documento, Apellidos y Nombres, Sexo, Fecha Nacimiento, Cargo, Area, Sede, Fecha Inicio, Universidad de Procedencia`.
    - Filas: las personas con `fecha_entrada` dentro del mes. Empleados son los no practicantes; practicantes, los practicantes.
  - `practicantes/BD Practicantes DD.MM.AAAA.xlsx`: un archivo, con la fecha de corte de la base. Hoja `Practicantes`, cabecera en A4. Columnas: `N°, DNI, APELLIDOS Y NOMBRES, CONDICION, FECHA ING, F. RENOVACION, SEDE, UNIVERSIDAD, JEFE INMEDIATO, GERENCIA`.
    - Filas: los 60 practicantes de la base.
    - `CONDICION`: `PRACTICANTE PROFESIONAL` (50) o `PRACTICANTE PRE PROFESIONAL` (10).
    - `F. RENOVACION` se fija para que los profesionales caigan, en este orden: 6 en 335–364 días, 8 en 365–639, 5 en 640–729 y 2 sin renovación; el resto, por debajo de 335.
  - `examenes_salida/Examenes de salida.xlsx`: un archivo. Hoja `DATA`, cabecera en A3. Columnas: `NOMBRE, DNI, FECHA DE CESE, CAUSA DE SALIDA, CARGO, NOMBRE DE CC`.
    - Filas: todas las personas con `fecha_salida` dentro de los periodos generados, más 3 practicantes con `CARGO = STUDENT INTERNAL`.
    - `NOMBRE DE CC` lleva el código de centro de costo de la persona, salvo en 2 filas vacías y 2 con el código `CC-999`.
  - Manifiesto, claves nuevas:
    - `planilla_regimen_especial: {periodo: filas}`
    - `altas_personal: {periodo: {empleados, practicantes}}`
    - `practicantes: {filas, por_cumplir_1, cumplio_1, por_cumplir_2, sin_renovacion}`; los conteos son solo de profesionales; `cumplio_1` incluye la ventana de `por_cumplir_2`.
    - `examenes_salida: {filas, practicantes_excluidos, ok, sin_codigo, codigo_no_encontrado}`; `ok` = código presente en los centros de costo de `en_base_personal()`.
  - Casos:
    - CASO-07: régimen especial sin la columna `DNI/CEX` (PRE-11, hoja `Planilla`, `fila 6`).
    - CASO-08: altas sin la hoja `PRACTICANTES` (PRE-09).
    - CASO-09: practicantes con el nombre `Practicantes.xlsx` (PRE-07).
    - CASO-10: exámenes sin la columna `FECHA DE CESE` (PRE-11, hoja `DATA`, `fila 3`).

- [ ] **Step 1: Write the failing tests** in `demo/tests/test_excel_dominios.py`:
  - `test_regimen_especial_un_archivo_por_periodo_con_totales`: con `--meses 3`, son 3 archivos; la hoja `Planilla` tiene en A6 el texto `CÓDIGO CORPORATIVO` y la última fila tiene `TOTALES` en A; las filas con DNI suman `manifiesto["planilla_regimen_especial"][periodo]`.
  - `test_altas_cuadran_con_entradas_del_mes`: las filas de EMPLEADOS de cada periodo = `manifiesto["altas_personal"][p]["empleados"]` = `manifiesto["periodos"][p]["entradas"]`.
  - `test_practicantes_ventanas`: `manifiesto["practicantes"] == {"filas": 60, "por_cumplir_1": 6, "cumplio_1": 13, "por_cumplir_2": 5, "sin_renovacion": 2}`.
  - `test_examenes_estados_del_cruce`: `sin_codigo == 2`, `codigo_no_encontrado == 2`, `practicantes_excluidos == 3`, `ok + sin_codigo + codigo_no_encontrado == filas`.
  - En `test_sucio_casos.py`, ampliar el test de casos: existen `CASO-07`…`CASO-10` en `casos.json` con su regla.

- [ ] **Step 2: Run** `uv run --project demo pytest demo/tests -q`. Expected: FAIL (módulos `excel_*` inexistentes).
- [ ] **Step 3: Implement** los escritores con `excel_comun.guardar_libro` (fechas con `FORMATO_FECHA`), el grupo `regimen_especial` en `simular_poblacion` (con la misma semilla: no cambia a las personas existentes), las claves del manifiesto y los casos. `generar` escribe los cuatro dominios en `limpio` y `sucio`; `aplicar_suciedad` no los toca.
- [ ] **Step 4: Run** `uv run --project demo pytest demo/tests -q`. Expected: PASS (todos, incluidos los de paridad que ya pasaban).
- [ ] **Step 5: Regenerar fixtures** con los tres comandos de `crates/core/tests/fixtures/README.md` (borrando antes `crates/core/tests/fixtures/demo/`), y correr `cargo test -p etl-local-core -j 2`. Expected: PASS sin cambios (los tests existentes no ven las carpetas nuevas). Documentar los dominios en `demo/README.md` (tabla de contenido generado y casos).

### Task 2: El molde genérico en el motor, con Control de practicantes

**Files:**
- Modify: `crates/core/src/proceso.rs` (`Entradas`, `Paso`, `pasos`, `procesar_con`), `crates/core/src/entradas.rs` (`guardadas`, `combinar`, `actualizar`), `crates/core/src/parametros.rs`, `crates/core/src/almacen.rs` (`fijar_variables`)
- Create: `config/fuentes/practicantes.yaml`, `config/sql/practicantes/10_gold.sql`, `config/sql/controles/practicantes_sin_renovacion.sql`
- Modify: `config/parametros.yaml`, `crates/core/tests/comun/mod.rs`
- Test: `crates/core/tests/proceso_otras.rs`, `crates/core/tests/entradas.rs`

**Interfaces:**
- Produces:
  - `Entradas { planillas, licencias, base_personal, otras: BTreeMap<String, Vec<PathBuf>> }`, con `#[serde(default)]` en `otras`.
  - `Paso::OtrosReportes`, serializado como `otros_reportes`. `pasos()` lo agrega antes de `Controles` cuando `otras` no está vacío.
  - En `ejecucion.json`, una etapa `<fuente>` por cada fuente de `otras`; también está la etapa `omitida` cuando hay archivos guardados pero no se usaron.
  - Salidas `gold/<fuente>.parquet` y `excel/<fuente>.xlsx`.
  - Contrato del molde (para las Tareas 3–5):
    - las hojas se cargan como `<fuente>_lectura`, unidas por nombre (`Almacen::unir`), con las constantes `hoja` (nombre de la hoja del contrato) y, según `fecha_nombre`, `periodo` (VARCHAR) o `fecha_corte` (DATE);
    - el SQL de `config/sql/<fuente>/` debe crear `<fuente>_gold`;
    - los controles se leen de `config/sql/controles/<fuente>_*.sql`.
  - `tests/comun`: `pub fn otro(modo: &str, fuente: &str) -> Vec<PathBuf>` (los `.xlsx` de `<modo>/<fuente>/`).
  - Parámetros:
    ```yaml
    control_practicantes:
      condicion: PRACTICANTE PROFESIONAL
      por_cumplir_1: [335, 364]
      cumplio_1: 365
      por_cumplir_2: [640, 729]
    ```
    Variables DuckDB: `cp_condicion`, `cp_pc1_desde`, `cp_pc1_hasta`, `cp_c1_desde`, `cp_pc2_desde`, `cp_pc2_hasta`.
  - `practicantes_gold`, columnas en este orden:
    - `numero_documento, condicion, fecha_entrada, fecha_renovacion, fecha_corte, sede, universidad, jefe_inmediato, gerencia`;
    - `dias_servicio, anios_servicio, meses_servicio, tiempo_servicio, por_cumplir_1, cumplio_1, por_cumplir_2`;
    - las alertas son BOOLEAN; un nulo significa «sin dato»;
    - las fórmulas son las de CP-FLG-01…08, con `D = fecha_renovacion − fecha_entrada`;
    - la condición se compara normalizada (mayúsculas, sin tildes ni espacios sobrantes);
    - con `D` nulo o negativo, todos los cálculos quedan nulos.
  - `tiempo_servicio`:
    - `1 año (En límite)` si `por_cumplir_1`; `2 años (Límite Máximo)` si `por_cumplir_2`;
    - si no, `N meses` / `N años` / `N años y M meses`, con singular `1 mes` / `1 año`.
  - Contrato `practicantes`:
    - patrón `(?i)BD Practicantes\s+(?P<fecha>\d{2}\.\d{2}\.\d{4})\.(xlsx|xlsm|xls)`, `fecha_en_nombre: { grupo: fecha, formato: DD.MM.AAAA }`;
    - hoja `Practicantes`, cabecera `A4`, clave `numero_documento` (`DNI`);
    - `N°` y `APELLIDOS Y NOMBRES` quedan fuera (CP §9.7: no se usan).
  - Control `PRA-01` (aviso): «N practicantes sin fecha de renovación o con renovación anterior al ingreso: no se calcula su tiempo de servicio.»

- [ ] **Step 1: Write the failing tests** in `crates/core/tests/proceso_otras.rs`:
  - `practicantes_cuadra_con_manifiesto`: procesa `Entradas { otras: {"practicantes": otro("limpio","practicantes")}, ..Default::default() }`. Lee `gold/practicantes.parquet` con DuckDB (como `proceso.rs` lee sus Parquet). Comprueba que filas = 60 y que contar `por_cumplir_1`, `cumplio_1` y `por_cumplir_2` en verdadero da 6, 13 y 5. Comprueba que `excel/practicantes.xlsx` existe y que hay un problema `PRA-01` con «2 practicantes» en el mensaje.
  - `tiempo_servicio_en_texto`: para filas con `dias_servicio` 30, 365, 400 y 340, el texto es `1 mes`, `1 año`, `1 año y 1 mes` y `1 año (En límite)`. Usar un Excel construido en el test con `rust_xlsxwriter` en un `tempdir`, con el layout del contrato.
  - `otra_fuente_con_errores_no_publica_nada`:
    - primero se procesa limpio, con planilla, base y practicantes;
    - luego se llama `actualizar` con `otras: {"practicantes": [CASO-09]}`, que debe devolver `ErrorProceso::Entradas`;
    - `actual/` y `entradas/` quedan idénticos (mismos nombres de archivo y `ejecucion.json` sin cambios).
  - `pasos_con_otras`: `pasos(&Entradas{ otras: {"x": [..]}, ..})` es `[Leer, OtrosReportes, Controles, Guardar]`.
- [ ] **Step 2: Write the failing tests** in `crates/core/tests/entradas.rs`:
  - `actualizar_conserva_otras_guardadas`: primero se procesan planilla y practicantes; luego se llama `actualizar` solo con planillas nuevas. `gold/practicantes.parquet` sigue en `actual/` y `entradas/practicantes/` conserva su archivo.
  - `otra_nueva_reemplaza_a_la_guardada`: con una fuente sin periodo en el nombre, el archivo nuevo reemplaza al guardado. `excluir: ["practicantes"]` la saca del proceso y no toca `entradas/practicantes/`.
- [ ] **Step 3: Run** `cargo test -p etl-local-core -j 2 --test proceso_otras --test entradas`. Expected: FAIL (no existe el campo `otras` ni `Paso::OtrosReportes`).
- [ ] **Step 4: Implement:**
  - En `procesar_con`, las fuentes de `otras` se leen con `leer_todos` junto con las demás, antes de cargar nada, así que un error en cualquier archivo no publica nada.
  - Después de la base de personal, por cada fuente se cargan sus hojas, se ejecuta `sql/<fuente>`, se registra la etapa y se exporta.
  - En `entradas.rs`, `guardadas` recorre las carpetas de `entradas/` que no son las tres fijas.
  - `combinar`: para una fuente cuyo contrato tiene `fecha_en_nombre` con formato `AAAA-MM`, combina por periodo con la misma lógica de la planilla (extraer `combinar_por_periodo(contrato, guardadas, nuevas) -> Result<Vec<PathBuf>, ErrorProceso>` y usarla también para la planilla). Para las demás fuentes, si hay archivos nuevos reemplazan a los guardados.
  - `actualizar` sincroniza `entradas/<fuente>` de cada fuente usada.
  - Los parámetros nuevos se agregan a `Parametros` (con `deny_unknown_fields`) y a `fijar_variables`.
- [ ] **Step 5: Run** `cargo test -p etl-local-core -j 2`. Expected: PASS (los tests nuevos y todos los anteriores).
- [ ] **Step 6: Regenerar los esquemas** que cambian con `Entradas`/`Paso`:
  - `ACTUALIZAR_ESQUEMA=1 cargo test -p etl-local-core -j 2 --test esquema_api`;
  - el test de esquema de la app (`app/src-tauri`, ver AGENTS.md);
  - `npm --prefix app/ui run tipos`.

  Después correr `npm --prefix app/ui run check`. Expected: el único error es el `switch` de `textoPaso` sin `otros_reportes`. Se arregla en la Tarea 6: anotarlo en el ledger y seguir.

### Task 3: Planilla régimen especial

**Files:**
- Create: `config/fuentes/planilla_regimen_especial.yaml`, `config/sql/planilla_regimen_especial/10_gold.sql`, `config/sql/controles/planilla_regimen_especial_persona_repetida.sql`
- Test: `crates/core/tests/proceso_otras.rs`, `crates/core/tests/preflight.rs`

**Interfaces:**
- Consumes: el molde de la Tarea 2 (`<fuente>_lectura` con `periodo`).
- Produces:
  - Contrato:
    - patrón `(?i).*Planilla Regimen Especial (?P<periodo>\d{4}-\d{2}).*\.(xlsx|xlsm|xls)`, periodo `AAAA-MM`;
    - hoja `Planilla`, cabecera `A6`, `filas_resumen: [totales, total ingresos, subtotal]`;
    - las columnas que comparte con `config/fuentes/planilla.yaml` llevan **sus mismos nombres canónicos y tipos**; la clave es `numero_documento`;
    - los montos (`TOTAL INGRESOS`, `TOTAL DESCUENTOS`, `TOTAL APORTACIONES`, `NETO`) no van en el contrato.
  - `planilla_regimen_especial_gold`:
    - `periodo, anio, mes, nombre_mes` y luego las columnas del contrato, ordenado por `periodo, archivo_origen, fila_origen`;
    - sin `archivo_origen`, `fila_origen` ni `hoja`.
  - Control `PRE-RE-01` (aviso): persona repetida en el mismo periodo (como `planilla_persona_repetida.sql`).
- [ ] **Step 1: Write the failing tests:**
  - en `proceso_otras.rs`, `regimen_especial_cuadra_y_sin_montos`: las filas por periodo = manifiesto; ninguna columna del Gold contiene `ingreso`, `descuento`, `aportacion` ni `neto`; `nombre_mes` del primer periodo es el nombre en español; ninguna fila tiene `numero_documento` nulo (la fila `TOTALES` se descartó);
  - en `preflight.rs`, agregar `("CASO-07", "planilla_regimen_especial", "PRE-11", Some("Planilla"), Some("fila 6"), "DNI/CEX")` a la tabla de `casos_reportan_su_error` (el arreglo pasa a `[CasoEsperado; 10]` cuando entren los cuatro casos);
  - un test que confirma que `Planilla 2025-11 Empleados.xlsx` no cumple el patrón de este contrato y que `Planilla Regimen Especial 2025-11.xlsx` no cumple el de `planilla`.
- [ ] **Step 2: Run** `cargo test -p etl-local-core -j 2 --test proceso_otras --test preflight`. Expected: FAIL (falta el contrato).
- [ ] **Step 3: Implement** el contrato, el SQL y el control.
- [ ] **Step 4: Run** `cargo test -p etl-local-core -j 2`. Expected: PASS.

### Task 4: Altas de personal

**Files:**
- Create: `config/fuentes/altas_personal.yaml`, `config/sql/altas_personal/10_gold.sql`
- Test: `crates/core/tests/proceso_otras.rs`, `crates/core/tests/entradas.rs`, `crates/core/tests/preflight.rs`

**Interfaces:**
- Consumes: el molde (`hoja` constante, unión por nombre).
- Produces:
  - Contrato:
    - patrón `(?i).*Altas de personal (?P<periodo>\d{4}-\d{2}).*\.(xlsx|xlsm|xls)`;
    - hojas `EMPLEADOS` y `PRACTICANTES` con cabecera `A1`; cada una mapea sus encabezados a los mismos nombres canónicos: `numero_documento` (clave), `nombre_completo`, `sexo`, `fecha_nacimiento`, `cargo`, `area`, `sede`, `fecha_entrada`;
    - solo en EMPLEADOS: `modalidad_contrato` (`TIPO DE CONTRATO`); solo en PRACTICANTES: `universidad` (`Universidad de Procedencia`, no requerida);
    - `PROYECTO` y `CODIGO SAP` no van en el contrato.
  - `altas_personal_gold`:
    - `periodo, anio, mes, nombre_mes, tipo` y luego las demás columnas;
    - `tipo` vale `empleado` o `practicante`, según `hoja`;
    - las columnas que una hoja no tiene quedan nulas.
- [ ] **Step 1: Write the failing tests:**
  - `altas_por_periodo_y_tipo` (`proceso_otras.rs`): el conteo por `(periodo, tipo)` = `manifiesto["altas_personal"]`;
  - `dos_altas_del_mismo_mes_detienen` (`entradas.rs`): `combinar` con dos archivos nuevos `Altas de personal 2025-11*.xlsx` (copias en un `tempdir`) devuelve `ErrorProceso::Datos` con «del mismo mes (2025-11)» en el mensaje;
  - CASO-08 en `preflight.rs`: `("CASO-08", "altas_personal", "PRE-09", Some("PRACTICANTES"), None, "PRACTICANTES")`.
- [ ] **Step 2: Run** `cargo test -p etl-local-core -j 2 --test proceso_otras --test entradas --test preflight`. Expected: FAIL.
- [ ] **Step 3: Implement** el contrato y el SQL. Si el mensaje de `combinar_por_periodo` dice «planillas», generalizarlo a «Hay dos archivos del mismo mes (…)» y ajustar el test de la planilla que lo cite.
- [ ] **Step 4: Run** `cargo test -p etl-local-core -j 2`. Expected: PASS.

### Task 5: Exámenes médicos de salida y vigencia sin fecha

**Files:**
- Create: `config/fuentes/examenes_salida.yaml`, `config/sql/examenes_salida/10_gold.sql`, `config/sql/controles/examenes_salida_sin_catalogo.sql`
- Modify: `config/parametros.yaml`, `crates/core/src/parametros.rs`, `crates/core/src/almacen.rs`, `crates/core/src/reportes.rs` (`estado_reportes`)
- Test: `crates/core/tests/proceso_otras.rs`, `crates/core/tests/reportes.rs`, `crates/core/tests/preflight.rs`

**Interfaces:**
- Consumes: la tabla `centros_costo` que deja `config/sql/base_personal/` (columnas `centro_costo, nombre_centro_costo, categoria_centro_costo, gerencia`; confirmarlo en `20_centros_costo.sql`). Antes de que corra el SQL, el molde garantiza que exista: si no se cargó la base, crea `centros_costo` vacía con esas columnas.
- Produces:
  - Parámetro `examenes_salida: { cargo_practicante: STUDENT INTERNAL }` y la variable `es_cargo_practicante`.
  - Contrato: sin patrón ni fecha en el nombre; hoja `DATA`, cabecera `A3`; `numero_documento` (`DNI`, clave), `nombre_completo` (`NOMBRE`), `fecha_salida` (`FECHA DE CESE`, fecha, requerida), `motivo_salida` (`CAUSA DE SALIDA`), `cargo`, `centro_costo` (`NOMBRE DE CC`, texto, no requerida).
  - `examenes_salida_gold`:
    - columnas `nombre_completo, numero_documento, fecha_salida, anio, mes, nombre_mes, motivo_salida, cargo, centro_costo, nombre_centro_costo, categoria_centro_costo, gerencia, estado_cruce`;
    - se excluyen las filas cuyo cargo normalizado es el parámetro;
    - el cruce compara `upper(trim(codigo))`;
    - `estado_cruce`:
      - `sin_catalogo` si `centros_costo` está vacía;
      - si no, `sin_codigo` cuando el código es nulo o vacío;
      - si no, `codigo_no_encontrado` o `ok`.
  - Control `EXS-01` (aviso), cuando hay filas `sin_catalogo`: «Cargue la base de personal para completar el centro de costo de los exámenes de salida.»
  - `estado_reportes`: si el contrato no tiene `fecha_en_nombre` y `entradas/<fuente>/` tiene archivos, la vigencia es `AlDia` con `ultimo: None`.
- [ ] **Step 1: Write the failing tests:**
  - `examenes_cruce_con_base` (`proceso_otras.rs`): con base y exámenes, los conteos por `estado_cruce` = `manifiesto["examenes_salida"]` y no queda ningún cargo `STUDENT INTERNAL`;
  - `examenes_sin_base_avisan`: sin base, todas las filas son `sin_catalogo` y hay un problema `EXS-01`;
  - `reporte_sin_fecha_en_nombre_cargado_esta_al_dia` (`reportes.rs`): en un `tempdir` con `entradas/examenes_salida/Examenes de salida.xlsx`, el estado de ese reporte es `AlDia` (el reporte se agrega en la Tarea 6, así que el test usa una copia de `config/` con una entrada `examenes_salida` en su `reportes.yaml`);
  - CASO-09 y CASO-10 en `preflight.rs`: `("CASO-09", "practicantes", "PRE-07", None, None, "nombre")` y `("CASO-10", "examenes_salida", "PRE-11", Some("DATA"), Some("fila 3"), "FECHA DE CESE")`.
- [ ] **Step 2: Run** `cargo test -p etl-local-core -j 2 --test proceso_otras --test reportes --test preflight`. Expected: FAIL.
- [ ] **Step 3: Implement** el contrato, el SQL, el control, el parámetro y la regla de vigencia.
- [ ] **Step 4: Run** `cargo test -p etl-local-core -j 2`. Expected: PASS.

### Task 6: App y tablero

**Files:**
- Modify: `config/reportes.yaml`, `app/src-tauri/src/comandos.rs` (`asignar`), `app/src-tauri/src/servicio.rs` (`resultado`), `app/ui/src/app/textos.ts` (`textoPaso`), `app/ui/src/app/simulado.ts`, `app/ui/src/tablero/textos.ts` (`fuentesUnicas`)
- Test: `app/src-tauri/src/servicio.rs` (módulo `tests`), `app/src-tauri/src/comandos.rs` (tests), `app/ui/src/app/textos.test.ts`, `app/ui/src/tablero/textos.test.ts`, `crates/core/tests/reportes.rs`

**Interfaces:**
- Consumes: `Entradas.otras`, `Paso::OtrosReportes` y las salidas `gold/<fuente>.parquet` (Tareas 2–5).
- Produces:
  - `reportes.yaml`:
    | id = fuente = excel | `titulo` | `descripcion` | `varios_archivos` | `genero` |
    |---|---|---|---|---|
    | `planilla_regimen_especial` | «Planilla régimen especial» | «Días y horas del personal con régimen especial» | sí | femenino |
    | `altas_personal` | «Altas de personal» | «Quién entró cada mes, empleados y practicantes» | sí | femenino |
    | `practicantes` | «Control de practicantes» | «Tiempo de servicio y alertas de 1 y 2 años» | no | masculino |
    | `examenes_salida` | «Exámenes médicos de salida» | «Ceses y su centro de costo» | no | masculino |

    Ninguno lleva complementos.
  - `asignar`: una fuente que no sea `planilla`, `licencias` ni `base_personal` va a `otras` (se acumula en su lista).
  - `resultado` para los reportes del molde:
    - cifras: `{ valor: filas de gold/<fuente>.parquet, etiqueta: "filas en el reporte" }` y la de avisos (la misma que la planilla);
    - título: `titulo_periodos(...)` con los periodos de las `fuentes` de esa fuente si tienen periodo; `Corte: …` si tienen fecha de corte; vacío si no tienen ninguna.
  - `textoPaso("otros_reportes")` = «Ordenar los demás reportes».
  - `fuentesUnicas` solo devuelve `planilla`, `licencias` y `base_personal` (las que usa el tablero).
- [ ] **Step 1: Write the failing tests:**
  - `cargar_reportes_trae_los_seis` (`crates/core/tests/reportes.rs`): los ids en orden;
  - `asignar_otras` (comandos): `asignar(&mut e, "practicantes", vec![a])` deja `e.otras["practicantes"] == [a]`;
  - `resultado_de_un_reporte_del_molde` (servicio): actualiza con practicantes del fixture limpio; las cifras empiezan con `valor: "60"`, etiqueta «filas en el reporte», y el título empieza con «Corte:»;
  - vitest `textoPaso("otros_reportes", …)` = «Ordenar los demás reportes»;
  - vitest `fuentes_del_tablero_ignora_otras`: `fuentesUnicas([planilla, examenes_salida])` devuelve solo la planilla.
- [ ] **Step 2: Run** `cargo test -p etl-local-core -j 2 --test reportes`, `cargo test -p etl-local-app -j 2` y `npm --prefix app/ui test`. Expected: FAIL.
- [ ] **Step 3: Implement.** `simulado.ts` agrega los cuatro reportes, para que el modo navegador los muestre.
- [ ] **Step 4: Run** de nuevo los tres comandos más `npm --prefix app/ui run check`. Expected: todo PASS, con 0 errores.
- [ ] **Step 5: Prueba manual** en `npm --prefix app/ui run tauri -- dev`:
  - procesar Control de practicantes (fixture limpio) y luego Exámenes de salida;
  - Inicio muestra los seis reportes, ambos «Al día»;
  - «Abrir en Excel» abre `excel/practicantes.xlsx`;
  - «Datos usados» del tablero no lista los nuevos.

  Capturas en el ledger.

### Task 7: Documentos y verificación final

**Files:**
- Modify:
  - `docs/logica-negocio/{nomina-regimen-minero,pdt-relacion-ingresos,control-practicantes,examen-retiro}.md`: sección «Decisiones del rework (Plan 8)»;
  - `docs/logica-negocio/reglas-transversales.md` §5: TR-D9 queda como decidido, reporte aparte y sin indicadores;
  - `config/README.md`: el molde, con cómo agregar un reporte;
  - `AGENTS.md` (estructura);
  - `docs/superpowers/plans/2026-11-16-hoja-de-ruta.md`: link y estado del Plan 8.

- [ ] **Step 1:** Escribir en cada documento de dominio qué preguntas de su §9 se resolvieron y cómo, citando el ID de la pregunta. Contenido: §5.2 y §7 del spec. `config/README.md` explica en 3 pasos cómo agregar un reporte con el molde.
- [ ] **Step 2: Run** toda la verificación:
  - `cargo test -j 2`;
  - `cargo clippy -j 2 --all-targets -- -D warnings`;
  - `cargo fmt --check`;
  - `uv run --project demo pytest demo/tests -q`;
  - `npm --prefix app/ui test`, `run check` y `run build`;
  - el chequeo de confidencialidad sobre `config/`, `docs/`, `demo/src`, `crates/` y `app/`.

  Expected: todo verde y el chequeo sin coincidencias.
- [ ] **Step 3:** Hoja de ruta: el Plan 8 pasa a **Completado** solo después de la revisión final del plan.
