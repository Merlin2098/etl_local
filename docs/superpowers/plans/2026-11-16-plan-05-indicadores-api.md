# Plan 5: indicadores y API local. Plan de implementación

> **Para agentes:** SUB-SKILL REQUERIDA: usar superpowers:subagent-driven-development (recomendado) o
> superpowers:executing-plans para implementar este plan tarea por tarea. Los pasos usan casillas (`- [ ]`).

**Objetivo:** `etl-local tablero --salida <carpeta>` levanta una API local de solo lectura en `127.0.0.1`. La API
entrega IND-01…08 e IND-C1…C3 calculados con SQL de DuckDB sobre `actual/`, con filtros, comparación con el mes
anterior, metas y origen de cada número. Los valores cuadran con el manifiesto del demo.

**Arquitectura:**
- **`crates/core`** suma el módulo `tablero`, con funciones síncronas y sin HTTP:
  - abre `actual/` y crea vistas sobre los Parquet Gold;
  - fija los filtros como tabla parametrizada;
  - ejecuta `config/sql/indicadores/*.sql`;
  - arma respuestas `serde` con los metadatos de `config/indicadores.yaml` y las metas de
    `config/parametros.yaml`.
  Las fórmulas viven solo en el SQL (ADR-0004 §2). Rust calcula la variación y la palabra "mejor/peor" y elige las
  notas. Esa es la única lógica de presentación.
- **`crates/api`** (nuevo) es una capa HTTP delgada con axum:
  - rutas `/api/v1/…` (ADR-0004 §3);
  - verificación del `Host`;
  - errores JSON;
  - cada pedido corre el núcleo en `spawn_blocking`.
- **`crates/cli`** suma el comando `tablero`.

**Stack:** axum 0.8.9 y tokio 1.54 (dependencias nuevas, ADR-0004). Para tests: tower 0.5.3 (`util`) y
http-body-util 0.1.5. Lo demás ya está en el workspace.

**Spec:**
- [ADR-0004](../../adr/0004-api-local-tablero.md).
- [indicadores-tablero.md](../../logica-negocio/indicadores-tablero.md): §2 IND-01…08, §3 motivos de ausencia,
  §4 comparaciones y metas, §5 IND-C1…C3.
- [reglas-transversales.md](../../logica-negocio/reglas-transversales.md): TR-D3, TR-D16, TR-D17, TR-D18.
- [bd.md](../../logica-negocio/bd.md) BD-FLG-05…10 (columnas de alerta del Gold).
- [AGENTS.md](../../../AGENTS.md) §4.

## Decisiones del usuario (2026-11-16, ADR-0004)

| Tema | Decisión |
|---|---|
| Estrategia | API local en Rust con DuckDB nativo; nada de DuckDB-WASM |
| Servidor | axum + tokio |
| Nombres de personas (TR-D18) | Solo en `/api/v1/detalle/…`; las demás rutas nunca los devuelven |
| Cálculo | En cada pedido, con los filtros; no se precalcula nada al procesar |

## Decisiones de este plan (revisar)

1. **Catálogo de indicadores** en `config/indicadores.yaml`. Por indicador: `id`, `nombre`, `pregunta`, `unidad`,
   `mejor_si`, `fuente`, `reglas` y `etiquetas` del desglose. Son metadatos, no fórmulas. Lo validan
   `deny_unknown_fields` y una regla: exactamente IND-01…IND-08, en orden.
2. **Metas** en una sección opcional `metas:` de `parametros.yaml` (mapa `IND-0N: número`).
   - Por defecto está vacía.
   - Una clave fuera de IND-01…08, o un número negativo o no finito, es un error al cargar.
3. **Periodo por defecto:** el último `periodo` de la planilla. Uno que no esté en la planilla es un filtro
   inválido.
4. **Filtros inválidos:**
   - un valor de `gerencia`, `sede` o `tipo_trabajador` que no figura en la base de personal (comparación exacta)
     es un error 400 que lista los valores válidos;
   - un parámetro desconocido (`gerencias=`) también es 400;
   - los valores llegan a DuckDB como parámetros preparados, nunca como texto dentro del SQL.
5. **Filtros de dimensión y personas sin base:** con algún filtro de gerencia, sede o tipo, las personas de la
   planilla sin registro en la base quedan fuera. IND-C1 lo explica (indicadores-tablero §5).
6. **Porcentajes como fracción:** `0.0073` es 0,73 %. La UI formatea. `unidad` es `personas`, `fraccion` u `horas`.
7. **IND-C2** (periodos mezclados) se calcula en Rust desde `ejecucion.json`. Hay aviso cuando el mes de
   `fecha_corte` de la base de personal (`AAAA-MM`) difiere del último periodo de la planilla. Solo compara dos
   metadatos de la ejecución, sin datos Gold.
8. **Sin mes anterior en los datos:** IND-04 da `null` con nota, y los demás indicadores van sin `anterior` ni
   comparación. JSON nunca lleva `NaN` ni infinitos, porque todo divisor pasa por `nullif(…, 0)`.
9. **Faltan fuentes en `actual/`:** el Parquet que no exista se reemplaza por una tabla vacía con el esquema de
   `config/sql/indicadores/vacias/<tabla>.sql`.
   - Sin planilla: IND-01…06 dan `null` con la nota "No hay planilla procesada".
   - Sin base de personal: IND-07 e IND-08 dan `null` con la nota "No hay base de personal procesada".
10. **IND-05 e IND-06** suman todas las filas de la planilla del mes, incluida la persona repetida que TR-D23
    conserva con aviso. El divisor (IND-01) cuenta personas distintas.
11. **Puerto:** por defecto 8787. Con `--puerto 0` el sistema elige uno libre y el CLI imprime el real (sirve
    para los tests).
12. **`--config`:** `tablero` usa la carpeta `config`, igual que `procesar`. La diferencia con `validar` y `leer`
    sigue pendiente (Plan 4).

## Restricciones globales

- **Sin git:** no hay pasos de commit. Cada tarea termina con `cargo test --workspace`,
  `cargo clippy --workspace --all-targets -- -D warnings` y `cargo fmt --all --check`.
  - Si el antivirus corrompe artefactos, borrar los `*.rmeta`/`*.rlib` de tamaño 0 en `target/debug/deps` y
    reintentar con `-j 2`.
- **Idioma y nombres** (AGENTS.md §5 y §4.8): español, identificadores ASCII y nombres canónicos de
  `vocabulario.md` en columnas y campos JSON.
- **Reglas:** cada fórmula cita su ID (IND-xx, TR-Dxx) en un comentario del SQL y en el nombre del test que la cubre.
- **Determinismo:** ningún cálculo usa el reloj (TR-D3).
- **Privacidad (TR-D18):** solo `/detalle/…` devuelve nombres o números de documento. Un test lo verifica.
- **Confidencialidad:** sin datos reales ni nombre del cliente.
- **Solo lectura:** el tablero nunca escribe en `<salida>/`.

## Revisión: casos que ningún test de tarea cubre por sí solo

1. **Primer mes de los datos** (sin mes anterior), que es lo primero que alguien filtra: IND-04 es `null` con nota,
   no hay comparación y el JSON no lleva `NaN`. → test en la Tarea 2.
2. **Filtro mal escrito o malicioso:**
   - con `gerencia=operaciones` (minúsculas), `gerencia=x' OR '1'='1` o `gerencias=…`, la respuesta es 400 con los
     valores válidos;
   - nunca 200 con ceros, y nunca un error de SQL.
   → test en la Tarea 3 (núcleo) y en la Tarea 5 (HTTP).
3. **Combinación de filtros sin personas** (una gerencia sin nadie en esa sede): da 200, con conteos en 0, tasas en
   `null` y sin `NaN`. → test en la Tarea 3.
4. **Todavía no se procesó nada, o solo se procesaron planillas:**
   - sin `actual/`, la respuesta es 404 `sin_datos` con un mensaje que dice qué hacer;
   - con solo planillas, IND-07 e IND-08 son `null` con nota.
   → tests en las Tareas 3 y 5.
5. **Otra página o equipo intenta leer la API** (`Host: evil.example`), o el puerto está ocupado:
   - lo primero da 403 `host_no_permitido`;
   - lo segundo, salida 2 con un mensaje que sugiere `--puerto`.
   → tests en las Tareas 5 y 6.

---

### Tarea 1: piezas de soporte en el núcleo

**Archivos:**
- Modificar:
  - `crates/core/src/almacen.rs`;
  - `crates/core/src/parametros.rs`;
  - `crates/core/src/proceso.rs`, `crates/core/src/informe.rs` y `crates/core/src/preflight.rs` (agregar
    `Deserialize`).
- Modificar: `config/parametros.yaml` (sección `metas: {}` con comentario).
- Tests: `crates/core/tests/almacen.rs`, `crates/core/tests/proceso.rs`.

**Interfaces (produce):**
- `Almacen::vista_parquet(&self, tabla: &str, ruta: &Path) -> Result<(), ErrorProceso>`: crea
  `CREATE VIEW <tabla> AS SELECT * FROM read_parquet(<ruta>)`.
- `Almacen::fijar_filtros(&self, filtros: &[(&str, Option<&str>)]) -> Result<(), ErrorProceso>`.
  - Crea (o reemplaza) la tabla `filtros`, con una fila y una columna `VARCHAR` por nombre.
  - Los valores van como parámetros de una sentencia preparada.
  - Los nombres son constantes de Rust; se rechaza el que no sea `[a-z_]+`.
- `Almacen::consultar_json(&self, sql: &str) -> Result<Vec<serde_json::Map<String, serde_json::Value>>, ErrorProceso>`.
  - Conversión de tipos: entero → número, `DOUBLE` → número (un no finito → `null`), `BOOLEAN` → bool,
    `DATE` → `"AAAA-MM-DD"`, `VARCHAR` → texto, `NULL` → `null`.
- `Parametros.metas: BTreeMap<String, f64>`, con `#[serde(default)]`.
- `Deserialize` en `Ejecucion`, `Fuente`, `Etapa`, `EstadoEtapa`, `FechaNombre`, `Problema` y `Severidad`.

- [ ] **Paso 1: tests que fallan.**
  - `fijar_filtros_con_comillas`: `fijar_filtros(&[("gerencia", Some("x' OR '1'='1")), ("sede", None)])`. Luego
    `SELECT gerencia, sede FROM filtros` da `[Some("x' OR '1'='1"), None]`.
  - `fijar_filtros_rechaza_nombre_raro`: el nombre `"a;DROP"` da `Err`.
  - `consultar_json_tipos`: `SELECT 1::BIGINT a, 2.5::DOUBLE b, true c, DATE '2025-11-12' d, 'x' e, NULL f,
    'nan'::DOUBLE g` da `{"a":1,"b":2.5,"c":true,"d":"2025-11-12","e":"x","f":null,"g":null}`.
  - `vista_parquet_lee`: exporta `tabla_de_tipos()` a Parquet, crea la vista `v` y verifica que `contar("v") == 2`.
  - `metas_validas` (en `almacen.rs`, junto a los otros tests de parámetros):
    - sin sección da un mapa vacío;
    - `metas: {IND-04: 0.02}` carga;
    - `metas: {IND-09: 1}` y `metas: {IND-04: -1}` dan `Err`, con un mensaje que nombra la clave.
  - `ejecucion_json_se_relee` (en `proceso.rs`): procesa limpio, deserializa `actual/ejecucion.json` como
    `Ejecucion` y compara `periodos` y `fecha_corte` con los devueltos por `procesar`.
- [ ] **Paso 2:** `cargo test -p etl-local-core`. Esperado: FAIL de compilación en los tests nuevos.
- [ ] **Paso 3: implementar** con las firmas de arriba.
- [ ] **Paso 4:** suite, clippy y fmt en verde.

### Tarea 2: IND-01…06 por mes, contra el manifiesto

**Archivos:**
- Crear:
  - `config/indicadores.yaml` (catálogo, decisión 1);
  - `config/sql/indicadores/vacias/planilla.sql` y `config/sql/indicadores/vacias/base_personal_empleados.sql`;
  - `config/sql/indicadores/10_filtrado.sql` y `config/sql/indicadores/20_mensuales.sql`.
- Crear: `crates/core/src/tablero.rs` y `crates/core/src/tablero/respuesta.rs` (tipos `serde`).
- Modificar:
  - `crates/core/src/lib.rs` (`pub mod tablero`);
  - `crates/core/src/error.rs` (`ErrorTablero`).
- Test: `crates/core/tests/tablero.rs`. Procesa los fixtures `limpio` y `sucio` una vez cada uno, en directorios
  temporales `OnceLock` como en `paridad.rs`.

**Interfaces:**
- Consume: Tarea 1; `procesar` del Plan 4 (en los tests).
- Produce, en `etl_local_core::tablero`:
  - `Filtros { periodo, gerencia, sede, tipo_trabajador }`.
    - Los cuatro campos son `Option<String>`.
    - Deriva `Deserialize`, `Serialize`, `Default`, `Clone` y `Debug`, con `#[serde(deny_unknown_fields)]`.
  - `Tablero::nuevo(config: &Path, salida: &Path) -> Tablero`.
  - `Tablero::indicadores(&self, f: &Filtros) -> Result<RespuestaIndicadores, ErrorTablero>`. En esta tarea,
    IND-01…06.
  - `respuesta::{RespuestaIndicadores, Indicador, Parte, Origen, MejorSi, Comparacion}`:
    - `RespuestaIndicadores { periodo: Option<String>, periodo_anterior: Option<String>,
      fecha_corte: Option<NaiveDate>, filtros: Filtros, indicadores: Vec<Indicador> }`; `filtros` lleva el
      periodo ya resuelto.
    - `Indicador { id, nombre, pregunta, unidad, valor: Option<f64>, anterior: Option<f64>,
      variacion: Option<f64>, mejor_si: MejorSi, comparacion: Option<Comparacion>, meta: Option<f64>,
      desglose: Vec<Parte>, origen: Origen, nota: Option<String> }`.
    - `Parte { categoria: String, etiqueta: String, valor: f64 }`.
    - `Origen { fuente: String, archivos: Vec<String>, fecha_corte: Option<NaiveDate>, periodo: Option<String>,
      reglas: Vec<String> }`. Los archivos salen de `ejecucion.json` (TR-D17).
    - `MejorSi`: `baja`, `sube` o `neutro`. `Comparacion`: `mejor`, `peor` o `igual`. Se serializan en
      `snake_case`.
  - `ErrorTablero` (en `error.rs`):
    - `SinDatos(PathBuf)`;
    - `FiltroInvalido { filtro: String, valor: String, opciones: Vec<String> }`;
    - `Proceso(ErrorProceso)`;
    - `Contrato(ErrorContrato)`;
    - método `codigo(&self) -> &'static str`: `sin_datos`, `filtro_invalido` e `interno` (las dos últimas
      variantes).

**Flujo de `abrir` (privado, lo usan todos los métodos):**
1. Si falta `<salida>/actual/ejecucion.json` → `SinDatos`, con el mensaje "Todavía no hay resultados en {ruta}.
   Procese los archivos primero."
2. Crear el `Almacen`. Para `planilla` y `base_personal_empleados`:
   - si existe `actual/gold/<t>.parquet`, usar `vista_parquet`;
   - si no, ejecutar `vacias/<t>.sql`.
3. Resolver el periodo y validar los filtros (decisiones 3 y 4).
4. Llamar a `fijar_filtros` con `periodo`, `gerencia`, `sede` y `tipo_trabajador`.
5. Ejecutar en orden los `config/sql/indicadores/*.sql` del primer nivel.

**SQL (cada fórmula con su ID en un comentario):**
- `10_filtrado.sql`:
  - `SET VARIABLE f_periodo`, `f_gerencia`, `f_sede` y `f_tipo_trabajador` desde `filtros`;
  - vista `planilla_f`: la planilla con `LEFT JOIN` a la base por `numero_documento` (aporta `gerencia`, `sede` y
    `tipo_trabajador`), filtrada por las tres dimensiones; un filtro nulo deja pasar todo (decisión 5);
  - vista `empleados_f`: la base filtrada igual.
- `20_mensuales.sql`:
  - tabla `ind_valor(periodo VARCHAR, id VARCHAR, valor DOUBLE)` con IND-01…06 para **todos** los periodos;
  - tabla `ind_desglose(periodo VARCHAR, id VARCHAR, categoria VARCHAR, valor DOUBLE)`:
    - IND-03 por `motivo_salida`, con `'Sin motivo'` si es nulo, en personas;
    - IND-05 por las 7 columnas de §3, en días, con `coalesce(…, 0)`;
    - IND-06 por las 4 columnas de horas, en horas totales;
  - días calendario: `day(last_day(make_date(anio, mes, 1)))`;
  - mes anterior: `strftime(make_date(anio, mes, 1) - INTERVAL 1 MONTH, '%Y-%m')`, con `LEFT JOIN` a los periodos
    presentes;
  - IND-04 es `NULL` si falta el anterior.

- [ ] **Paso 1: tests que fallan** (`tablero.rs`; `m` es el manifiesto de limpio y `p` cada periodo).
  - `ind_01_a_03_cuadran_con_manifiesto`: en cada periodo (filtro `periodo = p`), IND-01 es `personas_planilla`,
    IND-02 es `entradas` e IND-03 es `salidas`. La suma del desglose de IND-03 es igual a IND-03.
  - `ind_04_rotacion`: en 2025-10, IND-04 es `9 / ((1224 + 1226) / 2)` con tolerancia `1e-12`.
  - `ind_05_ausentismo`:
    - cada parte del desglose es igual a `dias_ausencia[categoria]` del manifiesto, y `dias_subsidio_otros` es 0;
    - IND-05 es `Σ desglose / (IND-01 × días del mes)`, con 30, 31 y 30 días.
  - `ind_06_horas_extra`: el desglose es igual a `horas_extra` (tolerancia `1e-6`) e IND-06 es `Σ / IND-01`.
  - `primer_mes_sin_anterior` (Revisión 1), en 2025-09:
    - `periodo_anterior` es `None`;
    - IND-04: `valor` `None` y `nota` que contiene "mes anterior";
    - todos: `anterior`, `variacion` y `comparacion` en `None`;
    - `serde_json::to_string` no contiene `NaN`.
  - `comparacion_segun_sentido`: en 2025-11, `variacion = valor − anterior`. `comparacion` es coherente con
    `mejor_si` (`baja` y variación < 0 → `mejor`), y es `None` en IND-01 (neutro).
  - `periodo_por_defecto_es_el_ultimo`: sin filtros, `periodo == Some("2025-11")`.
  - `origen_cita_reglas_y_archivos`: IND-05 tiene `origen.fuente == "planilla"`, 3 archivos y `reglas` que contiene
    `"IND-05"`.
  - `sucio_igual_a_limpio`: valores y desgloses de IND-01…06 iguales en los 3 periodos.
  - `meta_desde_parametros`: copia `config/` a un temporal con `metas: {IND-04: 0.02}` y verifica que la `meta` de
    IND-04 es `Some(0.02)` y las demás `None`.
  - `catalogo_invalido`: un `indicadores.yaml` sin IND-08 da `Err(ErrorTablero::Contrato(_))`.
- [ ] **Paso 2:** `cargo test -p etl-local-core --test tablero`. Esperado: FAIL de compilación.
- [ ] **Paso 3: implementar.**
  - SQL, catálogo y módulo `tablero`.
  - Textos de `indicadores.yaml` copiados de indicadores-tablero §2 y §3:
    - `nombre` y `pregunta` de §2;
    - etiquetas de IND-05 de §3;
    - etiquetas de IND-06: "Horas extra al 25 %", "al 35 %", "al 50 %" y "al 100 %";
    - `mejor_si` de §4.
- [ ] **Paso 4:** suite, clippy y fmt en verde.

### Tarea 3: IND-07/08, filtros de dimensión y fuentes ausentes

**Archivos:**
- Crear: `config/sql/indicadores/30_alertas.sql`.
- Modificar: `crates/core/src/tablero.rs`, `config/indicadores.yaml`.
- Test: `crates/core/tests/tablero.rs`.

**Interfaces:**
- Consume: Tarea 2.
- Produce: IND-07 e IND-08 en `ind_valor` y `ind_desglose`, con `periodo NULL`. No responden al periodo
  (indicadores-tablero §2).
  - Desglose de IND-07: `alerta_contrato_obra` y `alerta_contrato_incremento`, con las reglas BD-FLG-08, BD-FLG-09
    y TR-D24.
  - Desglose de IND-08: `cumple_65_este_anio`, `cumple_65_proximo_anio`, `cumple_70_este_anio` y
    `cumple_70_proximo_anio`, con las reglas BD-FLG-05…07 y TR-D15.

- [ ] **Paso 1: tests que fallan.**
  - `ind_07_y_08_cuadran_con_manifiesto`:
    - IND-07 = `casos_contrato_plazo` (18), con desglose obra 11 e incremento 7;
    - IND-08 = `casos_jubilacion` (13), con desglose 65 = 9 y 70 = 4 (este y próximo año sumados por edad);
    - `origen.fecha_corte` es `2025-11-12`.
  - `filtro_por_gerencia_reparte_el_total` (IND-C1): en 2025-11, la suma de IND-01 sobre todas las gerencias
    (valores de la base) más `personas_sin_base` (8) es igual a IND-01 sin filtro. Lo mismo con IND-07, sin
    sumar personas sin base.
  - `filtro_inexistente_es_invalido` (Revisión 2): `gerencia = "operaciones"` (en minúsculas) y
    `gerencia = "x' OR '1'='1"` dan `FiltroInvalido`, con `opciones` igual a las gerencias de la base. `periodo` igual
    a `"2030-01"` y a `"2025-13"` dan `FiltroInvalido` con `filtro == "periodo"`.
  - `combinacion_sin_personas` (Revisión 3):
    - el test busca, con `consultar_texto` sobre el Parquet Gold, un par (gerencia, sede) sin empleados (falla con
      un mensaje claro si no lo hay);
    - con esos filtros, IND-01…03 e IND-07/08 valen `Some(0.0)` e IND-04…06 valen `None`;
    - el JSON no contiene `NaN`.
  - `solo_planillas` (Revisión 4): procesa solo las planillas de limpio en un temporal propio. IND-01 tiene valor;
    IND-07 e IND-08 dan `valor == None` y una `nota` que contiene "base de personal".
  - `sin_resultados_es_sin_datos`: una carpeta vacía da `Err(ErrorTablero::SinDatos(_))`.
- [ ] **Paso 2:** FAIL esperado (IND-07/08 ausentes y filtros sin validar).
- [ ] **Paso 3: implementar.**
- [ ] **Paso 4:** suite, clippy y fmt en verde.

### Tarea 4: estado, opciones de filtro, series, checks y detalle nominal

**Archivos:**
- Crear: `config/sql/indicadores/40_checks.sql` y `config/sql/indicadores/50_detalle.sql`.
- Modificar: `crates/core/src/tablero.rs` y `crates/core/src/tablero/respuesta.rs`.
- Test: `crates/core/tests/tablero.rs`.

**Interfaces (produce):**
- `Tablero::estado(&self) -> Result<Estado, ErrorTablero>`:
  - `Estado { marca, version_motor, fecha_corte, periodos, fuentes: Vec<Fuente>, avisos: usize,
    periodos_mezclados: bool, aviso_periodos: Option<String> }`;
  - IND-C2 (decisión 7) con el aviso "La base de personal tiene fecha de corte {fecha} y la última planilla es de
    {periodo}.";
  - `avisos` es el número de problemas de `ejecucion.json`.
- `Tablero::filtros(&self) -> Result<OpcionesFiltro, ErrorTablero>`: `OpcionesFiltro { periodos, gerencias, sedes,
  tipos_trabajador }`, cada una ordenada y sin nulos.
- `Tablero::series(&self, f: &Filtros) -> Result<RespuestaSeries, ErrorTablero>`:
  - `RespuestaSeries { filtros, series: Vec<Serie> }`;
  - `Serie { id, nombre, unidad, puntos: Vec<Punto> }` y `Punto { periodo, valor: Option<f64> }`;
  - IND-01…06, en todos los periodos y en orden; ignora `f.periodo`.
- `Tablero::checks(&self, f: &Filtros) -> Result<RespuestaChecks, ErrorTablero>`:
  - `RespuestaChecks { periodo, checks: Vec<Check> }` y `Check { id, valor: Option<f64>, mensaje }`;
  - IND-C1: personas de la planilla del periodo sin la base, **sin** filtros de dimensión. Mensaje: "{n} personas
    no aparecen al filtrar por gerencia o sede".
  - IND-C3: `(IND-01(mes) − IND-01(ant)) − (IND-02 − IND-03)`. Mensaje con las tres cifras, o "Sin mes anterior" y
    `valor None`.
- `Tablero::detalle(&self, tipo: Detalle, f: &Filtros) -> Result<RespuestaDetalle, ErrorTablero>`:
  - `Detalle` es `Alertas` o `SinBase` (`serde` en `kebab-case`: `alertas`, `sin-base`);
  - `RespuestaDetalle { tipo, periodo, filas: Vec<serde_json::Map<…>> }`, armada con `consultar_json`;
  - `detalle_alertas`: `empleados_f` con `alerta_contrato_plazo OR alerta_jubilacion`. Columnas
    `numero_documento, nombre_completo, gerencia, sede, tipo_trabajador, modalidad_contrato, fecha_fin_contrato,
    fecha_nacimiento, alerta_contrato_obra, alerta_contrato_incremento, cumple_65_este_anio,
    cumple_65_proximo_anio, cumple_70_este_anio, cumple_70_proximo_anio`.
  - `detalle_sin_base`: planilla del periodo sin la base. Columnas `numero_documento, apellido_paterno,
    apellido_materno, nombres, area, centro_costo`.

- [ ] **Paso 1: tests que fallan.**
  - `estado_datos_usados`:
    - limpio da `fecha_corte` 2025-11-12, 3 periodos, `periodos_mezclados == false` y `avisos == 0`;
    - con un `ejecucion.json` editado en un temporal (fecha de corte 2025-12-05), da `true` y un aviso que
      contiene "2025-11".
  - `opciones_de_filtro`: `periodos == ["2025-09", "2025-10", "2025-11"]`; gerencias, sedes y tipos son los
    `DISTINCT` del Parquet de la base.
  - `series_cuadran_con_indicadores`: en cada periodo, el punto de IND-01 es igual al IND-01 de `indicadores`.
  - `ind_c1_y_c3`:
    - IND-C1 por periodo es igual a `personas_sin_base` (27, 9, 8);
    - IND-C3 en 2025-10 es `(1224 − 1226) − (16 − 9) = −9`;
    - IND-C3 en 2025-09 es `None`.
  - `detalle_alertas_coincide_con_ind_07_08`: las filas son tantas como las personas con alguna alerta, y tienen
    la columna `nombre_completo`.
  - `detalle_sin_base_del_periodo`: 8 filas en 2025-11.
  - `solo_detalle_trae_nombres` (TR-D18): el JSON de `estado`, `filtros`, `indicadores`, `series` y `checks` no
    contiene ningún `numero_documento` ni `nombre_completo` del Parquet. Se comprueba con las primeras 50 filas de
    la base como texto.
- [ ] **Paso 2:** FAIL esperado.
- [ ] **Paso 3: implementar.**
- [ ] **Paso 4:** suite, clippy y fmt en verde.

### Tarea 5: crate `api` con axum

**Archivos:**
- Crear: `crates/api/Cargo.toml` (lib `etl_local_api`) y `crates/api/src/lib.rs`.
- Modificar:
  - `Cargo.toml` del workspace: miembro `crates/api` y las dependencias `axum = "0.8.9"`,
    `tokio = { version = "1.54", features = ["rt-multi-thread", "macros", "net", "signal"] }`,
    `tower = { version = "0.5.3", features = ["util"] }` y `http-body-util = "0.1.5"`.
  - Dependencia de `etl-local-api` en el workspace.
- Test: `crates/api/tests/api.rs`. Procesa limpio con `etl_local_core::proceso::procesar` en un `OnceLock`. Los
  fixtures salen de `../core/tests/fixtures/demo/limpio`.

**Interfaces:**
- Consume: Tareas 2–4 (`Tablero`, `Filtros`, `Detalle` y `ErrorTablero::codigo`).
- Produce:
  - `pub fn router(tablero: Tablero, puerto: u16) -> axum::Router`;
  - `pub async fn servir(listener: tokio::net::TcpListener, tablero: Tablero) -> std::io::Result<()>`: toma el
    puerto de `listener.local_addr()` y termina con Ctrl+C mediante `with_graceful_shutdown`.

**Rutas:**
- `GET /api/v1/estado`;
- `GET /api/v1/filtros`;
- `GET /api/v1/indicadores`, `GET /api/v1/series` y `GET /api/v1/checks`, con `?periodo&gerencia&sede&tipo_trabajador`;
- `GET /api/v1/detalle/{tipo}`, con los mismos parámetros.

**Comportamiento:**
- **Ejecución:** cada handler llama al núcleo dentro de `tokio::task::spawn_blocking`.
- **Errores**, con cuerpo `{ "codigo", "mensaje", "opciones"? }`:
  - `sin_datos` → 404;
  - `filtro_invalido` → 400, con `opciones`;
  - `interno` → 500, con el mensaje del error;
  - un rechazo de `Query` (parámetro desconocido) → 400 `filtro_invalido`, con el mensaje de serde;
  - una ruta inexistente → 404 `ruta_inexistente`;
  - un tipo de detalle desconocido → 404 `ruta_inexistente`.
- **Host:** una capa (`middleware::from_fn`) acepta solo `127.0.0.1:{puerto}` y `localhost:{puerto}`. Cualquier
  otro `Host`, o su ausencia, da 403 `host_no_permitido`.

- [ ] **Paso 1: tests que fallan** (con `ServiceExt::oneshot` y el encabezado `Host: 127.0.0.1:8787`, salvo
  donde se indica).
  - `estado_ok`: 200, `fecha_corte == "2025-11-12"`.
  - `indicadores_con_filtro`: `?periodo=2025-10` da 200, `periodo == "2025-10"` y 8 indicadores.
  - `parametro_desconocido_400` (Revisión 2): `?gerencias=X` da 400 `filtro_invalido`.
  - `filtro_inyeccion_400`: `?gerencia=x%27%20OR%20%271%27%3D%271` da 400, con `opciones` no vacío.
  - `detalle_sin_base`: `/api/v1/detalle/sin-base?periodo=2025-11` da 200 y 8 filas. `/api/v1/detalle/otro` da 404.
  - `host_ajeno_403` (Revisión 5): `Host: evil.example`, `Host: 127.0.0.1:9999` y sin `Host` dan 403
    `host_no_permitido`.
  - `sin_datos_404` (Revisión 4): un router sobre una carpeta vacía responde 404 `sin_datos` en `/api/v1/estado` e
    `/api/v1/indicadores`.
  - `ruta_inexistente_404`: `/api/v1/nada` da 404 JSON.
- [ ] **Paso 2:** `cargo test -p etl-local-api`. Esperado: FAIL de compilación.
- [ ] **Paso 3: implementar.**
- [ ] **Paso 4:** suite, clippy y fmt en verde.

### Tarea 6: comando `etl-local tablero` y documentación

**Archivos:**
- Crear: `crates/cli/src/tablero.rs`.
- Modificar:
  - `crates/cli/src/main.rs` (variante `Tablero`);
  - `crates/cli/Cargo.toml` (`etl-local-api` y `tokio`).
- Test: `crates/cli/tests/cli.rs`.
- Documentación:
  - `docs/logica-negocio/indicadores-tablero.md`: nueva §7 "Implementación (Plan 5)", con una tabla ID → archivo
    SQL y las decisiones 3, 5, 6, 7, 8 y 10;
  - `AGENTS.md` §2 (crate `api`) y §6 (comando `tablero`);
  - `config/README.md` (`indicadores.yaml`, `sql/indicadores/`, `metas`);
  - hoja de ruta: Plan 5 **Completado**;
  - ADR-0004: nota con el enlace a la §7.

**Interfaces:**
- Consume: `etl_local_api::servir` y `Tablero::nuevo`.
- Produce: `OpcionesTablero { --salida <carpeta> (requerida), --config <carpeta> (default "config"),
  --puerto <u16> (default 8787) }`.
  - Liga `127.0.0.1:{puerto}` y, si lo logra, imprime en stdout `Tablero en http://127.0.0.1:{puerto_real}/api/v1/estado
    (Ctrl+C para salir)`.
  - Si el puerto está ocupado (`AddrInUse`): stderr "El puerto {n} está ocupado. Use --puerto con otro número." y
    salida 2.
  - Si falta `--salida/actual`, igual arranca: la API responde `sin_datos` y el usuario puede procesar después sin
    reiniciar (ADR-0004 §1).

- [ ] **Paso 1: tests que fallan.**
  - `tablero_responde`:
    - procesa limpio a un temporal;
    - lanza `etl-local tablero --salida <tmp> --puerto 0`;
    - lee la primera línea de stdout y extrae el puerto;
    - con `std::net::TcpStream` hace `GET /api/v1/estado` con `Host: 127.0.0.1:{puerto}`;
    - comprueba `HTTP/1.1 200` y que el cuerpo contiene `"2025-11-12"`;
    - mata el proceso.
  - `puerto_ocupado_sale_2` (Revisión 5): ocupa un puerto con `TcpListener::bind("127.0.0.1:0")` y lanza `tablero`
    en ese puerto. Salida 2 y stderr que contiene "--puerto".
- [ ] **Paso 2:** FAIL esperado.
- [ ] **Paso 3:** implementar y escribir la documentación.
- [ ] **Paso 4:** suite, clippy y fmt en verde.
- [ ] **Paso 5: prueba manual.**
  - Procesar `demo/generated/limpio` (6 meses) y levantar `tablero`.
  - Pedir `/api/v1/indicadores` y `/api/v1/series` con `curl` y anotar en el registro de avance:
    - los valores de IND-01…08 del último mes;
    - el tiempo de respuesta, con un objetivo menor a 500 ms.
  - Revisar que no haya nombres fuera de `/detalle/` (`grep` del JSON).

## Hechos verificados (2026-11-16)

- **Manifiesto de `limpio` (3 meses):**

  | Periodo | Personas | Entradas | Salidas | Sin base |
  |---|---|---|---|---|
  | 2025-09 | 1226 | 26 | 18 | 27 |
  | 2025-10 | 1224 | 16 | 9 | 9 |
  | 2025-11 | 1232 | 17 | 20 | 8 |

  - Base de personal: fecha de corte 2025-11-12, 18 casos de contrato y 13 de jubilación.
- **Planilla:** solo trae empleados (el generador excluye a los practicantes). Quien entra aparece desde su mes, y
  quien sale aparece en su mes de salida.
- **Columnas:** el Gold de la planilla tiene `fecha_entrada`, `fecha_salida`, `motivo_salida`, las 7 columnas de
  días de §3 (`dias_subsidio_otros` siempre existe) y las 4 de horas extra. `base_personal_empleados` tiene
  `gerencia`, `sede`, `tipo_trabajador`, `nombre_completo`, los flags `cumple_*` y `alerta_*`, y un registro por
  documento.
- **Plan 4:** su motor da 11 + 7 alertas de contrato y 9 + 4 de jubilación, que coinciden con el manifiesto.
- **Versiones de crates.io (2026-11-16):** axum 0.8.9, tokio 1.54.1, tower 0.5.3 y http-body-util 0.1.5.
