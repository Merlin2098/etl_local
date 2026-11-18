# Lógica de negocio — Control de Practicantes

> Especificación agnóstica de lenguaje extraída del legacy Python (`HR_Data_Integration_Pipeline`).
> Rutas relativas a la raíz del repo legacy. "(inferido)" = deducción no explícita en el código.

## 1. Propósito

Procesar el Excel de control de practicantes (un archivo por corte, con la fecha de corte en el nombre) y producir:

- **Silver**: tabla limpia con las 10 columnas de negocio + `FECHA_DOCUMENTO`.
- **Gold**: la misma población enriquecida con tiempo de servicio (días/años/meses), tres flags de vencimiento para `PRACTICANTE PROFESIONAL` y un texto legible `tiempo_servicio`.

Flujo productivo (UI): Widget → Worker (preflight) → Executor YAML (2 stages) → `step1.procesar_sin_gui` → `step2.procesar_sin_gui`.
(`src/modules/control_practicantes/ui/worker.py:41-116`, `src/orchestrators/pipelines/pipeline_control_practicantes.yaml:11-35`)

## 2. Fuentes (Bronze)

| Aspecto | Valor | Fuente |
|---|---|---|
| Tipo | Excel `.xlsx` / `.xlsm` / `.xls` (la lectura usa un lector OOXML; `.xls` real probablemente falla, ver §9) | `assets/validate_source/control_practicantes.yaml:4-8` |
| Patrón de nombre | `(?i)^BD Practicantes\s+(?P<fecha>\d{2}\.\d{2}\.\d{4})\.(xlsx\|xlsm\|xls)$` → ej. `BD Practicantes 30.03.2026.xlsx` | `assets/validate_source/control_practicantes.yaml:3`, `src/utils/bd_document_date.py:17-19` |
| Hoja | `Practicantes` (obligatoria; otras hojas se ignoran) | `step1_controlpracticantes.py:168-173` |
| Fila de cabecera | Fila 4 (1-based), desde columna A | `step1_controlpracticantes.py:176`, contrato `:17-19` |
| Inicio de datos | Fila 5 | `step1_controlpracticantes.py:219` |
| Fin de datos | 1000 filas consecutivas sin DNI **después** de haber leído al menos una fila válida; si no, hasta la última fila de la hoja | `step1_controlpracticantes.py:35, 231-243` |
| Columnas requeridas | `N°`, `DNI`, `APELLIDOS Y NOMBRES`, `CONDICION`, `FECHA ING`, `F. RENOVACION`, `SEDE`, `UNIVERSIDAD`, `JEFE INMEDIATO`, `GERENCIA` | `assets/esquemas/esquema_control_practicantes.yaml:1-11` |
| Tipos esperados | Todas string salvo `FECHA ING` y `F. RENOVACION` = date | `assets/esquemas/esquema_control_practicantes.yaml:12-22` |

Las cabeceras pueden venir "combinadas"/multilínea; se limpian con CP-BRZ-03.

## 3. Reglas Bronze → Silver

- **CP-BRZ-01 — Preflight obligatorio.** Antes de cualquier stage se valida el archivo contra el contrato `control_practicantes` (ver `validacion-preflight.md`). Si falla, no se ejecuta nada. (`ui/worker.py:79-84`)
- **CP-BRZ-02 — Validación estructural del executor.** El executor vuelve a comprobar que el archivo existe y que contiene la hoja `Practicantes`; si el nombre no contiene un fragmento `DD.MM.YYYY` solo emite WARNING. (`pipeline_control_practicantes_executor.py:131-174`)
- **CP-BRZ-03 — Limpieza de cabecera (`limpiar_nombre_columna`).** Para cada celda de la fila 4: `null` → `""`; convertir a texto; `trim`; reemplazar `\n` y `\r` por espacio; colapsar cualquier secuencia de espacios en blanco en un único espacio. **No** cambia mayúsculas/minúsculas ni acentos. (`step1_controlpracticantes.py:104-119`)
- **CP-BRZ-04 — Mapeo de columnas por nombre exacto.** Se recorren las celdas de la fila 4; si el nombre limpio coincide **exactamente** (sensible a mayúsculas y acentos) con una columna requerida, se guarda su índice. Si dos cabeceras limpias son iguales, gana la última (sobrescritura de mapa). Las columnas no requeridas se descartan. (`step1_controlpracticantes.py:194-199`)
- **CP-BRZ-05 — Columnas faltantes = aborto.** Si falta cualquier columna requerida → error "Columnas requeridas no encontradas en 'Practicantes': …" (lista ordenada alfabéticamente). Si la fila 4 no tiene ninguna cabecera no nula → error. (`step1_controlpracticantes.py:179-186, 202-209`)
- **CP-BRZ-06 — Filtro de filas por DNI.** Una fila se descarta si su celda DNI es nula, vacía o solo espacios (o si la fila es más corta que el índice de DNI). Contador de consecutivas sin DNI se reinicia con cada fila válida; solo cuenta tras la primera fila válida. Al llegar a 1000 consecutivas, se corta la lectura (fin de datos). (`step1_controlpracticantes.py:219-245`)
- **CP-BRZ-07 — Sin filas válidas = aborto.** Si tras el filtro no queda ninguna fila → error. (`step1_controlpracticantes.py:260-265`)
- **CP-BRZ-08 — Normalización de valores (`normalizar_valor_celda`).** Por celda, en este orden (`step1_controlpracticantes.py:122-141`):
  1. `null` → `null`.
  2. Si la columna es `FECHA ING` o `F. RENOVACION` → CP-BRZ-09.
  3. datetime → texto `YYYY-MM-DD HH:MM:SS`.
  4. float entero (p.ej. `12345678.0`) → `"12345678"`; float no entero → representación decimal fija sin ceros finales (`1.50` → `"1.5"`).
  5. Cualquier otro → texto con `trim`.
  No hay relleno de ceros a la izquierda en DNI (un DNI numérico con cero inicial lo pierde; inferido).
- **CP-BRZ-09 — Conversión de fechas (`convertir_fecha_excel`).** Devuelve texto `YYYY-MM-DD` o `null` (`step1_controlpracticantes.py:335-372`):
  - datetime → su fecha.
  - Número (entero o real) = serial Excel: `1899-12-30 + truncar(valor)` días (parte decimal/hora descartada). Base 1899-12-30 compensa el bug del año bisiesto 1900.
  - Texto (trim) → se prueban en orden: `%Y-%m-%d`, `%d/%m/%Y`, `%d-%m-%Y`, `%Y/%m/%d`, `%d.%m.%Y` (día primero). El primero que parsea gana.
  - Cualquier otro caso (p.ej. texto con hora `2025-04-01 00:00:00`, texto en otro formato) → `null` silencioso.
- **CP-BRZ-10 — Tipado de fechas.** Tras construir la tabla, `FECHA ING` y `F. RENOVACION` se re-pasan por CP-BRZ-09 (idempotente sobre `YYYY-MM-DD`) y se convierten a tipo **Date** en modo no estricto (lo no parseable queda `null`, no falla). (`step1_controlpracticantes.py:273-316`)
- **CP-BRZ-11 — Resto de columnas como texto.** Toda columna distinta de las dos fechas se fuerza a string. (`step1_controlpracticantes.py:319-321`)
- **CP-BRZ-12 — Validación de esquema no bloqueante.** Se comprueban columnas requeridas y tipos (`string` ↔ Utf8/String, `date` ↔ Date); los errores solo se imprimen como advertencias. (`step1_controlpracticantes.py:63-101, 545-550`)
- **CP-BRZ-13 — `FECHA_DOCUMENTO`.** Se extrae la fecha del nombre del archivo con la regex de §2 (búsqueda sobre el nombre base), se parsea como `DD.MM.YYYY` (debe ser fecha de calendario válida, p.ej. `31.02.2026` falla) y se formatea `YYYY-MM-DD`. Se añade como **columna de texto constante** al final de la tabla. Si el nombre no cumple → `ValueError("Filename does not match expected pattern 'BD Practicantes DD.MM.YYYY.xlsx|xlsm|xls'")`; fecha imposible → `ValueError("Invalid document date in filename: …")`. Se añade **después** de la validación de esquema. (`src/utils/bd_document_date.py:22-42, 62-68, 92-99`; `step1_controlpracticantes.py:552-553`; test `tests/test_control_practicantes_document_date.py:22-62, 115-116`)

**Esquema Silver resultante** (orden): las 10 columnas requeridas en el orden de `required_columns` + `FECHA_DOCUMENTO` (string). Sin deduplicación, sin ordenamiento.

## 4. Reglas Silver → Gold

- **CP-GLD-01 — Motor.** Se carga Silver y se ejecuta `assets/queries/query_control_practicantes_gold.sql` sobre una tabla llamada `control_practicantes_silver` (DuckDB en memoria). (`step2_controlpracticantes.py:269-291`)
- **CP-GLD-02 — Proyección.** Gold contiene, en este orden: `DNI`, `CONDICION`, `Fecha_Ingreso` (= `FECHA ING`), `Fecha_Renovacion` (= `F. RENOVACION`), `FECHA_DOCUMENTO`, `SEDE`, `UNIVERSIDAD`, `JEFE INMEDIATO`, `GERENCIA`, `dias_servicio`, `anios_servicio`, `meses_servicio`, `por_cumplir_1`, `cumplio_1`, `por_cumplir_2`, `tiempo_servicio`. **Se descartan `N°` y `APELLIDOS Y NOMBRES`.** (`query_control_practicantes_gold.sql:2-11, 31-32, 57-72`)
- **CP-GLD-03 — Orden.** `ORDER BY Fecha_Ingreso DESC` (nulos al final por defecto del motor; inferido). (`query_control_practicantes_gold.sql:74`)
- **CP-GLD-04 — Sin filtros.** No se filtra ninguna fila; todas las CONDICION pasan a Gold.

## 5. Reglas de negocio / flags

Sea `D = F. RENOVACION − FECHA ING` en días enteros (fecha − fecha).

- **CP-FLG-01 — `dias_servicio`.** `D` si `F. RENOVACION` no es nula; si no, `null`. Si `FECHA ING` es nula, el resultado también es nulo (aritmética con nulo). **Nota: se mide contra la fecha de renovación, no contra `FECHA_DOCUMENTO` ni contra hoy.** (`query_control_practicantes_gold.sql:12-16`)
- **CP-FLG-02 — `anios_servicio`.** `floor(D / 365.25)` como entero; `null` si renovación nula. El comentario "≥335 días se trata como año 1" **no está implementado**. (`query_control_practicantes_gold.sql:17-22`)
- **CP-FLG-03 — `meses_servicio`.** `floor((D mod 365.25) / 30.44)` como entero; `null` si renovación nula. `mod` es módulo real (resto con decimales). Con `D` negativo el resultado puede ser negativo (inferido). (`query_control_practicantes_gold.sql:23-27`)
- **CP-FLG-04 — `por_cumplir_1`** (por cumplir 1 año, faltan ≤ ~1 mes):
  `null` si `dias_servicio` nulo; `'SI'` si `CONDICION = 'PRACTICANTE PROFESIONAL'` **y** `335 ≤ D ≤ 364`; si no `'NO'`. (`query_control_practicantes_gold.sql:33-39`)
- **CP-FLG-05 — `cumplio_1`** (ya cumplió 1 año):
  `null` si nulo; `'SI'` si `PRACTICANTE PROFESIONAL` **y** `D ≥ 365`; si no `'NO'`. Sin tope superior: también es `'SI'` en la ventana de `por_cumplir_2` y por encima de 730. (`query_control_practicantes_gold.sql:40-46`)
- **CP-FLG-06 — `por_cumplir_2`** (por cumplir 2 años, faltan ≤ ~3 meses):
  `null` si nulo; `'SI'` si `PRACTICANTE PROFESIONAL` **y** `640 ≤ D ≤ 729`; si no `'NO'`. (`query_control_practicantes_gold.sql:47-53`)
- **CP-FLG-07 — Comparación de CONDICION.** Igualdad exacta, sensible a mayúsculas, contra `'PRACTICANTE PROFESIONAL'` (el valor llega con `trim` desde Silver). Cualquier otra condición (p.ej. pre-profesional) recibe `'NO'` en los tres flags cuando `D` no es nulo. (`query_control_practicantes_gold.sql:35, 42, 49`)
- **CP-FLG-08 — `tiempo_servicio` (texto).** Primera regla que aplica (`query_control_practicantes_gold.sql:60-72`):
  1. `anios_servicio` nulo → `null`
  2. `por_cumplir_1 = 'SI'` → `'1 año (En límite)'`
  3. `por_cumplir_2 = 'SI'` → `'2 años (Límite Máximo)'`
  4. `anios = 0` → `'<meses> meses'` (incluye `'0 meses'`, `'1 meses'`; sin singular)
  5. `anios = 1` y `meses = 0` → `'1 año'`
  6. `anios = 1` → `'1 año y <meses> meses'`
  7. `meses = 0` → `'<anios> años'`
  8. resto → `'<anios> años y <meses> meses'`

Tabla resumen de ventanas (solo `PRACTICANTE PROFESIONAL`):

| D (días) | por_cumplir_1 | cumplio_1 | por_cumplir_2 |
|---|---|---|---|
| < 335 | NO | NO | NO |
| 335–364 | **SI** | NO | NO |
| 365–639 | NO | **SI** | NO |
| 640–729 | NO | **SI** | **SI** |
| ≥ 730 | NO | **SI** | NO |
| renovación nula | null | null | null |

- **CP-FLG-09 — Estadísticas.** Se cuentan filas con `'SI'` por flag y se devuelven en el resultado del stage (`flags`) y en el mensaje final de UI; no afectan datos. (`step2_controlpracticantes.py:296-300`; `ui/worker.py:136-144`)

## 6. Salidas

| Capa | Ruta | Formato | Versionado |
|---|---|---|---|
| Silver | `<carpeta_del_archivo>/silver/control_practicantes_silver.parquet` | Parquet snappy | Sin timestamp; se sobrescribe |
| Gold | `<carpeta_del_archivo>/gold/control_practicantes_flagsgold.parquet` | Parquet snappy | Sin timestamp; se sobrescribe |
| Gold Excel (opcional) | `<carpeta_del_archivo>/gold/control_practicantes_flagsgold.xlsx` | xlsx (tipos tal cual) | Solo si la casilla "Generar Excel adicional en Gold" está marcada (por defecto **no**) |

- **CP-OUT-01** — La carpeta de trabajo es siempre la carpeta que contiene el Excel de entrada (`ui/worker.py:87`; `src/utils/ui/widgets/base_etl_widget.py:175`). Se crean `silver/` y `gold/` si no existen (`step1_controlpracticantes.py:557`, `step2_controlpracticantes.py:308`).
- **CP-OUT-02** — El executor pasa rutas fijas por índice de stage (stage 0: archivo + `silver/`; stage 1: `silver/control_practicantes_silver.parquet` + `gold/` + flag Excel). (`pipeline_control_practicantes_executor.py:286-319`)
- **CP-OUT-03** — Tras cada stage se verifica la existencia de los outputs declarados en el YAML con `required: true` (Silver parquet y Gold parquet); el Excel es `required: false`. Falta de un requerido = error del stage. (`pipeline_control_practicantes_executor.py:321-344`; `pipeline_control_practicantes.yaml:17-35`)
- **CP-OUT-04** — No hay histórico ni timestamp en este dominio.

## 7. Validaciones y errores

| Situación | Comportamiento | Fuente |
|---|---|---|
| Preflight falla (nombre, fecha de nombre, hoja, cabecera en A4, columnas) | Se aborta antes de los stages; error "Preflight / Validate Source" | `ui/worker.py:172-182` |
| Archivo no existe / sin hoja `Practicantes` (executor) | `success=false`, "Validación de archivo fallida" | `pipeline_control_practicantes_executor.py:359-361` |
| Columnas no encontradas por nombre exacto en step1 | Excepción → stage falla | `step1_controlpracticantes.py:202-209` |
| 0 filas con DNI | Excepción → stage falla | `step1_controlpracticantes.py:260-265` |
| Tipos no conformes al esquema | Solo advertencia | `step1_controlpracticantes.py:547-550` |
| Fechas no parseables | `null` silencioso → `dias_servicio` y flags `null` | CP-BRZ-09/10 |
| Stage devuelve `success=false` o `error` | Se trata como fallo | `pipeline_control_practicantes_executor.py:66-86, 233-235` |
| Fallo de stage | `stop_on_error: true` → se detiene; se reportan stage, módulo, tipo de excepción y 8 líneas de traceback | `pipeline_control_practicantes.yaml:8`; executor `:378-396` |

## 8. Discrepancias

1. **Preflight vs step1 en matching de cabeceras.** El preflight compara cabeceras en mayúsculas y con espacios colapsados (insensible a mayúsculas); step1 exige coincidencia exacta. Una cabecera `Dni` pasa el preflight y luego falla en step1. (`src/utils/validate_source.py:363-373` vs `step1_controlpracticantes.py:194-199`)
2. **Widget/executor "advertencia suave" vs requisito duro.** Widget y executor solo avisan si falta un fragmento `DD.MM.YYYY` en el nombre (y el widget en ese caso **omite** la comprobación de hoja), pero el preflight y `FECHA_DOCUMENTO` exigen el patrón completo `BD Practicantes DD.MM.YYYY.ext` → error duro. (`ui/widget.py:76-93`; executor `:145-151`; `bd_document_date.py:29-33`)
3. **Docstrings vs SQL.** Docstring de step2 dice "falta ≤ 1 mes" / "≤ 3 meses" (ventanas reales en días: 335–364 y 640–729) y nombra la salida `control_practicantes_gold.*` (real: `control_practicantes_flagsgold.*`). Comentario SQL "≥335 días = año 1" no implementado en `anios_servicio`. (`step2_controlpracticantes.py:8-16`; SQL `:17`)
4. **Solapamiento de flags.** `cumplio_1` y `por_cumplir_2` son ambos `'SI'` en 640–729; `tiempo_servicio` muestra `'2 años (Límite Máximo)'` cuando el tiempo real es ~1 año y 9–11 meses. No existe flag de "superó 2 años". (SQL `:40-53, 63-64`)
5. **Modo standalone vs headless.** `main()` de step2 llama a `guardar_resultados` sin `export_excel` → nunca genera Excel; la carpeta Gold se deriva de `silver.parent.parent`. El modo headless (el productivo) recibe `carpeta_gold` explícita. `main()` de step1 no ejecuta preflight. (`step2_controlpracticantes.py:124-171, 205`; `step1_controlpracticantes.py:411-511`)
6. **Doble fuente de columnas requeridas.** El contrato preflight declara `extract: required_columns_array` del esquema **y** una lista explícita; la lista explícita gana y el esquema se ignora en preflight (pueden divergir). (`assets/validate_source/control_practicantes.yaml:20-33`; `validate_source.py:254-256`)
7. **Filtro del diálogo de archivos** del widget no incluye `.xlsm`, aunque sí se acepta por extensión y por contrato. (`ui/widget.py:31, 67`)
8. **YAML de pipeline sin parámetros reales**: `params` vacíos; el executor codifica rutas y argumentos por índice de stage. (`pipeline_control_practicantes.yaml:16, 28`; executor `:286-319`)

## 9. Ambigüedades y preguntas abiertas

1. ¿El tiempo de servicio debe medirse hasta `F. RENOVACION` (comportamiento actual) o hasta `FECHA_DOCUMENTO`/hoy? `FECHA_DOCUMENTO` se calcula pero no se usa en ningún cálculo.
2. ¿Qué significa `F. RENOVACION` (próxima renovación o última)? Determina si los flags son "a futuro" o "a la fecha de la última renovación".
3. ¿Deben otras condiciones (pre-profesional, etc.) tener reglas propias? Hoy reciben siempre `'NO'`.
4. ¿Qué hacer con `D` negativo (renovación anterior al ingreso)? Hoy produce años/meses negativos sin error (inferido).
5. ¿Debe preservarse el cero inicial del DNI y debe deduplicarse por DNI? Hoy no se hace ninguna de las dos.
6. ¿`.xls` (BIFF) está realmente soportado? El lector usado (openpyxl) solo abre OOXML, por lo que un `.xls` real fallaría en preflight con "Unable to open Excel file" (inferido).
7. ¿Por qué se descartan `N°` y `APELLIDOS Y NOMBRES` en Gold? ¿Intencional (privacidad) o descuido?
8. Aproximaciones 365.25 / 30.44: ¿aceptables o se requiere diferencia de calendario exacta?

## Decisiones del rework (Plan 8)

Reporte «Control de practicantes» (`practicantes`), hecho con el molde de reportes
([config/README.md](../../config/README.md)).

| Pregunta / regla | Decisión |
|---|---|
| §9.1–9.2 hasta qué fecha | Se conserva la fórmula del legacy: `D = F. RENOVACION − FECHA ING`. |
| §9.3 otras condiciones | Solo la condición de `parametros.yaml` (`control_practicantes.condicion`) tiene alertas; se compara normalizada (TR-D24). |
| §9.4 `D` negativo o sin renovación | Tiempo y alertas nulos, con el aviso `PRA-01` que dice las filas. |
| §9.5 cero inicial del DNI | Lo conserva el tipo `documento` del contrato. |
| §9.7 `N°` y `APELLIDOS Y NOMBRES` | No se leen. |
| §9.8 aproximaciones | Años de **365** días (el legacy usaba 365,25 y a los 365 días decía «11 meses» junto a «cumplió 1 año»); meses de 30,44 días. |
| CP-FLG-04…06 | Ventanas en `parametros.yaml` (`control_practicantes`); alertas booleanas. |
| CP-FLG-08 texto | Con singular correcto («1 mes», «1 año»). |

## 10. Referencias

- `src/modules/control_practicantes/steps/step1_controlpracticantes.py:34-35, 63-141, 144-332, 335-372, 514-573`
- `src/modules/control_practicantes/steps/step2_controlpracticantes.py:32-44, 63-171, 247-339`
- `src/modules/control_practicantes/ui/worker.py:41-192`; `ui/widget.py:29-141`; `ui/config.py:23-30`
- `src/orchestrators/pipeline_control_practicantes_executor.py:66-86, 131-174, 176-284, 286-344, 346-429`
- `src/orchestrators/pipelines/pipeline_control_practicantes.yaml:1-35`
- `src/utils/bd_document_date.py:12-19, 22-50, 62-99`
- `src/utils/gold_export.py:7-27`
- `assets/validate_source/control_practicantes.yaml:1-33`
- `assets/esquemas/esquema_control_practicantes.yaml:1-22`
- `assets/queries/query_control_practicantes_gold.sql:1-74`
- `tests/test_control_practicantes_document_date.py:22-177`
