# Lógica de negocio — Examen de Retiro

> Especificación agnóstica de lenguaje extraída del legacy Python (`HR_Data_Integration_Pipeline`).
> Rutas relativas a la raíz del repo legacy. "(inferido)" = deducción no explícita en el código.

## 1. Propósito

Procesar el Excel de programación de exámenes médicos de retiro (empleados cesados) y producir:

- **Silver**: todas las columnas del Excel, limpias (trim, fechas tipadas, DNI como texto).
- **Gold**: subconjunto de columnas del esquema + columnas derivadas `AÑO`, `MES`, `NOMBRE_MES`, excluyendo practicantes (`CARGO = 'STUDENT INTERNAL'`).
- **Gold enriquecido**: Gold + datos del centro de costo (CC) desde dos catálogos (actual y antiguo) con un indicador de calidad del match.

Flujo productivo (UI): Widget (3 archivos) → `ExamenRetiroWorker` (preflight + 3 steps inline). El worker **no** usa `BaseETLWorker` ni el executor YAML. (`src/modules/examen_retiro/ui/worker.py:40-80, 136-614`)

## 2. Fuentes (Bronze)

### 2.1 Excel de exámenes de retiro

| Aspecto | Valor | Fuente |
|---|---|---|
| Tipo | `.xlsx` / `.xlsm` / `.xls` (lector OOXML; `.xls` real probablemente falla, inferido) | `assets/validate_source/examen_retiro.yaml:3-7`; `ui/widget.py:194-196` |
| Patrón de nombre | **Ninguno** (no hay `filename_regex`) | `assets/validate_source/examen_retiro.yaml` |
| Hoja | `DATA` (obligatoria) | `step1_clean.py:62-67` |
| Cabecera | Fila 3, desde columna A; se leen celdas hasta la **primera celda vacía** | `step1_clean.py:70-75` |
| Inicio de datos | Fila 4 | `step1_clean.py:83` |
| Fin de datos | Primera fila cuya celda de la columna A es nula o texto vacío/espacios | `step1_clean.py:84-86` |
| Columnas requeridas (preflight) | `NOMBRE`, `DNI`, `FECHA DE CESE` | `assets/validate_source/examen_retiro.yaml:19-28` |
| Columnas usadas en Gold | `NOMBRE`, `DNI`, `FECHA DE CESE`, `CAUSA DE SALIDA`, `CARGO`, `NOMBRE DE CC` | `assets/esquemas/esquema_examen_retiro.yaml:8-32` |

### 2.2 Catálogos de centros de costo (Parquet)

| Archivo | Origen | Columnas que usa el join |
|---|---|---|
| `CC_ACTUAL` | Generado por el módulo `bd`: `<carpeta_trabajo_bd>/centros_costo/actual/CC_ACTUAL.parquet` (copias con timestamp `CC_ACTUAL_YYYYMMDD_HHMMSS.parquet` en `centros_costo/historico/`). Deduplicado por `CC` (se queda la primera aparición) y ordenado por `CC`. | `CC`, `NOMBRE CC`, `CATEGORIA CC`, `GERENCIA` |
| `CC_OLD` | **Ningún módulo del repo lo genera**; el usuario lo selecciona manualmente (inferido: catálogo histórico mantenido a mano). Debe tener las mismas 4 columnas. | `CC`, `NOMBRE CC`, `CATEGORIA CC`, `GERENCIA` |

Fuentes: `src/modules/bd/steps/step1_5_centrosdecosto.py:127-144, 166-205`; `src/modules/bd/ui/worker.py:341-377`; `assets/esquemas/esquema_cc.yaml:1-11`; `assets/queries/query_cc_join.sql:50-67`.
El widget solo exige que ambos archivos tengan extensión `.parquet`; no se valida su contenido antes del join. (`ui/widget.py:221-223, 249-251`)

## 3. Reglas Bronze → Silver (`step1_clean`)

- **ER-BRZ-01 — Preflight.** Solo el Excel Bronze se valida contra el contrato `examen_retiro` (hoja `DATA`, cabecera no vacía en A3, columnas `NOMBRE`/`DNI`/`FECHA DE CESE`). Si falla se aborta todo. Los parquets CC no pasan preflight. (`ui/worker.py:150-157, 616-629`)
- **ER-BRZ-02 — Cabeceras.** Valor de cada celda de la fila 3 convertido a texto con `trim`, de izquierda a derecha, deteniéndose en la primera celda nula. Columnas posteriores a un hueco se ignoran. No se normalizan mayúsculas, acentos ni espacios internos. Cabeceras duplicadas: la última columna con ese nombre sobrescribe a las anteriores (inferido por semántica de diccionario). (`step1_clean.py:70-75, 120-123`)
- **ER-BRZ-03 — Ancho de lectura.** Cada fila de datos se lee solo hasta la columna N = número de cabeceras. (`step1_clean.py:83`)
- **ER-BRZ-04 — Fin de datos.** La lectura termina en la primera fila con columna A vacía (nula o solo espacios); filas posteriores se ignoran aunque tengan datos. (`step1_clean.py:84-86`)
- **ER-BRZ-05 — Normalización de celdas** (`step1_clean.py:90-104`):
  - Texto que **empieza por `#`** (errores Excel `#N/A`, `#REF!`… y cualquier texto con `#` inicial) → `null`.
  - datetime → texto `YYYY-MM-DD HH:MM:SS`; date → `YYYY-MM-DD`.
  - Entero o float entero → texto sin decimales (`12345678.0` → `"12345678"`); float no entero → texto del número.
  - Resto sin cambios.
- **ER-BRZ-06 — Sin datos.** Si no hay filas → tabla vacía; el worker devuelve error "No se encontraron datos en el archivo Excel". (`step1_clean.py:115-117`; `ui/worker.py:189-201`)
- **ER-SLV-01 — Trim.** Toda columna de texto: `trim` de espacios en ambos extremos. (`step1_clean.py:151-155`)
- **ER-SLV-02 — Fechas.** Columnas `F. NACIMIENTO`, `FECHA DE CESE`, `UTLIMA FECHA PARA EXAMEN MEDICO` (sic, con la errata), `2DA CONVOCATORIA EXAMEN MEDICO`, si existen: parseo no estricto con formato `YYYY-MM-DD HH:MM:SS` y conversión a Date. Valores que no cumplen ese formato (p.ej. fechas escritas como texto `15/03/2025`) quedan **`null` sin error**. El fallback `YYYY-MM-DD` solo se ejecuta si la primera conversión lanza excepción (no ocurre por ser no estricta), por lo que en la práctica no se usa (inferido). (`step1_clean.py:158-186`)
- **ER-SLV-03 — DNI.** Se fuerza a texto y se elimina un sufijo `.0` final. Sin relleno de ceros. (`step1_clean.py:189-195`)
- **ER-SLV-04 — Nulos en texto.** Todo `null` en columnas de texto → cadena vacía `""` (no afecta a columnas Date). Esto es lo que permite el estado `SIN_CODIGO` en el join. (`step1_clean.py:198-200`)
- **ER-SLV-05 — Sin filtros ni deduplicación** en Silver; se conservan todas las columnas del Excel.

## 4. Reglas Silver → Gold (`step2_gold`)

- **ER-GLD-01 — Esquema.** Se carga `assets/esquemas/esquema_examen_retiro.(yaml|yml|json)` (resolución por extensión, ver `validacion-preflight.md` PRE-03). Si no se encuentra, el worker **omite Step 2 y Step 3** con advertencia y termina con éxito (solo Silver). (`ui/worker.py:280-288`)
- **ER-GLD-02 — Columnas base.** Columnas del bloque `schema` **sin** la clave `derivado_de`, en el orden del esquema: `NOMBRE`, `DNI`, `FECHA DE CESE`, `CAUSA DE SALIDA`, `CARGO`, `NOMBRE DE CC`. Las que no existan en Silver se omiten con advertencia (no es error). El resto de columnas Silver se descartan. (`step2_gold.py:103-115`)
- **ER-GLD-03 — Tipado.** Por columna base según `type`: `string`→texto, `integer`→entero 64, `float`→real 64, `boolean`, `date`. Desconocido → texto. Todos los casts son no estrictos (fallo → `null`). Para `date`: si ya es Date se deja; si es texto se parsea `YYYY-MM-DD HH:MM:SS` no estricto; otro tipo → cast directo. (`step2_gold.py:118-165`)
- **ER-GLD-04 — Derivadas** (solo si existe `FECHA DE CESE`) (`step2_gold.py:170-180`; `src/utils/month_name.py:6-48`):
  - `AÑO = year(FECHA DE CESE)`
  - `MES = month(FECHA DE CESE)` (1–12)
  - `NOMBRE_MES` = nombre del mes en **inglés** (`January` … `December`) a partir de `MES`; `null` si `MES` nulo o fuera de rango. Se inserta inmediatamente después de `MES`.
  - Si `FECHA DE CESE` es nula, las tres derivadas son nulas.
  - El bloque `columnas_derivadas` del YAML es solo documental: el código implementa estas reglas de forma fija.
- **ER-GLD-05 — Filtros del esquema.** Para cada elemento de `filtros` (si la columna existe): `NOT_EQUAL` → conservar `col ≠ valor`; `EQUAL` → `col = valor`; `IS_NOT_NULL`; `IS_NULL`; otra condición → se ignora. Las comparaciones son exactas y sensibles a mayúsculas; con `NOT_EQUAL`/`EQUAL` las filas con valor nulo se **eliminan** (lógica trivalente). (`step2_gold.py:183-210`)
- **ER-GLD-06 — Filtro vigente.** `CARGO <> 'STUDENT INTERNAL'` (excluir practicantes). Como `CARGO` vacío llega como `""` desde Silver, se conserva. (`assets/esquemas/esquema_examen_retiro.yaml:76-80`)
- **ER-GLD-07 — Restricciones no aplicadas.** `constraints.primary_key` (`DNI` + `FECHA DE CESE`), `nullable: false`, `min_value`/`max_value` de `MES` y `transformations` **no se validan ni aplican** en código; no hay deduplicación. (`assets/esquemas/esquema_examen_retiro.yaml:81-94`; sin referencias en `src/modules/examen_retiro/`)

**Esquema Gold resultante:** `NOMBRE`, `DNI`, `FECHA DE CESE` (Date), `CAUSA DE SALIDA`, `CARGO`, `NOMBRE DE CC`, `AÑO`, `MES`, `NOMBRE_MES`.

## 5. Reglas de negocio / joins (`step3_join` + `query_cc_join.sql`)

- **ER-JOIN-01 — Registro de tablas.** Gold → `examenes`; CC actual → `cc_actual`; CC antiguo → `cc_old`. SQL ejecutado en DuckDB en memoria. (`step3_join.py:194-201`)
- **ER-JOIN-02 — Catálogo unificado** (`query_cc_join.sql:32-69`):
  ```sql
  combined = SELECT CC,"NOMBRE CC","CATEGORIA CC",GERENCIA, 2 AS fuente FROM cc_actual
             UNION ALL
             SELECT CC,"NOMBRE CC","CATEGORIA CC",GERENCIA, 1 AS fuente FROM cc_old
  cc_unificado = DISTINCT CC, FIRST_VALUE(col) OVER (PARTITION BY CC ORDER BY fuente DESC) por cada col
  ```
  Precedencia: **si el código existe en `CC_ACTUAL`, todos sus atributos vienen de `CC_ACTUAL`**, aunque sean nulos (FIRST_VALUE no salta nulos); `CC_OLD` solo aporta códigos ausentes en el actual. Resultado: una fila por `CC`. Si un mismo catálogo tiene códigos duplicados con atributos distintos, el elegido no es determinista (inferido).
- **ER-JOIN-03 — Join.** `examenes LEFT JOIN cc_unificado ON examenes."NOMBRE DE CC" = cc_unificado.CC`. Igualdad exacta de texto (sensible a mayúsculas; sin trim en el catálogo). La columna de origen se llama `NOMBRE DE CC` pero contiene el **código** de CC. Se conservan todas las filas de Gold; no hay multiplicación de filas (catálogo único por CC). (`query_cc_join.sql:101-103`)
- **ER-JOIN-04 — `status_match`** (`query_cc_join.sql:95-99`):
  1. `NOMBRE DE CC = ''` → `'SIN_CODIGO'`
  2. sin match (`cc.CC` nulo) → `'CODIGO_NO_ENCONTRADO'` (incluye código `null`, caso posible si la columna entera venía vacía; inferido)
  3. si no → `'OK'`
- **ER-JOIN-05 — Proyección y orden.** Columnas: `NOMBRE`, `DNI`, `FECHA DE CESE`, `CAUSA DE SALIDA`, `CARGO`, `codigo_cc_original` (= `NOMBRE DE CC`), `AÑO`, `MES`, `NOMBRE_MES`, `codigo_cc`, `nombre_cc_completo`, `categoria_cc`, `gerencia`, `status_match`. `ORDER BY NOMBRE`. Si Gold no tiene alguna de estas columnas, el SQL falla → error del Step 3. (`query_cc_join.sql:76-105`)
- **ER-JOIN-06 — Estadísticas.** Conteo por `status_match` (`total`, `ok`, `sin_codigo`, `no_encontrado`); solo informativo. Con 0 filas, el cálculo de porcentaje en el log del worker divide por cero (ver §8). (`step3_join.py:213-259`; `ui/worker.py:529`)

## 6. Salidas

Carpeta de trabajo = carpeta del Excel Bronze (`ui/widget.py:335`).

| Capa | Ruta | Versionado |
|---|---|---|
| Silver | `silver/examenes_retiro.parquet` | Sin timestamp; se sobrescribe (`step1_clean.py:225-237`) |
| Gold | `gold/examenes_retiro_gold.parquet` | Se sobrescribe (`step2_gold.py:251-257`) |
| Gold histórico | `gold/historico/examenes_retiro_gold_DD.MM.YYYY_HH.MM.SS.parquet` | Nuevo en cada ejecución (`step2_gold.py:240-244, 278-284`) |
| Gold enriquecido | `gold/examenes_retiro_gold_enriquecido.parquet` (misma carpeta que el Gold) | Se sobrescribe (`step3_join.py:284, 297-303`) |
| Enriquecido histórico | `gold/historico/examenes_retiro_gold_enriquecido_DD.MM.YYYY_HH.MM.SS.parquet` | Nuevo en cada ejecución (`step3_join.py:287-291, 318-324`) |
| Excel (opcional) | Mismo nombre con `.xlsx` para cada parquet de Gold/enriquecido (actual e histórico) | Solo si "Generar Excel adicional en Gold" está marcado (default no) (`ui/widget.py:136-137`) |

- **ER-OUT-01** — Parquet con compresión snappy. Timestamp con hora local, formato `%d.%m.%Y_%H.%M.%S`.
- **ER-OUT-02** — En el Excel de Gold (step 2) las columnas Date se escriben como texto `YYYY-MM-DD`; en el Excel del enriquecido (step 3) se escriben con su tipo nativo. (`step2_gold.py:260-265`; `step3_join.py:306-310`)
- **ER-OUT-03** — El histórico nunca se purga.

## 7. Validaciones y errores

| Situación | Resultado del worker | Fuente |
|---|---|---|
| Preflight falla | `success=false`, etapa "Preflight / Validate Source" | `ui/worker.py:616-629` |
| Hoja `DATA` ausente / error de lectura en Step 1 | `success=false`, "Error en extracción Bronze→Silver" | `ui/worker.py:253-267` |
| Excel sin filas | `success=false` | `ui/worker.py:189-201` |
| Esquema no encontrado | **`success=true`** con solo Silver (Step 2 y 3 omitidos) | `ui/worker.py:284-288, 599-605` |
| Excepción en Step 2 | Se registra, Step 3 se omite; **`success=true`** "Procesamiento Silver completado" | `ui/worker.py:385-398, 412-416, 599-605` |
| Query SQL no encontrada / excepción en Step 3 | `success=false`, "No se pudo generar examenes_retiro_gold_enriquecido.parquet" | `ui/worker.py:437-468, 549-574` |
| Columnas base faltantes en Silver | Advertencia en Step 2; luego falla el SQL del Step 3 | `step2_gold.py:109-111` |

## 8. Discrepancias

1. **Fallo de Step 2 reportado como éxito.** El worker solo marca error si falla el Step 3; si el Step 2 falla (o no hay esquema) el Step 3 se salta con *warning* y el resultado final es `success=true`. Test `tests/test_gold_export.py:158-251` solo cubre el fallo del Step 3. (`ui/worker.py:385-398, 568-605`)
2. **Ubicación de la query.** El worker carga `assets/queries/query_cc_join.sql` vía rutas de recursos; el modo standalone de `step3_join` busca una carpeta `queries/` en el cwd o en ancestros del script, que no existe en el repo (está en `assets/queries/`) → el standalone fallaría salvo que se ejecute desde `assets/` (inferido). (`ui/worker.py:440-442` vs `step3_join.py:28-58`)
3. **Esquema.** Worker: resolución por recurso (`resolve_structured_path`). Standalone step2: rutas relativas al cwd + diálogo manual. (`ui/worker.py:280-282` vs `step2_gold.py:35-44, 316-336`)
4. **Excel.** El worker pasa `export_excel` desde la UI; los `main()` standalone no lo pasan → nunca generan Excel aunque sus docstrings digan "parquet y Excel". (`step2_gold.py:376-379`; `step3_join.py:405`)
5. **Preflight vs extracción.** Preflight tolera hasta 2 celdas vacías entre cabeceras y compara sin mayúsculas; step1 corta en la primera cabecera vacía y step2 exige nombres exactos. Una cabecera `Nombre` o una columna tras un hueco pasa el preflight y desaparece en Silver/Gold. (`validate_source.py:339-373` vs `step1_clean.py:70-75`, `step2_gold.py:109-115`)
6. **YAML de esquema vs código.** `columnas_derivadas`, `constraints` (PK `DNI`+`FECHA DE CESE`), `nullable`, `min_value/max_value`, `transformations` son documentales; no se aplican. El preflight sí usa `nullable: false` (vía `extract: schema_non_nullable`), pero la lista explícita `required_columns` del contrato tiene precedencia y lo anula. (`assets/validate_source/examen_retiro.yaml:19-28`; `validate_source.py:254-256`)
7. **Formatos de timestamp distintos** entre módulos: aquí `DD.MM.YYYY_HH.MM.SS`; en el catálogo CC del módulo `bd`, `YYYYMMDD_HHMMSS`. (`step2_gold.py:244`; `step1_5_centrosdecosto.py:175`)
8. **División por cero.** Si el Gold queda vacío (p.ej. todos `STUDENT INTERNAL`), el log del Step 3 calcula `ok/total` con `total=0` → excepción **después** de haber escrito los archivos → el worker reporta fallo aunque los parquets existan. (`ui/worker.py:498-529`; también `step3_join.py:420-427`)
9. **Comentario de la SQL** con "resultados esperados" (485 registros, ~37,7 % OK) es una foto de un dataset concreto, no una regla. (`query_cc_join.sql:15-19`)

## 9. Ambigüedades y preguntas abiertas

1. ¿Quién mantiene `CC_OLD` y con qué estructura? No hay módulo que lo genere.
2. ¿Debe un atributo nulo en `CC_ACTUAL` caer a `CC_OLD` (coalesce por columna)? Hoy no lo hace.
3. ¿Debe normalizarse el código de CC (trim/mayúsculas/ceros) antes del join? Hoy es igualdad exacta.
4. ¿Deben fallar (o reportarse) las fechas de cese no parseables? Hoy quedan `null` silenciosamente y las derivadas también.
5. ¿Se debe deduplicar por la PK declarada (`DNI` + `FECHA DE CESE`)?
6. El filtro `STUDENT INTERNAL` es exacto y sensible a mayúsculas; ¿hay variantes en los datos reales?
7. ¿Qué columna es la A de la hoja `DATA` (determina el fin de datos)? No está documentado; si está vacía en una fila intermedia se truncan los datos.
8. ¿Es intencional que cualquier texto que empiece por `#` se convierta en nulo?
9. ¿`NOMBRE_MES` en inglés es requisito del consumidor (Power BI)?

## Decisiones del rework (Plan 8)

Reporte «Exámenes médicos de salida» (`examenes_salida`), hecho con el molde de reportes
([config/README.md](../../config/README.md)).

| Pregunta / regla | Decisión |
|---|---|
| §9.1 `CC_OLD` | Se descarta: solo se cruza con `centros_costo` de la base de personal de la misma ejecución. |
| §9.2 coalesce por columna | No aplica (un solo catálogo). |
| §9.3 normalizar el código | Se compara con trim y mayúsculas. |
| §9.4 fechas no parseables | Las valida el contrato con mensajes por celda. |
| §9.6 filtro de practicantes | Cargo comparado normalizado contra `examenes_salida.cargo_practicante` de `parametros.yaml`. |
| §9.7 fin de datos | El del lector común (TR-D2, TR-D21). |
| §9.9 NOMBRE_MES en inglés | En español. |
| ER-JOIN-04 `status_match` | `estado_cruce`: `ok`, `sin_codigo`, `codigo_no_encontrado` y, si no se cargó la base, `sin_catalogo` con el aviso `EXS-01`. |
| Vigencia en Inicio | Sin fecha en el nombre: «Al día» en cuanto hay un archivo guardado. |

## 10. Referencias

- `src/modules/examen_retiro/steps/step1_clean.py:42-132, 135-209, 212-241`
- `src/modules/examen_retiro/steps/step2_gold.py:35-44, 88-214, 217-303, 306-421`
- `src/modules/examen_retiro/steps/step3_join.py:28-58, 86-128, 131-210, 213-259, 262-343, 346-455`
- `src/modules/examen_retiro/ui/worker.py:47-80, 136-642`
- `src/modules/examen_retiro/ui/widget.py:136-137, 179-260, 317-344`
- `src/utils/month_name.py:6-48`; `src/utils/gold_export.py:7-27`
- `assets/validate_source/examen_retiro.yaml:1-28`
- `assets/esquemas/esquema_examen_retiro.yaml:1-102`
- `assets/queries/query_cc_join.sql:32-105`
- `assets/esquemas/esquema_cc.yaml:1-11`; `src/modules/bd/steps/step1_5_centrosdecosto.py:113-205`; `src/modules/bd/ui/worker.py:341-377`
- `tests/test_gold_export.py:97-251`
