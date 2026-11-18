# Plan 4 — Motor: capas Silver y Gold — plan de implementación

> **Para agentes:** SUB-SKILL REQUERIDA: usar superpowers:subagent-driven-development (recomendado) o
> superpowers:executing-plans para implementar este plan tarea por tarea. Los pasos usan casillas (`- [ ]`).

**Objetivo:** `etl-local procesar` toma las planillas, el control de licencias y la base de personal ya leídos por
el Plan 3, construye las capas Silver y Gold con SQL de DuckDB y deja Parquet con nombres canónicos en `actual/`,
con copia en `historico/`. El resultado cuadra con el manifiesto del demo y con los resultados del legacy (Plan 2),
salvo las correcciones decididas.

**Arquitectura:**
- `crates/core` agrega cuatro piezas, sin Tauri ni UI:
  - `parametros`: umbrales de negocio (TR-D10) leídos de `config/parametros.yaml`;
  - `almacen`: una base DuckDB en memoria; carga cada `TablaHoja` como tabla, ejecuta SQL y exporta Parquet;
  - `proceso`: orquesta lectura → Silver → Gold → controles → escritura atómica (TR-D6, TR-D7, TR-D16);
  - un ajuste a `lectura` para reconocer filas de totales (NOM-BRZ-10).
- La lógica de negocio va en SQL (AGENTS.md §4.4), en `config/sql/`: un archivo por paso y otro por control. Rust
  solo carga tablas, fija parámetros como variables de DuckDB (`SET VARIABLE`, `getvariable()`) y ejecuta los
  archivos en orden.
- `crates/cli` agrega el comando `procesar`.

**Stack:**
- Rust 1.99, `duckdb` 1.20000.2 con la feature `bundled` (DuckDB compilado dentro del binario, sin instalar nada).
- Lo demás ya está en el workspace (Plan 3).
- En `demo/` (Python): un argumento `--meses` nuevo en `generar-golden`.

**Spec:**
- [nomina.md](../../logica-negocio/nomina.md) §2.1 (NOM-BRZ-10), §3 (NOM-SLV), §4 (NOM-GLD), §5 (NOM-ENR);
- [licencias.md](../../logica-negocio/licencias.md) §3 (LIC-SLV), LIC-ENR;
- [bd.md](../../logica-negocio/bd.md) §3 (BD-SLV), §4 (BD-GLD), §5 (BD-CC, BD-FLG);
- [reglas-transversales.md §5](../../logica-negocio/reglas-transversales.md):
  - TR-D3, TR-D4, TR-D5, TR-D6, TR-D7, TR-D8 y TR-D10;
  - decididas en esta sesión: TR-D15 (reescrita), TR-D16, TR-D21, TR-D22, TR-D23 y TR-D24;
- [indicadores-tablero.md](../../logica-negocio/indicadores-tablero.md) (columnas que necesita el Plan 5);
- [vocabulario.md](../../logica-negocio/vocabulario.md); [AGENTS.md](../../../AGENTS.md) §4.

## Decisiones del usuario (2026-11-16)

| Tema | Decisión | Regla |
|---|---|---|
| Alertas de jubilación | Por año: `año(nacimiento) = año(fecha_corte) − 65` (este año), `− 64` (próximo); ídem 70/69. No se calcula la edad exacta | TR-D15 (reescrita), TR-D3 |
| Montos de dinero | Fuera del contrato y del Gold; la paridad compara solo columnas comunes | TR-D22 |
| Duplicados en la planilla | Dos archivos del mismo periodo: **error**. Persona repetida en un periodo: **aviso** con las filas, y se conserva | TR-D23 |
| Modalidad de contrato | Sin modalidad = empleado, con aviso. Las modalidades se comparan normalizadas | TR-D24 |

## Decisiones de este plan (revisar)

1. **Filas de totales en el contrato.** El campo opcional nuevo `filas_resumen` va en la hoja: una lista de textos.
   Una fila cuya primera celda, sin `+`, `-`, `*` ni espacios al inicio, contiene alguno de esos textos (comparación
   normalizada) se descarta sin aviso (NOM-BRZ-10). Es un cambio **compatible** del formato de ADR-0003 (versión 1
   sigue válida): se agrega una nota al ADR. Con esto, la planilla demo vuelve a tener 0 avisos.
2. **Nombres canónicos en Silver y Gold** (AGENTS.md §4.8). Los nombres nuevos se agregan a `vocabulario.md`:
   - del periodo: `anio`, `mes`, `nombre_mes`;
   - de licencias: `tipo_licencia`, `motivo_licencia_con_goce`, `motivo_licencia_sin_goce`;
   - de antigüedad y edad: `tiempo_servicio_texto`, `cumple_65_este_anio`, `cumple_65_proximo_anio`,
     `cumple_70_este_anio`, `cumple_70_proximo_anio`;
   - alertas: `alerta_contrato_obra`, `alerta_contrato_incremento`, `alerta_contrato_plazo`, `alerta_jubilacion`;
   - trazabilidad: `archivo_origen`, `fila_origen`.
3. **Tipos:** `anio` y `mes` son `INTEGER` (el legacy usaba texto). `nombre_mes` va en **español** (`Enero`…
   `Diciembre`), porque el tablero es en español (el legacy: `January`…).
4. **Un solo Gold de planilla**, ya con las licencias. El legacy escribía dos idénticos (NOM-ENR-03/04).
5. **Subsidios dinámicos:**
   - `dias_subsidio_maternidad` existe siempre (nulo si el mes no trae la columna; NOM-SLV-04);
   - el resto de columnas dinámicas `dias_subsidio_*` se suma en `dias_subsidio_otros` (vocabulario §4); nulo si no
     hay ninguna;
   - no se genera el total del legacy (`DIAS SUBSIDIOS TOTAL`): el tablero lo excluye para no contar dos veces;
   - qué columnas se conservan y dónde va el resto se declara en `config/parametros.yaml`, no en Rust.
6. **Motivos de licencia:** `STRING_AGG(DISTINCT motivo, ' | ' ORDER BY motivo)` por `(periodo, numero_documento)`.
   El `DISTINCT` corrige la repetición de texto cuando hay filas duplicadas. Las licencias sin persona en la planilla
   generan un aviso con la cantidad (LIC-ENR-04).
7. **Base de personal:**
   - documento repetido: aviso, y en el Gold queda la primera fila (TR-D4);
   - centros de costo: se excluyen los nulos; deduplicación por la primera aparición y orden por texto (BD-CC-02/03);
   - antigüedad = días entre `fecha_entrada` y `COALESCE(fecha_fin_contrato, fecha_corte)`, dividido por 365,25,
     como en el legacy (BD-FLG-02/03); el generador cuenta con eso;
   - `tiempo_servicio_texto` mantiene el formato legacy `"<A> años, <M> meses"`, pero es **nulo** si no hay fecha
     de entrada o la antigüedad es negativa (corrige Q-15);
   - flags y alertas en un solo `SELECT` (TR-D4);
   - `alerta_contrato_plazo = obra OR incremento`; `alerta_jubilacion = cualquiera de los cuatro cumple_*`. Ambas
     son `false`, no nulas, cuando falta el dato.
8. **Fecha de corte:** la del nombre del archivo de la base de personal (TR-D3). Si no hay base de personal, no se
   calculan flags (etapa `omitida`).
9. **Salida** en `<salida>/actual/` y `<salida>/historico/<marca>/`:
   - todo se arma en `<salida>/.en-curso-<marca>/` y solo se promueve si todas las etapas requeridas terminaron;
   - `<salida>` lleva el marcador `.etl-local`: el motor no borra una carpeta sin él (lección de los Planes 1 y 2);
   - la marca de tiempo la pone el CLI; el núcleo la recibe como parámetro (AGENTS.md §4.5).
10. **Paridad con el legacy:** `generar-golden` recibe `--meses` y los resultados del legacy se regeneran con 3
    meses, igual que los fixtures de Rust. Hoy son de 6 meses y no se pueden comparar.

## Restricciones globales

- **Sin git** (decisión del usuario): no hay pasos de commit. Cada tarea termina con:
  - `cargo test --workspace`;
  - `cargo clippy --workspace --all-targets -- -D warnings`;
  - `cargo fmt --all --check`;
  - en las tareas que tocan `demo/`, además `uv run --project demo pytest demo/tests -q` y
    `uv run --project demo ruff check demo`.
- **Idioma y nombres** (AGENTS.md §5 y §4.8): español; identificadores ASCII; nombres canónicos de
  `vocabulario.md` en tablas, columnas y Parquet.
- **Reglas:** cada regla de negocio cita su ID en un comentario del SQL y en el nombre del test que la cubre
  (AGENTS.md §4.1).
- **Determinismo:** ninguna regla usa el reloj. La fecha de referencia es `fecha_corte` y la marca de la ejecución
  es un parámetro.
- **Confidencialidad:** sin datos reales ni nombre del cliente.
- **Legacy:** el legacy solo se ejecuta desde `demo/` sobre datos sintéticos (Tarea 2).
- **Fallar explícito** (AGENTS.md §4.6): ninguna etapa reporta éxito si falló; un error de DuckDB detiene el proceso
  con el nombre del archivo SQL.

## Revisión: casos que ningún test de tarea cubre por sí solo

1. **Solo planillas, sin base de personal ni licencias** (lo primero que alguien prueba): el proceso termina, deja
   el Gold de planilla con los motivos en nulo y marca las etapas de base y licencias como `omitida`. → test en la
   Tarea 6.
2. **Carpeta de salida dentro de una carpeta sincronizada o con un archivo abierto** (Windows bloquea el renombre):
   un error claro que nombra la ruta, y `actual/` anterior intacto. → test en la Tarea 6 con un `actual/` de solo
   lectura.
3. **Planillas de meses con columnas distintas** (la columna dinámica aparece en un mes y no en otro): la unión no
   falla y los meses sin la columna quedan en nulo. → test en la Tarea 4.
4. **Persona de la planilla sin registro en la base de personal** (IND-C1, 8 casos en el demo): la fila de planilla
   no se pierde. El Gold de planilla no hace join con la base: ese cruce es del Plan 5. → test en la Tarea 4.
5. **Segunda ejecución con otra marca:** `actual/` refleja la última ejecución y `historico/` conserva las dos. →
   test en la Tarea 6.

---

### Tarea 1: filas de totales en el contrato (NOM-BRZ-10)

**Archivos:**
- Modificar:
  - `crates/core/src/contrato.rs` (`Hoja.filas_resumen: Vec<String>`, `#[serde(default)]`);
  - `crates/core/src/contrato/validar.rs` (V13: ningún texto vacío);
  - `crates/core/src/lectura.rs`.
- Modificar: `config/fuentes/planilla.yaml`, `config/esquema-contrato.json` (regenerado).
- Modificar: `docs/adr/0003-formato-contrato-fuente.md` (nota de 2026-11-16) y `validacion-preflight.md` §9.
- Tests: `crates/core/tests/lectura.rs`, `crates/core/tests/contrato.rs`.

**Interfaces:**
- Produce: `Hoja::filas_resumen`. En `lectura`, la función privada
  `es_fila_resumen(libro, hoja_nombre, fila, columna_inicio, textos: &[String]) -> bool`.

Regla:
- Se toma la primera celda de la fila, en la columna de inicio de la cabecera.
- Si es texto, se le quitan los caracteres iniciales `+`, `-`, `*` y los espacios, y se normaliza con
  `normalizar_cabecera`.
- Si contiene la normalización de algún texto de `filas_resumen`, la fila se descarta **antes** de mirar la clave, y
  sin aviso.
- Una primera celda que no es texto nunca es fila de totales (nomina.md §9 Q8).

`planilla.yaml` → `filas_resumen: [ingresos del mes, descuentos del mes, aportaciones del mes, total ingresos,
total descuentos, total aportaciones, subtotal, totales]`.

- [ ] **Paso 1: tests que fallan.**
  - `limpio_cuadra_con_manifiesto` vuelve a exigir **0 avisos** (sin la excepción de las dos filas de totales).
  - `filas_de_totales_se_descartan_sin_aviso` (`rust_xlsxwriter`, contrato mínimo con
    `filas_resumen: [total ingresos]`): filas `1/Ana`, `+ TOTAL INGRESOS` en A (sin documento) y `* Total ingresos`
    en A con documento `9`. Resultado: 1 fila y 0 avisos.
  - `numero_en_primera_celda_no_es_total`: la celda A con el número `5` y sin documento sigue dando PRE-24.
  - `filas_resumen_vacio_es_invalido` (contrato): `filas_resumen: ['']` da `Invalido`.
- [ ] **Paso 2:** `cargo test -p etl-local-core`. Esperado: FAIL en esos tests.
- [ ] **Paso 3:**
  - implementar y regenerar el esquema con
    `ACTUALIZAR_ESQUEMA=1 cargo test -p etl-local-core --test contratos_config esquema_contrato_al_dia`;
  - documentar en ADR-0003 (nota) y en §9 de `validacion-preflight.md`.
- [ ] **Paso 4:** suite, clippy y fmt en verde.

### Tarea 2: resultados del legacy con 3 meses

**Archivos:**
- Modificar:
  - `demo/src/paridad_legacy/cli.py` (`--meses`, entero de 1 a 24; por defecto 6);
  - `demo/src/paridad_legacy/golden.py` (sin cambios de lógica: ya recibe `ParametrosGeneracion`);
  - `demo/README.md` (comando nuevo; hallazgos vigentes).
- Modificar: `demo/tests/test_paridad_fallos.py` (test nuevo).
- Regenerar: `demo/golden/fixtures/limpio/` y `demo/golden/fixtures/sucio/`.

**Interfaces:**
- Produce: `uv run --project demo generar-golden --modo limpio --meses 3`. La Tarea 7 lee
  `demo/golden/fixtures/{limpio,sucio}/*.parquet`, generados con la misma semilla, `periodo_fin` y 3 meses que
  `crates/core/tests/fixtures/demo/`.

- [ ] **Paso 1: test que falla.** `test_cli_pasa_meses` en `test_paridad_fallos.py`: con `golden.generar_golden`
  reemplazado por un doble que captura `p`, `cli.main(["--legacy", …, "--salida", …, "--meses", "3"])` llama con
  `p.meses == 3`; y `--meses 0` sale con código 2 (error de argparse).
- [ ] **Paso 2:** `uv run --project demo pytest demo/tests/test_paridad_fallos.py -q`. Esperado: FAIL.
- [ ] **Paso 3:** implementar. Regenerar con `--meses 3` en `limpio` y en `sucio`; `casos` no cambia.
- [ ] **Paso 4:** verificar que en `resumen.json` de `limpio`, `filas_planilla_por_periodo` coincide y que hay 3
  periodos (2025-09…2025-11). Actualizar la tabla de hallazgos del README si cambió algún número.
- [ ] **Paso 5:** pytest y ruff en verde.

### Tarea 3: parámetros y almacén DuckDB

**Archivos:**
- Modificar: `Cargo.toml` (workspace: `duckdb = { version = "1.20000.2", features = ["bundled"] }`),
  `crates/core/Cargo.toml`.
- Crear: `config/parametros.yaml`, `crates/core/src/parametros.rs`, `crates/core/src/almacen.rs`.
- Crear: `crates/core/tests/almacen.rs`.
- Modificar: `crates/core/src/lib.rs`, `crates/core/src/error.rs` (`ErrorProceso`).

**Interfaces:**
- Consume: `TablaHoja`, `ColumnaLeida`, `Valor` y `TipoDato` (Plan 3).
- Produce:

```rust
// parametros.rs — config/parametros.yaml (TR-D10), deny_unknown_fields
pub struct Parametros {
    pub version: u32,                                    // 1
    pub jubilacion: Jubilacion,                          // { edad: 65, edad_extendida: 70 }
    pub contrato_obra: ReglaContrato,                    // { modalidad, anios_alerta: 4.5 }
    pub contrato_incremento: ReglaContrato,              // { modalidad, anios_alerta: 2.5 }
    pub practicante: Practicante,                        // { modalidad_contiene: "TERMINO DE CONVENIO" }
    pub subsidios: Subsidios,                            // { prefijo: "dias_subsidio_",
                                                         //   conservar: ["dias_subsidio_maternidad"],
                                                         //   resto: "dias_subsidio_otros",
                                                         //   fijas: ["dias_subsidio_enfermedad"] }
}
pub fn cargar_parametros(ruta: &Path) -> Result<Parametros, ErrorContrato>;  // reutiliza Yaml/Invalido

// almacen.rs
pub struct Almacen { /* duckdb::Connection */ }
impl Almacen {
    pub fn nuevo() -> Result<Almacen, ErrorProceso>;                       // en memoria
    /// Crea `nombre` con las columnas de la tabla (tipo DuckDB según TipoDato) más
    /// `constantes` (misma columna y valor en todas las filas) y `archivo_origen`/`fila_origen`.
    pub fn cargar(&self, nombre: &str, tabla: &TablaHoja, archivo: &str,
                  constantes: &[(&str, Valor)]) -> Result<(), ErrorProceso>;
    /// `CREATE TABLE destino AS SELECT * FROM a UNION ALL BY NAME SELECT * FROM b …`
    pub fn unir(&self, destino: &str, origenes: &[String]) -> Result<(), ErrorProceso>;
    pub fn fijar_variables(&self, p: &Parametros, fecha_corte: Option<NaiveDate>) -> Result<(), ErrorProceso>;
    pub fn ejecutar_archivo(&self, ruta: &Path) -> Result<(), ErrorProceso>;   // error nombra el archivo
    pub fn ejecutar(&self, sql: &str) -> Result<(), ErrorProceso>;
    pub fn columnas(&self, tabla: &str) -> Result<Vec<String>, ErrorProceso>;
    pub fn contar(&self, tabla: &str) -> Result<u64, ErrorProceso>;
    pub fn exportar_parquet(&self, tabla: &str, ruta: &Path) -> Result<u64, ErrorProceso>; // filas
    pub fn consultar_texto(&self, sql: &str) -> Result<Vec<Vec<Option<String>>>, ErrorProceso>; // controles y tests
}

#[derive(thiserror::Error, Debug)]
pub enum ErrorProceso {
    #[error(transparent)] Contrato(#[from] ErrorContrato),
    #[error("Hay archivos con errores; no se procesó nada.")] Entradas(Vec<Informe>),
    #[error("{0}")] Datos(String),                      // reglas de proceso (p. ej. periodo repetido)
    #[error("Error de DuckDB en {donde}: {detalle}")] Duckdb { donde: String, detalle: String },
    #[error("No se pudo escribir en {}: {fuente}", ruta.display())] Salida { ruta: PathBuf, fuente: std::io::Error },
    #[error("{} no es una carpeta de salida de etl-local (falta .etl-local); no se toca.", .0.display())]
    CarpetaAjena(PathBuf),
}
```

- **Mapeo de tipos:**

| `TipoDato` | Tipo DuckDB |
|---|---|
| `texto`, `documento`, `periodo` | `VARCHAR` |
| `entero` | `BIGINT` |
| `decimal` | `DOUBLE` |
| `fecha` | `DATE` |
| `booleano` | `BOOLEAN` |

  Los valores se insertan con el `Appender` de DuckDB y `Valor::Nulo` pasa como `NULL`.
- **Variables de DuckDB:** `fecha_corte` (DATE o NULL), `edad_jubilacion`, `edad_jubilacion_extendida`,
  `modalidad_obra`, `anios_alerta_obra`, `modalidad_incremento`, `anios_alerta_incremento` y
  `practicante_contiene`. Las modalidades se guardan ya normalizadas con `normalizar_cabecera`.
- **Macro SQL** creada por `nuevo()`: `normalizar(x) = upper(trim(regexp_replace(strip_accents(x), '\s+', ' ', 'g')))`.
  Es la misma regla que `normalizar_cabecera` (TR-D1, TR-D24).

- [ ] **Paso 1: tests que fallan** en `crates/core/tests/almacen.rs`:
  - `carga_tipos_y_exporta_parquet`: una `TablaHoja` hecha a mano con los 6 tipos y un nulo en cada uno.
    `exportar_parquet` devuelve 2, y releer con `read_parquet` da los mismos valores (fecha como `DATE`).
  - `unir_por_nombre_con_columnas_distintas`: `t1(a, b)` y `t2(a, c)` dan la unión `(a, b, c)` con 2 filas y
    nulos donde falta la columna (caso de revisión 3).
  - `variables_y_normalizar`: después de `fijar_variables` con la fecha 2025-11-12,
    `SELECT getvariable('fecha_corte'), getvariable('edad_jubilacion'), normalizar(' contrato  por obra ')` da
    `2025-11-12`, `65` y `CONTRATO POR OBRA`.
  - `error_sql_nombra_el_archivo`: `ejecutar_archivo` sobre un `.sql` con `SELEC 1` da `ErrorProceso::Duckdb` y
    `donde` contiene el nombre del archivo.
  - `parametros_cargan_y_validan`: `config/parametros.yaml` carga, y una clave mal escrita da error.
- [ ] **Paso 2:** `cargo test -p etl-local-core --test almacen`. Esperado: no compila. La primera compilación de
  DuckDB con `bundled` tarda varios minutos (ver "Hechos verificados").
- [ ] **Paso 3:** implementar.
- [ ] **Paso 4:** suite, clippy y fmt en verde.

### Tarea 4: Silver y Gold de planilla y licencias

**Archivos:**
- Crear: `config/sql/planilla/10_silver.sql`, `config/sql/licencias/10_silver.sql`, `config/sql/planilla/20_gold.sql`
- Crear: `config/sql/controles/planilla_persona_repetida.sql`, `config/sql/controles/licencias_sin_planilla.sql`
- Crear: `crates/core/src/proceso.rs` (solo las funciones de esta tarea) y `crates/core/tests/proceso_planilla.rs`

**Interfaces:**
- Consume: `Almacen` y `Parametros` (Tarea 3); `leer_archivo` y `Lectura` (Plan 3).
- Produce (en `proceso.rs`):

```rust
pub(crate) fn cargar_planillas(a: &Almacen, lecturas: &[Lectura], p: &Parametros) -> Result<Vec<String>, ErrorProceso>;
    // devuelve los periodos; error ErrorProceso::Datos si dos archivos traen el mismo periodo (TR-D23)
pub(crate) fn cargar_licencias(a: &Almacen, lectura: Option<&Lectura>) -> Result<(), ErrorProceso>;
pub(crate) fn controles(a: &Almacen, carpeta: &Path) -> Result<Vec<Problema>, ErrorProceso>;
```

Tablas que quedan en DuckDB:
- **`planilla_lectura`:** unión por nombre de las planillas. Cada archivo se carga con la constante `periodo`, que
  sale de `FechaNombre::Periodo`. Rust agrega `dias_subsidio_maternidad` si falta y calcula `dias_subsidio_otros`:
  la suma, con `COALESCE` a 0, de las columnas que empiezan por `subsidios.prefijo` y no están en `conservar` ni en
  `fijas`; nulo si no hay ninguna. Luego borra esas columnas sumadas.
- **`licencias_lectura`:** unión de las dos hojas, con `tipo_licencia` constante (`CON_GOCE` / `SIN_GOCE` según la
  hoja; LIC-SLV-05). Las dos hojas tienen las mismas columnas.

SQL, con el ID de regla en un comentario al inicio de cada bloque:
- **`planilla/10_silver.sql`:** crea `planilla_silver` (NOM-SLV-03/06/08/09) con
  `periodo, anio, mes, <columnas del contrato en orden>, dias_subsidio_maternidad, dias_subsidio_otros,
  archivo_origen, fila_origen`, donde `anio = CAST(substr(periodo,1,4) AS INTEGER)` y
  `mes = CAST(substr(periodo,6,2) AS INTEGER)`.
- **`licencias/10_silver.sql`:** crea `licencias_silver` con
  `periodo, numero_documento, apellido_paterno, apellido_materno, nombres, motivo_licencia, tipo_licencia,
  archivo_origen, fila_origen`. Excluye las filas con `motivo_licencia`, `numero_documento` o `periodo` nulos o
  vacíos tras `trim` (LIC-SLV-04/06/09).
- **`planilla/20_gold.sql`:** crea `planilla_gold`:
  - `periodo, anio, mes, nombre_mes`, donde `nombre_mes` sale de `CASE mes WHEN 1 THEN 'Enero' … END`
    (NOM-GLD-05, en español);
  - luego las columnas de Silver sin `archivo_origen`/`fila_origen`;
  - al final `motivo_licencia_con_goce` y `motivo_licencia_sin_goce`, con `LEFT JOIN` a dos CTE agregadas sobre
    `licencias_silver` (NOM-ENR-01/02 con `DISTINCT`).

  Si no hay licencias, `licencias_silver` existe vacía y los motivos quedan nulos.
- **Controles:** cada uno devuelve filas `(codigo, severidad, hoja, celda, mensaje)`:
  - `planilla_persona_repetida`: `NOM-GLD-15`, `aviso`; mensaje «{documento} aparece {n} veces en {periodo}
    (filas {lista})»;
  - `licencias_sin_planilla`: `LIC-ENR-04`, `aviso`; mensaje «{n} licencias no tienen a la persona en la planilla
    del mismo periodo», en una sola fila si `n > 0`.

  `controles()` ejecuta todos los `.sql` de la carpeta en orden alfabético y los convierte en `Problema`. El
  `archivo` del problema es el de la primera fila implicada, o `""`.

- [ ] **Paso 1: tests que fallan** en `crates/core/tests/proceso_planilla.rs`. Usan una función de prueba pública
  `etl_local_core::proceso::prueba::planilla_gold(config, planillas, licencias) -> Result<Almacen, ErrorProceso>`,
  detrás de `#[doc(hidden)]`, que corre lectura + carga + SQL de esta tarea.
  - `gold_cuadra_con_manifiesto`: para cada periodo de `limpio`:
    - `count(*)` es igual a `personas_planilla`;
    - las sumas de `dias_licencia_con_goce`, `dias_descanso_medico`, `dias_subsidio_maternidad` y
      `horas_extra_25` son iguales a las del manifiesto;
    - `dias_subsidio_otros` es nulo en todas las filas (el demo no trae otras columnas dinámicas).
  - `columnas_del_gold`: la lista de columnas de `planilla_gold` es exactamente la esperada, en orden: 4 de periodo,
    22 del contrato, 2 de subsidios y 2 de motivos.
  - `meses_con_columnas_distintas`: 2025-09 no trae la columna de maternidad y 2025-11 sí. La unión funciona y las
    filas de 2025-09 tienen nulo (caso de revisión 3).
  - `nombre_mes_en_espanol`: periodo `2025-11` da `nombre_mes = 'Noviembre'`, `anio = 2025` y `mes = 11`.
  - `motivos_de_licencia`:
    - la cantidad de filas con `motivo_licencia_con_goce` no nulo es igual a la de pares `(periodo, documento)`
      distintos de la hoja con goce que están en la planilla;
    - `licencias_silver` tiene tantas filas como las dos hojas leídas.
  - `periodo_repetido_es_error`: dos copias de la planilla 2025-11 (en un tempdir, con nombres distintos que
    cumplen el patrón) dan `ErrorProceso::Datos`, cuyo texto contiene `2025-11` y los dos nombres.
  - `persona_repetida_avisa`: una planilla mínima con el mismo documento en dos filas da un aviso `NOM-GLD-15`
    que menciona las dos filas, y `planilla_gold` conserva las dos.
  - `sin_base_de_personal_no_se_pierden_filas` (caso de revisión 4): el `count(*)` de `planilla_gold` del último
    periodo incluye a las 8 personas sin base; es igual a `personas_planilla`.
- [ ] **Paso 2:** `cargo test -p etl-local-core --test proceso_planilla`. Esperado: no compila.
- [ ] **Paso 3:** implementar los SQL y `proceso.rs`.
- [ ] **Paso 4:** suite, clippy y fmt en verde.

### Tarea 5: Silver y Gold de la base de personal

**Archivos:**
- Crear:
  - `config/sql/base_personal/10_silver.sql`;
  - `config/sql/base_personal/20_centros_costo.sql`;
  - `config/sql/base_personal/30_gold.sql`.
- Crear:
  - `config/sql/controles/base_personal_documento_repetido.sql`;
  - `config/sql/controles/base_personal_sin_modalidad.sql`.
- Modificar: `crates/core/src/proceso.rs` (`cargar_base_personal`).
- Crear: `crates/core/tests/proceso_base_personal.rs`.

**Interfaces:**
- Consume: `Almacen::cargar` y `fijar_variables` (Tarea 3); `controles()` (Tarea 4).
- Produce: `pub(crate) fn cargar_base_personal(a: &Almacen, lectura: &Lectura) -> Result<NaiveDate, ErrorProceso>`.
  Carga `base_personal_lectura` con la constante `fecha_corte` (de `FechaNombre::FechaCorte`) y devuelve esa fecha.
  Su prueba pública es `proceso::prueba::base_personal(config, archivo) -> Result<Almacen, ErrorProceso>`.

SQL:
- **`10_silver.sql`:** crea `base_personal_silver` con las 16 columnas del contrato, `fecha_corte` (DATE),
  `archivo_origen` y `fila_origen` (BD-SLV-01/08).
- **`20_centros_costo.sql`:** crea `centros_costo(centro_costo, nombre_centro_costo, categoria_centro_costo, gerencia)`.
  Toma la primera fila por `centro_costo` según `fila_origen`, excluye los nulos y ordena por `centro_costo` como
  texto (BD-CC-01…03).
- **`30_gold.sql`:**
  - **`base_personal_unica`:** la primera fila por `numero_documento` según `fila_origen` (TR-D4).
  - **`base_personal_practicantes_gold`:** las filas cuya modalidad normalizada contiene
    `getvariable('practicante_contiene')` (BD-GLD-06, TR-D24). Columnas: las 13 de BD-GLD-01 en nombres canónicos
    + `fecha_corte`.
  - **`base_personal_empleados_gold`:** el resto, incluidas las que no tienen modalidad (TR-D24). Mismas columnas
    + flags calculados en **un solo** `SELECT` (TR-D4):
    - `anios_servicio = date_diff('day', fecha_entrada, COALESCE(fecha_fin_contrato, fecha_corte)) / 365.25`
      (BD-FLG-02/03; columna intermedia en una CTE, no se exporta);
    - `tiempo_servicio_texto = floor(anios) || ' años, ' || floor((anios - floor(anios)) * 12) || ' meses'`, o
      nulo si `anios` es nulo o negativo (BD-FLG-04, corrige Q-15);
    - `cumple_65_este_anio = year(fecha_nacimiento) = year(fecha_corte) - edad_jubilacion`; `cumple_65_proximo_anio`
      con `- edad_jubilacion + 1`; ídem 70 con `edad_jubilacion_extendida` (BD-FLG-05…07, TR-D15, TR-D3). Nulo si
      no hay fecha de nacimiento;
    - `alerta_contrato_obra = normalizar(modalidad_contrato) = modalidad_obra AND anios >= anios_alerta_obra`, y
      `alerta_contrato_incremento` igual con su variable (BD-FLG-08…10, TR-D24);
    - `alerta_contrato_plazo = COALESCE(obra, false) OR COALESCE(incremento, false)`;
    - `alerta_jubilacion = COALESCE(cumple_65_este_anio OR cumple_65_proximo_anio OR cumple_70_este_anio OR
      cumple_70_proximo_anio, false)`.
- **Controles:**
  - `base_personal_documento_repetido`: `BD-SLV-07`, `aviso`; dice el documento y las filas.
  - `base_personal_sin_modalidad`: `BD-GLD-07`, `aviso`; «{n} personas sin modalidad de contrato; se contaron como
    empleados (filas …)».

- [ ] **Paso 1: tests que fallan** en `crates/core/tests/proceso_base_personal.rs`, con el manifiesto de `limpio`:
  - `split_empleados_practicantes`: `count(empleados) == empleados` y `count(practicantes) == practicantes`.
  - `alertas_de_contrato`:
    - `count_if(alerta_contrato_obra) == 11` y `count_if(alerta_contrato_incremento) == 7` (parámetros del
      generador, `demo/README.md`);
    - `count_if(alerta_contrato_plazo) == casos_contrato_plazo` (18).
  - `alertas_de_jubilacion`:
    - `count_if(cumple_65_este_anio OR cumple_65_proximo_anio) == 9`;
    - `count_if(cumple_70_este_anio OR cumple_70_proximo_anio) == 4`;
    - `count_if(alerta_jubilacion) == casos_jubilacion` (13).
  - `sucio_igual_que_limpio`: con `sucio`, los mismos conteos que con `limpio`. Con fechas escritas como texto
    (SUC-03), el legacy daba 10 alertas de obra; aquí 11.
  - `tiempo_servicio_texto_formato`: el texto cumple `^\d+ años, \d{1,2} meses$` en toda fila con `fecha_entrada`.
    Con un libro mínimo: entrada 2023-03-01, fin nulo y corte 2025-11-12 dan `"2 años, 8 meses"`.
  - `centros_costo_unicos`: `count(*) == count(DISTINCT centro_costo)`, ninguno nulo, y su cantidad es igual a
    `count(DISTINCT centro_costo)` del Silver sin nulos.
  - Con un libro mínimo (`rust_xlsxwriter`, contrato de `base_personal` y nombre `BD. 12.11.2025..xlsx`):
    - `sin_modalidad_es_empleado_con_aviso`: una fila sin modalidad cuenta en empleados y da un aviso `BD-GLD-07`;
    - `modalidad_con_espacios_y_minusculas`: `' contrato por obra o servicio especifico '` con 5 años de antigüedad
      da `alerta_contrato_obra = true`;
    - `documento_repetido_avisa_y_queda_el_primero`: dos filas con el mismo documento dan un aviso `BD-SLV-07` y
      una sola fila en el Gold, la primera.
- [ ] **Paso 2:** `cargo test -p etl-local-core --test proceso_base_personal`. Esperado: no compila.
- [ ] **Paso 3:** implementar.
- [ ] **Paso 4:** suite, clippy y fmt en verde.

### Tarea 6: proceso completo, salida atómica y comando `procesar`

**Archivos:**
- Modificar: `crates/core/src/proceso.rs` (API pública).
- Modificar: `crates/cli/src/main.rs`; crear `crates/cli/src/procesar.rs`.
- Crear: `crates/core/tests/proceso.rs`; modificar `crates/cli/tests/cli.rs`.

**Interfaces:**
- Consume: todo lo de las Tareas 3–5.
- Produce:

```rust
pub struct Entradas { pub planillas: Vec<PathBuf>, pub licencias: Option<PathBuf>, pub base_personal: Option<PathBuf> }
#[derive(Serialize)] #[serde(rename_all = "snake_case")]
pub enum EstadoEtapa { Ok, Aviso, Fallo, Omitida }
#[derive(Serialize)] pub struct Etapa { pub nombre: String, pub estado: EstadoEtapa, pub filas: Option<u64> }
#[derive(Serialize)] pub struct Fuente { pub fuente: String, pub archivo: String, pub fecha_nombre: Option<FechaNombre> }
#[derive(Serialize)] pub struct Ejecucion {
    pub marca: String, pub version_motor: String, pub fecha_corte: Option<NaiveDate>,
    pub periodos: Vec<String>, pub fuentes: Vec<Fuente>, pub etapas: Vec<Etapa>,
    pub salidas: Vec<(String, u64)>,      // ("gold/planilla.parquet", filas)
    pub problemas: Vec<Problema>,         // avisos de lectura + controles
}
/// `config`: carpeta con `fuentes/`, `sql/` y `parametros.yaml`.
pub fn procesar(config: &Path, entradas: &Entradas, salida: &Path, marca: &str) -> Result<Ejecucion, ErrorProceso>;
```

**Flujo:**
1. Cargar contratos y parámetros. Si no hay ninguna entrada, es `ErrorProceso::Datos`.
2. Leer todos los archivos con `leer_archivo`. Si alguno trae errores, termina con `ErrorProceso::Entradas`, con
   todos los informes, sin escribir nada.
3. Cargar en DuckDB y ejecutar, en orden, los SQL de `planilla`, `licencias` y `base_personal` con las etapas
   presentes; las demás quedan `Omitida`.
4. Ejecutar los controles.
5. Exportar a `<salida>/.en-curso-<marca>/`:
   - `silver/planilla.parquet`, `silver/licencias.parquet`, `silver/base_personal.parquet`;
   - `gold/planilla.parquet`, `gold/base_personal_empleados.parquet`, `gold/base_personal_practicantes.parquet`,
     `gold/centros_costo.parquet`;
   - `ejecucion.json` (TR-D16).
6. Promover:
   - renombrar `.en-curso-<marca>` → `historico/<marca>`;
   - copiar a `.actual-nuevo`;
   - renombrar el `actual` vigente a `.actual-viejo`, `.actual-nuevo` a `actual`, y borrar `.actual-viejo`.

   Si falla un renombre, se restaura el `actual` anterior y se devuelve `ErrorProceso::Salida` con la ruta.

**Reglas de la carpeta de salida:**
- `<salida>` se crea con el marcador `.etl-local` si no existe.
- Si existe y no está vacía y no tiene el marcador, es `ErrorProceso::CarpetaAjena`.
- Una `marca` repetida en `historico/` es `ErrorProceso::Datos`.

**CLI:**

```
etl-local procesar [--config config] --salida <CARPETA>
                   [--planilla <ARCHIVOS>...] [--licencias <ARCHIVO>] [--base-personal <ARCHIVO>] [--json]
```

- La marca es la fecha y hora local `AAAAMMDD-HHMMSS`, con `chrono::Local`; es el único uso del reloj, y fuera del
  núcleo.
- La salida humana muestra:
  - las etapas, con su estado y sus filas;
  - los avisos, con el mismo formato de `validar`;
  - «Resultados en {salida}/actual».
- Códigos de salida: 0 ok o con avisos; 1 si hay archivos con errores (se imprimen como en `validar`); 2 si falla
  el contrato, la configuración, la salida o DuckDB.
- `chrono` pasa a ser dependencia directa de `cli` (ya está en el workspace).

- [ ] **Paso 1: tests que fallan.** En `crates/core/tests/proceso.rs` (con `tempfile` y los fixtures de `limpio`):
  - `proceso_completo_limpio`:
    - existen los 7 parquet y `ejecucion.json`;
    - `etapas` incluye `planilla`, `licencias`, `base_personal` y `controles` en `Ok` o `Aviso`;
    - `fecha_corte == 2025-11-12` y `periodos` son los 3 del manifiesto;
    - `historico/m1/` es igual a `actual/`.
  - `solo_planillas` (caso de revisión 1): sin licencias ni base de personal, termina. Las etapas `licencias` y
    `base_personal` quedan `Omitida`, existe `gold/planilla.parquet` y no existe `gold/base_personal_empleados.parquet`.
  - `segunda_ejecucion_reemplaza_actual` (caso de revisión 5): con las marcas `m1` y `m2`, `actual/ejecucion.json`
    dice `m2` y existen `historico/m1` y `historico/m2`.
  - `archivo_con_error_no_escribe`: con CASO-03 como planilla, el resultado es `ErrorProceso::Entradas`. No existe
    `actual/` y la carpeta queda sin `.en-curso-*`.
  - `carpeta_ajena_no_se_toca`: un tempdir con `mio.txt` y sin marcador da `CarpetaAjena`, y `mio.txt` sigue ahí.
  - `actual_bloqueado_conserva_el_anterior` (caso de revisión 2, solo `#[cfg(windows)]`):
    - después de una ejecución `m1`, se abre `actual/gold/planilla.parquet` y se mantiene abierto con
      `std::fs::File::open`. En Windows eso impide renombrar la carpeta;
    - la ejecución `m2` da `ErrorProceso::Salida`, cuyo mensaje contiene `actual`;
    - `actual/ejecucion.json` sigue diciendo `m1`.

  En `crates/cli/tests/cli.rs`:
  - `procesar_limpio_sale_0`: con las 3 planillas, licencias y base de `limpio`, el código es 0, stdout contiene
    `Resultados en` y existe `actual/gold/planilla.parquet`.
  - `procesar_con_caso_roto_sale_1`.
- [ ] **Paso 2:** `cargo test --workspace`. Esperado: FAIL en los tests nuevos.
- [ ] **Paso 3:** implementar.
- [ ] **Paso 4:** suite, clippy y fmt en verde.

### Tarea 7: paridad con el legacy y documentación

**Archivos:**
- Crear: `crates/core/tests/paridad.rs`.
- Modificar:
  - `docs/logica-negocio/vocabulario.md` (nombres nuevos, decisión 2);
  - `docs/logica-negocio/nomina.md`, `bd.md` y `licencias.md` (una sección final "Rework (Plan 4)" con las reglas
    corregidas y su decisión: preservar, corregir o descartar; TR-D11);
  - `AGENTS.md` §6 (comando `procesar`);
  - la hoja de ruta (Plan 4 → Completado).

**Interfaces:**
- Consume: `procesar` (Tarea 6) y los resultados del legacy regenerados en la Tarea 2.

Mapeo de columnas legacy → canónico:
- Se define en el test como una tabla `const`.
- Viene de los `origen` de los contratos y de las decisiones de este plan; por ejemplo `"DNI/CEX" →
  "numero_documento"`, `"MOTIVO_CON_GOCE" → "motivo_licencia_con_goce"`, `"cumple_65_esteaño" →
  "cumple_65_este_anio"`.
- Se compara con DuckDB (`read_parquet` de ambos lados).

- [ ] **Paso 1: tests** en `crates/core/tests/paridad.rs`. Los resultados del legacy están en
  `demo/golden/fixtures/<modo>/`, y el test se omite con un mensaje si no existen. Se ejecuta `procesar` una vez por
  modo:
  - `planilla_igual_que_legacy` (`limpio` y `sucio`):
    - el mismo conjunto de `(periodo, numero_documento)`;
    - para cada columna común de días y horas, la misma suma por periodo;
    - los motivos de licencia son iguales por `(periodo, documento)`, comparando el legacy después de quitar
      repetidos del texto (decisión 6).
  - `licencias_igual_que_legacy`: el mismo multiconjunto de `(periodo, documento, motivo, tipo)`.
  - `base_personal_igual_que_legacy`:
    - las mismas cantidades de empleados y practicantes;
    - el mismo conjunto de documentos en cada grupo;
    - los mismos centros de costo.
  - `alertas_de_contrato_vs_legacy`:
    - en `limpio`, `alerta_contrato_obra` y `alerta_contrato_incremento` son iguales por documento;
    - en `sucio` difiere exactamente 1 documento de obra, por la fecha escrita como texto (corrección BD-DIF-03);
      el test lo deja explícito.
  - `tiempo_servicio_vs_legacy`: es igual en las filas con `fecha_fin_contrato` no nula, que no dependen del día
    de ejecución. El resto no se compara (`dependencias_de_fecha` de `resumen.json`).
  - `jubilacion_difiere_por_fecha_de_referencia`:
    - los conteos del motor son 9 y 4, los del manifiesto;
    - los del legacy son distintos y coinciden con aplicar la misma fórmula con `year(fecha_ejecucion)` de
      `resumen.json`, lo que demuestra que la única diferencia es la fecha (TR-D15, TR-D3).
- [ ] **Paso 2:** `cargo test -p etl-local-core --test paridad`. Si algún test falla, la causa se documenta:
  - si es una corrección decidida, se ajusta el test con una cita a la regla;
  - si es un error del motor, se usa systematic-debugging.
- [ ] **Paso 3:** actualizar los documentos (lista de archivos arriba).
- [ ] **Paso 4: verificación final.**
  - `cargo test --workspace`, clippy, fmt;
  - pytest y ruff de `demo/`;
  - el nombre del cliente sin coincidencias, con el procedimiento del Plan 3, Tarea 7, paso 5;
  - `cargo run -p etl-local-cli -- procesar --salida <tmp> --planilla …limpio… --licencias … --base-personal …`
    da código 0.

## Hechos verificados (2026-11-16)

Comprobado con un proyecto descartable en el scratchpad:
- **Versión:** `duckdb` 1.20000.2 con `bundled` compila en Windows (MSVC) y trae **DuckDB v2.0.0**.
- **Tiempo de compilación:** la primera vez tardó **~10 minutos** (compila DuckDB en C++); después solo se
  recompila el código propio. Conviene excluir `target/` del antivirus (ver la respuesta sobre AVG).
- **Appender y Parquet:** el `Appender` inserta filas con `NULL`, `COPY … TO '….parquet' (FORMAT parquet)` escribe
  el archivo, y `read_parquet` lo relee conservando los nulos.
- **SQL comprobado:**
  - `SET VARIABLE` / `getvariable()` con `DATE` y enteros;
  - `CREATE MACRO normalizar(x)` con `strip_accents` + `regexp_replace('\s+')` (`' contrato  por obra ó '` →
    `CONTRATO POR OBRA O`);
  - `UNION ALL BY NAME` con columnas distintas;
  - `string_agg(DISTINCT x, ' | ' ORDER BY x)`.
- **SQL desde Rust:** el SQL embebido en código necesita cadenas crudas (`r"…"`) por las barras invertidas. En este
  plan el SQL vive en archivos `.sql`, así que solo afecta a los tests.
- **Resultados del legacy:** son de 6 meses (`ParametrosGeneracion` por defecto, `fecha_ejecucion` 2026-11-16) y los
  fixtures de Rust son de 3: por eso existe la Tarea 2.
- **Modalidades del generador** (`demo/src/generador_demo/poblacion.py`):
  - `CONTRATO POR OBRA O SERVICIO ESPECIFICO`;
  - `CONTRATO POR INCREMENTO DE ACTIVIDAD`;
  - `PLAZO INDETERMINADO`;
  - `TERMINO DE CONVENIO`.
- **Casos de jubilación del generador:** se plantan por año de nacimiento (`año_corte − 65`, `− 64`, `− 70`,
  `− 69`), lo que confirma la decisión TR-D15.
