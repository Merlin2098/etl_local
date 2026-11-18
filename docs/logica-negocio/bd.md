# Lógica de negocio — Dominio `bd` (Base de Datos de personal)

> Especificación agnóstica de lenguaje extraída del código Python legado
> (`HR_Data_Integration_Pipeline`). Rutas relativas a la raíz de ese repo.
> El nombre real del cliente se reemplaza por `<CLIENTE>`.
> "(inferido)" = deducido del comportamiento de librerías/código, no explícito.

---

## 1. Propósito del dominio

Procesar el maestro de personal ("BD") que RR. HH. exporta periódicamente en Excel
para obtener: (a) una copia limpia de todas las columnas (Silver), (b) un catálogo
único de Centros de Costo, (c) dos tablas Gold separadas —empleados y practicantes—
con 14 columnas de negocio, y (d) la tabla de empleados enriquecida con alertas
de jubilación (65/70 años) y de vencimiento de contratos temporales.

Existen dos implementaciones: los scripts `steps/` (CLI con diálogos Tk, ejecución
paso a paso) y `ui/worker.py` (lo que realmente ejecuta la app de escritorio,
pipeline completo en una sola corrida). **El worker reimplementa la lógica; no
llama a los steps.** Donde difieren se indica en cada regla y en la sección 8.

Orden de ejecución del worker: Preflight → Step 1 (Bronze→Silver) → Step 1.5
(Centros de Costo) → Step 2 (Silver→Gold) → Step 3 (Flags)
(`src/modules/bd/ui/worker.py:105-153`).

---

## 2. Fuentes (Bronze)

| ID | Regla | Fuente |
|---|---|---|
| BD-BRZ-01 | Un único archivo Excel por corrida. La UI permite seleccionar solo 1 archivo (`multiple=False`) con filtro `*.xlsx *.xlsm *.xls`. El worker usa solo `archivos[0]` y falla con "No se proporcionó archivo Excel" si la lista está vacía. | `src/modules/bd/ui/widget.py:50-77`, `src/modules/bd/ui/worker.py:93-96` |
| BD-BRZ-02 | **Patrón de nombre** (case-insensitive): `^BD\.\s*(?P<fecha>\d{2}\.\d{2}\.\d{4})\.\.(xlsx\|xlsm\|xls)$`. Es decir: `BD.` + espacios opcionales + fecha `DD.MM.YYYY` + **dos puntos** + extensión. Ejemplo válido: `BD. 21.12.2025..xlsx`. Ejemplo inválido: `Base de Datos 21.12.2025.xlsm`. | `src/utils/bd_document_date.py:14-16`, `assets/validate_source/bd.yaml:3`, `tests/test_bd_document_date.py:23-35` |
| BD-BRZ-03 | La fecha del nombre debe ser una fecha de calendario válida (`DD.MM.YYYY`); p. ej. `31.02.2025` se rechaza con "Invalid document date in filename: 31.02.2025". | `src/utils/bd_document_date.py:35-42`, `tests/test_bd_document_date.py:38-44` |
| BD-BRZ-04 | Hoja leída por el procesamiento: la **hoja activa** del libro (la que estaba seleccionada al guardar), no una hoja por nombre. El contrato de preflight exige que exista una hoja llamada `<CLIENTE>`, pero no se verifica que sea la activa. | `src/modules/bd/steps/step1_capasilver.py:67-68`, `src/modules/bd/ui/worker.py:243-244`, `assets/validate_source/bd.yaml:9-14` |
| BD-BRZ-05 | Encabezados en la **fila 10** (1-based), comenzando en columna A. Datos desde la **fila 11**. Se leen valores calculados (no fórmulas). | `src/modules/bd/steps/step1_capasilver.py:49-51,67`, `src/modules/bd/ui/worker.py:243-247`, `assets/esquemas/esquema_bd.yaml:6-7` |
| BD-BRZ-06 | **Lectura de encabezados (step):** se recorre la fila 10 desde A y se **detiene en la primera celda vacía/falsy**; cada encabezado se convierte a texto (sin `trim`). **(worker):** se toman todas las celdas no nulas de la fila 10, saltando huecos (ver BD-DIF-01). | `src/modules/bd/steps/step1_capasilver.py:73-78`, `src/modules/bd/ui/worker.py:250` |
| BD-BRZ-07 | **Columna clave para fin de datos (solo step):** primer encabezado cuyo texto en mayúsculas contiene `NUMERO` **y** `DOC`. Si no existe, error "No se encontró la columna 'NUMERO DE DOC' en los encabezados" (aborta). Nota: `NÚMERO` con tilde no coincide. | `src/modules/bd/steps/step1_capasilver.py:82-91` |
| BD-BRZ-08 | **Fin de datos (step):** se leen filas desde la 11 (solo las primeras N columnas, N = nº de encabezados) y se **detiene en la primera fila cuyo NUMERO DE DOC es nulo o vacío tras trim**. Todo lo que haya debajo se ignora. | `src/modules/bd/steps/step1_capasilver.py:103-111` |
| BD-BRZ-09 | **Fin de datos (worker):** no hay corte. Se leen todas las filas desde la 11 hasta el final de la hoja y se **conserva toda fila con al menos una celda no nula** en las primeras N columnas; las filas totalmente vacías se saltan (no detienen). No exige NUMERO DE DOC. | `src/modules/bd/ui/worker.py:254-257` |
| BD-BRZ-10 | **Columnas requeridas en origen (13)**, comparadas en preflight: `NUMERO DE DOC`, `CODIGO SAP2`, `NOMBRE COMPLETOS`, `GERENCIA`, `SEXO`, `SEDE3`, `WHITE COLLAR / BLUE COLLAR`, `Modalidad de Contrato`, `Fecha de Termino`, `SERVICIO`, `REGIMEN DE TRABAJO`, `FECH_INGR.`, `FECHA DE NAC.`. La lista explícita del contrato tiene prioridad sobre la extracción desde el esquema (por eso `FECHA_DOCUMENTO`, que es `required: true` en el esquema Gold pero no existe en el Excel, no se exige). | `assets/validate_source/bd.yaml:20-36`, `src/utils/validate_source.py:246-256` |
| BD-BRZ-11 | Columnas adicionales **necesarias pero no validadas en preflight**: `CC`, `NOMBRE CC`, `CATEGORIA CC` (y `GERENCIA`) para el Step 1.5. Si faltan, el fallo ocurre después de haber escrito Silver. | `assets/esquemas/esquema_cc.yaml:1-5`, `src/modules/bd/ui/worker.py:334-337` |

### 2.1 Contrato de preflight (solo worker)

Se ejecuta antes de cualquier transformación; si hay **cualquier** error, aborta
todo el ETL (`worker.py:106-112`, `validate_source.py:55-57`). Chequeos en orden
(`src/utils/validate_source.py:75-189`):

| ID | Chequeo | Comportamiento |
|---|---|---|
| BD-BRZ-12 | El archivo existe. | Si no: error y fin del reporte. (`validate_source.py:89-91`) |
| BD-BRZ-13 | Extensión ∈ {`.xlsx`, `.xlsm`, `.xls`} (case-insensitive). | Si no: error y fin del reporte. (`validate_source.py:93-103`, `bd.yaml:4-8`) |
| BD-BRZ-14 | Nombre cumple la regex BD-BRZ-02. | Error, pero continúa con los demás chequeos. (`validate_source.py:105-112`) |
| BD-BRZ-15 | Solo si la regex coincidió: la fecha del nombre es válida (BD-BRZ-03). | Error, continúa. (`validate_source.py:114-120`) |
| BD-BRZ-16 | El libro se puede abrir. | Si no: "Unable to open Excel file: …" y fin. (`validate_source.py:122-128`). (inferido) los `.xls` binarios antiguos no son legibles por el lector usado, por lo que en la práctica fallarían aquí. |
| BD-BRZ-17 | Existe la hoja requerida `<CLIENTE>`. `enforce_allowed_sheets: false` → hojas extra se permiten. | Error "Missing sheet '…'". (`validate_source.py:134-146`, `bd.yaml:9-14`) |
| BD-BRZ-18 | Celda A10 de esa hoja no vacía (tras trim). | Error "Expected header at row 10 col A … but found empty/shifted". (`validate_source.py:153-166`) |
| BD-BRZ-19 | Lectura de encabezados para validación: desde A, hasta 250 columnas; las vacías se saltan; se detiene tras **3 vacías consecutivas** una vez hallado al menos un encabezado. Cada valor se `trim`ea. | (`validate_source.py:339-360`) |
| BD-BRZ-20 | Todas las columnas de BD-BRZ-10 presentes. Comparación **normalizada**: trim + colapsar espacios internos + mayúsculas. | Error "Missing required columns in sheet '…': [...]". (`validate_source.py:175-185,363-373`) |

Mensaje de fallo agregado: `Preflight validation failed for 'bd' (BD Bronze)` seguido
de una línea `- <archivo>: <mensaje>` por error (`validate_source.py:48-53`).

---

## 3. Reglas Bronze → Silver

| ID | Regla | Fuente |
|---|---|---|
| BD-SLV-01 | Silver conserva **todas** las columnas del encabezado, en el orden del Excel, sin renombrar. | `src/modules/bd/steps/step1_capasilver.py:166-170`, `src/modules/bd/ui/worker.py:268-277` |
| BD-SLV-02 | **Todos los valores se guardan como texto** (o nulo). Celda vacía → nulo. | `step1_capasilver.py:172-174`, `worker.py:273-274` |
| BD-SLV-03 | **(solo step)** Celdas de texto: se aplica `trim`. Si tras el trim coinciden con `^\d{1,2}/\d{1,2}/\d{4}$` y son fecha válida `DD/MM/YYYY`, se convierten a fecha-hora. Si no son válidas, quedan como el texto ya recortado. | `step1_capasilver.py:93-127` |
| BD-SLV-04 | **(step)** Valores fecha-hora (nativos del Excel o convertidos por BD-SLV-03) se serializan como `YYYY-MM-DD HH:MM:SS` (p. ej. `2020-01-01 00:00:00`). **(worker)** se serializan con la representación textual por defecto del lenguaje, que coincide con `YYYY-MM-DD HH:MM:SS` salvo si hay microsegundos (inferido) — y los textos `DD/MM/YYYY` **no** se convierten. | `step1_capasilver.py:154-164`, `worker.py:274` |
| BD-SLV-05 | Números: se convierten a su representación textual por defecto (un entero `12345678` → `"12345678"`; un decimal `123.0` → `"123.0"`) (inferido de `str()`). Ceros a la izquierda de documentos almacenados como número se pierden en el origen. | `step1_capasilver.py:174`, `worker.py:274` |
| BD-SLV-06 | **(solo step)** Los textos literales `"None"`, `"nan"`, `"NaT"` se convierten en nulo (incluso si eran valores reales de la celda). El string vacío `""` **no** se convierte en nulo. | `step1_capasilver.py:176-180` |
| BD-SLV-07 | No hay filtros, deduplicación ni validación de tipos en Silver. Las filas duplicadas por NUMERO DE DOC se mantienen. | `step1_capasilver.py` (ausencia), `worker.py:190-239` |
| BD-SLV-08 | **Columna derivada `FECHA_DOCUMENTO`**: constante para todas las filas, igual a la fecha del nombre de archivo (BD-BRZ-02) normalizada a `YYYY-MM-DD` (texto). Se agrega **al final**. Si el nombre no cumple el patrón, el paso aborta con "Filename does not match expected pattern 'BD. DD.MM.YYYY..xlsx\|xlsm\|xls'". | `src/utils/bd_document_date.py:12,22-59,71-89`, `step1_capasilver.py:262-263`, `worker.py:212-213`, `tests/test_bd_document_date.py:47-55` |
| BD-SLV-09 | Encabezados duplicados (inferido): en el step el último gana silenciosamente (se pierde la columna anterior); en el worker se acumulan valores en la misma lista y la construcción de la tabla falla por longitudes distintas. | `step1_capasilver.py:167-170`, `worker.py:268-277` |

---

## 4. Reglas Silver → Gold

| ID | Regla | Fuente |
|---|---|---|
| BD-GLD-01 | **Selección de columnas**: se conservan solo las 14 columnas del esquema `BD_Tipo2_Gold` v1.0: las 13 de BD-BRZ-10 + `FECHA_DOCUMENTO`. Comparación de nombres **exacta** (case- y espacio-sensible). | `assets/esquemas/esquema_bd.yaml:1-68`, `step2_capagold.py:96-120`, `worker.py:464-477` |
| BD-GLD-02 | El **orden** de las columnas Gold es el de Silver (orden del Excel), no el del esquema. `FECHA_DOCUMENTO` queda última. | `step2_capagold.py:103-104`, `worker.py:467` |
| BD-GLD-03 | Columnas del esquema ausentes en Silver: **solo advertencia** ("⚠️ Columnas faltantes: …"); Gold se genera sin ellas. Excepción: `Modalidad de Contrato` (BD-GLD-06). | `step2_capagold.py:114-117`, `worker.py:471-473` |
| BD-GLD-04 | No hay renombres. Columnas con `type: string` quedan como texto. | `esquema_bd.yaml:10-68` |
| BD-GLD-05 | **Fechas** (`type: date`): `Fecha de Termino`, `FECH_INGR.`, `FECHA DE NAC.`. Se parsea el texto con formato exacto `YYYY-MM-DD HH:MM:SS` en modo no estricto y se trunca a fecha (sin hora). Valor que no cumple el formato → **nulo** (sin error). Consecuencia: `YYYY-MM-DD` sin hora, `DD/MM/YYYY` no convertido, etc. quedan nulos. El campo `format: YYYY-MM-DD` del esquema es documental y no se usa. | `step2_capagold.py:131-161`, `worker.py:479-498`, `esquema_bd.yaml:42-64` |
| BD-GLD-06 | **Split empleados / practicantes** por `Modalidad de Contrato`: **practicante** si el texto **contiene** la subcadena `TERMINO DE CONVENIO` (sensible a mayúsculas, sin trim); **empleado** si no la contiene. Si la columna no existe → error que aborta ("La columna 'Modalidad de Contrato' no existe…" / "Columna 'Modalidad de Contrato' no encontrada"). | `step2_capagold.py:164-195`, `worker.py:500-516`, `tests/test_bd_document_date.py:58-107` |
| BD-GLD-07 | **Modalidad nula**: la condición evalúa a nulo en ambos filtros, por lo que la fila **no aparece ni en empleados ni en practicantes** (se pierde) (inferido de la semántica de filtrado con nulos). | `step2_capagold.py:180-187`, `worker.py:505-511` |
| BD-GLD-08 | No hay joins, deduplicación ni filtros adicionales en Gold. `FECHA_DOCUMENTO` se propaga a ambas salidas. | `step2_capagold.py:393-413`, `tests/test_bd_document_date.py:103-107` |

---

## 5. Reglas de negocio / flags

### 5.1 Centros de Costo (Step 1.5)

| ID | Regla | Fuente |
|---|---|---|
| BD-CC-01 | Origen: Silver completo (incluye empleados y practicantes, y en el worker posibles filas sin documento). | `step1_5_centrosdecosto.py:113-146`, `worker.py:317` |
| BD-CC-02 | Columnas: `CC`, `NOMBRE CC`, `CATEGORIA CC`, `GERENCIA` (en ese orden), todas texto. Si falta alguna → error que aborta: "Columnas faltantes…". La sección `tipos_datos` del esquema no se aplica (no hay casteo). | `assets/esquemas/esquema_cc.yaml:1-11`, `step1_5_centrosdecosto.py:127-132`, `worker.py:334-341` |
| BD-CC-03 | **Deduplicación** por `CC` conservando la **primera** ocurrencia (orden del archivo). Los demás atributos (`NOMBRE CC`, etc.) son los de esa primera fila; inconsistencias entre filas no se reportan. Un `CC` nulo cuenta como un valor más (queda una fila con CC nulo) (inferido). | `esquema_cc.yaml:12`, `step1_5_centrosdecosto.py:136-138`, `worker.py:344` |
| BD-CC-04 | Orden de salida: ascendente por `CC` como **texto** (lexicográfico: `"1000"` < `"200"`); nulos primero (inferido del default de la librería). | `step1_5_centrosdecosto.py:143-144`, `worker.py:345` |

### 5.2 Flags de empleados (Step 3)

Lógica en `assets/queries/queries_flags_gold.sql` (DuckDB). Entrada: solo
`bd_empleados_gold` (practicantes no reciben flags). Fecha de referencia:
`CURRENT_DATE` = fecha del sistema al ejecutar (no `FECHA_DOCUMENTO`).

| ID | Regla | Fuente |
|---|---|---|
| BD-FLG-01 | **Columnas requeridas**: `NUMERO DE DOC`, `FECHA DE NAC.`, `FECH_INGR.`, `Fecha de Termino`, `Modalidad de Contrato`. El step valida y aborta con "El archivo no contiene las columnas requeridas"; el worker no valida (fallará la consulta). | `step3_flags_empleados.py:131-161,422-426`, `queries_flags_gold.sql:17-18` |
| BD-FLG-02 | **Fecha fin efectiva** = `Fecha de Termino` si no es nula; si es nula, `CURRENT_DATE`. Nota: si `Fecha de Termino` es **futura**, la antigüedad se mide hasta esa fecha futura. | `queries_flags_gold.sql:27` |
| BD-FLG-03 | **`años_servicio`** (intermedio, no se exporta) = `días_entre(FECH_INGR., fecha_fin_efectiva) / 365.25` (decimal). Si `FECH_INGR.` es nula → nulo. | `queries_flags_gold.sql:27` |
| BD-FLG-04 | **`tiempo_servicio_texto`** (texto) = `"<A> años, <M> meses"` con `A = floor(años_servicio)` y `M = floor((años_servicio − A) × 12)`, ambos enteros. Ej.: 1000 días → 2.7379 años → `"2 años, 8 meses"`. Con antigüedad negativa (término antes de ingreso) floor da valores negativos (p. ej. −0.3 → `"-1 años, 8 meses"`) (inferido). Con `FECH_INGR.` nula, la concatenación de DuckDB ignora nulos y produciría `" años,  meses"` (inferido). | `queries_flags_gold.sql:28-33` |
| BD-FLG-05 | **Edad por año calendario** (no por fecha exacta): `año_nac = YEAR(FECHA DE NAC.)`, `año_actual = YEAR(CURRENT_DATE)`. | `queries_flags_gold.sql:25-26` |
| BD-FLG-06 | `cumple_65_esteaño = (año_nac = año_actual − 65)`; `cumple_65_proximoaño = (año_nac = año_actual + 1 − 65)`. | `queries_flags_gold.sql:43-44` |
| BD-FLG-07 | `cumple_70_esteaño = (año_nac = año_actual − 70)`; `cumple_70_proximoaño = (año_nac = año_actual + 1 − 70)`. Las flags de edad aplican a toda modalidad. Con `FECHA DE NAC.` nula → las 4 flags son **nulo** (no FALSE). | `queries_flags_gold.sql:46-47` |
| BD-FLG-08 | `alerta_contrato_obra = (Modalidad = 'CONTRATO POR OBRA O SERVICIO ESPECIFICO' AND años_servicio >= 4.5)`. Igualdad exacta (mayúsculas, sin trim). | `queries_flags_gold.sql:57-58` |
| BD-FLG-09 | `alerta_contrato_incremento = (Modalidad = 'CONTRATO POR INCREMENTO DE ACTIVIDAD' AND años_servicio >= 2.5)`. | `queries_flags_gold.sql:60-61` |
| BD-FLG-10 | Lógica trivalente en alertas: modalidad distinta → FALSE aunque `años_servicio` sea nulo; modalidad coincidente con `años_servicio` nulo → **nulo**; `Modalidad` nula → nulo (inferido de SQL estándar). | `queries_flags_gold.sql:57-61` |
| BD-FLG-11 | **Salida** = todas las columnas de entrada (orden original) + `tiempo_servicio_texto`, `cumple_65_esteaño`, `cumple_65_proximoaño`, `cumple_70_esteaño`, `cumple_70_proximoaño`, `alerta_contrato_obra`, `alerta_contrato_incremento` (en ese orden). `FECHA_DOCUMENTO` se conserva. | `queries_flags_gold.sql:67-75`, `tests/test_bd_document_date.py:110-126` |
| BD-FLG-12 | **Recombinación por `NUMERO DE DOC`** mediante 3 LEFT JOIN (`empleados` ↔ cálculos ↔ flags edad ↔ flags contrato). Consecuencias: (a) un documento repetido k veces genera k⁴ filas en la salida (k × k³) con combinaciones cruzadas de flags; (b) filas con `NUMERO DE DOC` nulo conservan sus datos pero todas las flags y `tiempo_servicio_texto` quedan nulas. La reimplementación debería calcular las flags **por fila** (ver Q-05). | `queries_flags_gold.sql:76-79` |
| BD-FLG-13 | `assets/esquemas/reglas_flags.yaml` **no es leído por ningún código** (confirmado: ninguna referencia en `.py`). Es documentación: sus umbrales coinciden con el SQL (obra 4.5 años/54 meses, máx. 5; incremento 2.5 años/30 meses, máx. 3; nulo de término → fecha actual; días/365.25). Lista "contratos excluidos" (`PLAZO INDETERMINADO`, `CONTRATO EXTRANJERO`, `TERMINO DE CONVENIO`) que en el SQL quedan excluidos implícitamente por no coincidir con las dos modalidades. | `assets/esquemas/reglas_flags.yaml:1-65` |

Pseudocódigo equivalente (por fila, comportamiento deseado sin el problema de BD-FLG-12):

```
fin   = coalesce(fecha_termino, hoy)
anios = dias(fech_ingr, fin) / 365.25          # nulo si fech_ingr nula
texto = f"{floor(anios)} años, {floor((anios - floor(anios))*12)} meses"
yn = year(fecha_nac); ya = year(hoy)
c65_este = yn == ya-65 ; c65_prox = yn == ya-64
c70_este = yn == ya-70 ; c70_prox = yn == ya-69
obra       = modalidad == 'CONTRATO POR OBRA O SERVICIO ESPECIFICO' and anios >= 4.5
incremento = modalidad == 'CONTRATO POR INCREMENTO DE ACTIVIDAD'    and anios >= 2.5
```

---

## 6. Salidas

Carpeta de trabajo (`<base>`): en el worker, la **carpeta del Excel seleccionado**
(`src/utils/ui/widgets/base_etl_widget.py:175-182`). En los steps CLI: Step 1 usa la
carpeta del Excel; Steps 1.5/2 la carpeta padre de `silver/`; Step 3 la carpeta del
parquet Gold elegido.

| ID | Archivo | Comportamiento | Fuente |
|---|---|---|---|
| BD-OUT-01 | `<base>/silver/bd_silver.parquet` | Sin timestamp, **se sobrescribe** en cada corrida; sin histórico. Compresión snappy. | `step1_capasilver.py:190-219`, `worker.py:285-295` |
| BD-OUT-02 | `<base>/centros_costo/actual/CC_ACTUAL.parquet` y `CC_ACTUAL.xlsx` | Se sobrescriben. El Excel se genera **siempre** (no depende de la opción de exportación). | `step1_5_centrosdecosto.py:166-194`, `worker.py:354-371` |
| BD-OUT-03 | `<base>/centros_costo/historico/CC_ACTUAL_YYYYMMDD_HHMMSS.parquet` (+ `.xlsx` solo en step) | Nueva versión por corrida. Timestamp `%Y%m%d_%H%M%S` (p. ej. `20260108_153012`). | `step1_5_centrosdecosto.py:175,196-213`, `worker.py:362,373-377` |
| BD-OUT-04 | `<base>/gold/bd_empleados_gold.parquet`, `<base>/gold/bd_practicantes_gold.parquet` | "Actual", se sobrescriben — **solo si el grupo tiene ≥1 fila**; si un grupo queda vacío, no se escribe nada y el archivo de una corrida anterior **permanece** (obsoleto). No existe subcarpeta `gold/actual/`: los actuales van directo en `gold/`. | `step2_capagold.py:217-336`, `worker.py:518-566` |
| BD-OUT-05 | `<base>/gold/historico/bd_empleados_gold_DD.MM.YYYY_HH.MM.SS.parquet`, idem `bd_practicantes_gold_…` | Timestamp `%d.%m.%Y_%H.%M.%S` (p. ej. `08.01.2026_15.30.12`) — **formato distinto** al de CC. | `step2_capagold.py:226,258-264`, `worker.py:526,543-565` |
| BD-OUT-06 | `.xlsx` de Gold empleados/practicantes | Solo si `export_excel_gold = true` (checkbox "Generar Excel adicional en Gold", desmarcado por defecto). Worker: solo el Excel "actual"; step: actual + histórico. En CLI la opción queda siempre en `false`. | `src/utils/gold_export.py:7-27`, `src/utils/ui/widgets/base_etl_widget.py:60-61`, `worker.py:535-559`, `step2_capagold.py:246-276,413` |
| BD-OUT-07 | `<base>/gold/bd_empleados_flags_gold.parquet` (+ `.xlsx` si export) y `<base>/gold/historico/bd_empleados_flags_gold_DD.MM.YYYY_HH.MM.SS.parquet` (+ `.xlsx` si export) | Actual se sobrescribe; histórico nuevo por corrida. Worker: Excel plano. Step: Excel con hoja `Empleados_Flags`, encabezado con estilo y ancho de columna = min(máx. longitud en primeras 100 filas + 2, 50). | `worker.py:639-681`, `step3_flags_empleados.py:262-377` |
| BD-OUT-08 | Dos corridas en el mismo segundo producen el mismo nombre histórico y la segunda sobrescribe a la primera (inferido). | `worker.py:362,526,643` |

Consumidor conocido: el módulo `examen_retiro` usa `CC_ACTUAL.parquet` como entrada
(`src/modules/examen_retiro/steps/step3_join.py:7,139-153`).

---

## 7. Validaciones y errores

| ID | Situación | Efecto |
|---|---|---|
| BD-ERR-01 | Preflight con algún error (sección 2.1). | Aborta todo; nada se escribe. Mensaje "Preflight validation failed for 'bd' (BD Bronze)" + lista. (`worker.py:109-110`) |
| BD-ERR-02 | Fallo en Step 1, 1.5 o 2 (nombre inválido, columnas CC faltantes, falta `Modalidad de Contrato`, esquema no encontrado: "No se encontró el esquema de centros de costo" / "No se encontró el esquema BD"). | Aborta el ETL con resultado de error que indica la etapa (`current_stage`). **No hay rollback**: los archivos de pasos previos ya escritos permanecen. (`worker.py:68-81,321-325,421-424`) |
| BD-ERR-03 | Fallo en Step 3 (falta query SQL "No se encontró queries_flags_gold.sql", falta `gold/bd_empleados_gold.parquet`, columnas faltantes, DuckDB no instalado). | **Solo advertencia**: el ETL termina con `success = true` y el resumen muestra "⚠️ Flags: Error (ver log)". (`worker.py:137-153,168-173`) |
| BD-ERR-04 | Columnas del esquema Gold ausentes. | Advertencia; continúa. (BD-GLD-03) |
| BD-ERR-05 | Fechas no parseables en Gold. | Silencioso: quedan nulas. (BD-GLD-05) |
| BD-ERR-06 | Mensaje final de éxito (worker): conteos de Silver, CC únicos, empleados, practicantes, registros con flags y tiempo total. | (`worker.py:160-179`) |
| BD-ERR-07 | Steps CLI: cualquier excepción se imprime y el script termina sin propagar (salvo Step 1.5, que re-lanza). Cancelar un diálogo de selección termina el proceso sin cambios. | (`step1_capasilver.py:232-296`, `step1_5_centrosdecosto.py:302-307`, `step2_capagold.py:353-455`, `step3_flags_empleados.py:390-456`) |

---

## 8. Discrepancias step vs worker / config vs código

| ID | Discrepancia | Fuente |
|---|---|---|
| BD-DIF-01 | **Encabezados:** step corta en la primera celda vacía de la fila 10; worker omite las vacías y sigue. Si hay un hueco en el encabezado, el worker **desalinea** datos y nombres (los valores se asignan por posición sobre una lista de encabezados sin huecos) y además incluye columnas a la derecha del hueco. El worker tampoco convierte el encabezado a texto. | `step1_capasilver.py:73-78` vs `worker.py:250,255-257` |
| BD-DIF-02 | **Fin de datos:** step se detiene en el primer NUMERO DE DOC vacío; worker lee hasta el final y solo salta filas totalmente vacías → puede incluir filas sin documento, totales, notas al pie, o filas que el step descartaría tras un hueco. El worker no exige la columna NUMERO DE DOC. | `step1_capasilver.py:82-111` vs `worker.py:254-257` |
| BD-DIF-03 | **Limpieza de texto:** step hace trim de todas las celdas texto, convierte `DD/MM/YYYY` a fecha y nulifica `"None"/"nan"/"NaT"`; el worker no hace nada de esto. Efecto en Gold: con el worker, las fechas escritas como texto `DD/MM/YYYY` terminan **nulas**, y textos con espacios sobrantes (p. ej. `Modalidad de Contrato`) pueden no coincidir en BD-FLG-08/09. | `step1_capasilver.py:113-127,176-180` vs `worker.py:265-277` |
| BD-DIF-04 | **Excel histórico de CC:** step escribe `CC_ACTUAL_<ts>.xlsx` en histórico; el worker no. | `step1_5_centrosdecosto.py:209-213` vs `worker.py:373-377` |
| BD-DIF-05 | **Excel histórico de Gold (empleados/practicantes):** step lo escribe si la exportación está activa; el worker nunca. | `step2_capagold.py:267-276,320-329` vs `worker.py:542-565` |
| BD-DIF-06 | **Entrada del Step 3:** step usa el parquet elegido por el usuario; el worker lee siempre `<base>/gold/bd_empleados_gold.parquet`. Si en la corrida actual no hubo empleados (BD-OUT-04), el worker calcula flags sobre el archivo **obsoleto** de una corrida anterior sin advertir. | `step3_flags_empleados.py:388-415` vs `worker.py:601-609` |
| BD-DIF-07 | **Validación previa de flags:** step valida las 5 columnas requeridas y aborta; worker no valida. | `step3_flags_empleados.py:422-426` vs `worker.py:572-633` |
| BD-DIF-08 | **Formato Excel de flags:** step hoja `Empleados_Flags` con estilo; worker escritura plana por defecto de la librería. | `step3_flags_empleados.py:302-343` vs `worker.py:654-658` |
| BD-DIF-09 | **Parámetro muerto:** el worker pasa `CC_ACTUAL` al Step 3 y su docstring dice que "requiere archivos CC", pero no se usa (el propio docstring lo admite: "NO usa CC"). | `worker.py:10,138-140,572-578` |
| BD-DIF-10 | **Preflight solo en worker**; los steps CLI no validan nombre de hoja ni columnas antes de procesar (pero sí exigen el patrón de nombre al calcular `FECHA_DOCUMENTO`). | `worker.py:105-112` |
| BD-DIF-11 | **Ubicación de recursos:** steps buscan esquemas en rutas relativas al directorio actual (y piden selección manual si no los encuentran) y la query en una carpeta `queries/` subiendo hasta 3 niveles (desde cwd y desde el script); worker usa `assets/esquemas/<nombre>` y `assets/queries/<nombre>` resueltos desde la raíz/paquete. Extensiones de esquema probadas: `.yaml`, `.yml`, `.json`. Nota: el step 3 busca una carpeta `queries/`, pero el archivo vive en `assets/queries/`. | `step2_capagold.py:50-59`, `step3_flags_empleados.py:56-128`, `worker.py:710-733`, `src/utils/structured_config.py:12-28` |
| BD-DIF-12 | **Hoja leída vs validada:** preflight valida la hoja `<CLIENTE>`; el procesamiento lee la hoja activa. | `bd.yaml:9-16` vs `worker.py:244` |
| BD-DIF-13 | **Normalización de encabezados:** preflight compara normalizado (trim, espacios, mayúsculas); Gold selecciona por nombre exacto. Un Excel con `Numero de Doc` o `SEXO ` pasa el preflight pero la columna no llega a Gold (solo warning). | `validate_source.py:363-373` vs `worker.py:466-467` |
| BD-DIF-14 | **Docstring vs código:** step2 dice "Silver 35 columnas / Gold 13"; el esquema actual define 14 (incluye `FECHA_DOCUMENTO`). Arquitectura documentada `gold/actual` no existe: actuales van en `gold/`. Solo CC usa `actual/` + `historico/`. | `step2_capagold.py:9-10`, `esquema_bd.yaml:65-68` |
| BD-DIF-15 | **Formatos de timestamp** distintos: CC `%Y%m%d_%H%M%S`; Gold/flags `%d.%m.%Y_%H.%M.%S`. | `worker.py:362` vs `worker.py:526,643` |
| BD-DIF-16 | **Config sin uso:** `reglas_flags.yaml` (todo), `esquema_cc.yaml:tipos_datos`, `esquema_bd.yaml:metadata` (`header_row`, `data_start_row`, `key_column`) y `format` por columna no son leídos por el código; los valores están codificados en duro (filas 10/11, umbrales en SQL). | `reglas_flags.yaml`, `esquema_cc.yaml:6-10`, `esquema_bd.yaml:4-8` |

---

## 9. Ambigüedades y preguntas abiertas

| ID | Pregunta |
|---|---|
| Q-01 | ¿Qué semántica de lectura es la canónica: la del step (corte en primer NUMERO DE DOC vacío, trim, conversión `DD/MM/YYYY`) o la del worker (lo que hoy está en producción)? Recomendación: la del step, que es más robusta, pero confirmar que no se dependa de filas que hoy solo el worker incluye. |
| Q-02 | Filas con `Modalidad de Contrato` nula hoy desaparecen de Gold (BD-GLD-07). ¿Deben ir a empleados, a practicantes, o a un reporte de rechazos? |
| Q-03 | Antigüedad: ¿medir hasta `Fecha de Termino` futura (fin de contrato proyectado, lo que hoy hace) o hasta la fecha de corte? ¿La fecha de referencia debe ser `CURRENT_DATE` (ejecución) o `FECHA_DOCUMENTO` (corte del archivo)? Reprocesar un archivo antiguo hoy cambia todas las flags. |
| Q-04 | Edad 65/70 se evalúa por año de nacimiento, no por fecha exacta. ¿Es intencional (alerta anual) o se requiere fecha de cumpleaños? |
| Q-05 | `NUMERO DE DOC` duplicado multiplica filas (k⁴) en flags (BD-FLG-12). ¿Se debe deduplicar en Silver/Gold, rechazar, o calcular flags por fila? ¿Puede un mismo documento tener varias filas legítimas (p. ej. recontratación)? |
| Q-06 | Documentos guardados como número pierden ceros a la izquierda y pueden verse como `"123.0"`. ¿Normalizar (entero sin decimales, padding a 8 dígitos para DNI)? |
| Q-07 | ¿Se soportan realmente `.xls`? El regex y el filtro los aceptan pero el lector no los abre (inferido). |
| Q-08 | El patrón de nombre exige `..` antes de la extensión (`BD. DD.MM.YYYY..xlsx`). ¿Es la convención real de RR. HH. o un error tolerado? |
| Q-09 | Archivos "actuales" obsoletos cuando un grupo queda vacío (BD-OUT-04/BD-DIF-06). ¿Escribir un archivo vacío con esquema, o borrar el anterior? |
| Q-10 | ¿El Step 3 debe ser no bloqueante (hoy: éxito con advertencia) o un fallo debe marcar el ETL como fallido? |
| Q-11 | ¿Leer la hoja activa o la hoja por nombre (`<CLIENTE>`)? ¿Validar en preflight las columnas `CC`, `NOMBRE CC`, `CATEGORIA CC`? |
| Q-12 | Comparaciones de texto (split por `TERMINO DE CONVENIO`, modalidades de alertas) son sensibles a mayúsculas/espacios. ¿Normalizar (trim + upper)? |
| Q-13 | Deduplicación de CC por "primera ocurrencia": ¿hay que detectar CC con distinto `NOMBRE CC`/`CATEGORIA CC`/`GERENCIA` entre filas? ¿Excluir CC nulo? ¿Orden numérico o textual? |
| Q-14 | Política de Excel: CC siempre genera `.xlsx` actual; Gold solo con checkbox. ¿Unificar? ¿Histórico en Excel? ¿Unificar formato de timestamp? |
| Q-15 | Comportamiento de `tiempo_servicio_texto` con ingreso nulo o antigüedad negativa (BD-FLG-04): definir texto esperado (nulo / vacío / error). |
| Q-16 | ¿`reglas_flags.yaml` debe convertirse en la fuente de configuración de umbrales en la reescritura, o eliminarse? |

---

## 10. Rework (Plan 4)

Implementado en `config/sql/base_personal/` y `config/parametros.yaml` (umbrales, TR-D10).

| Regla | Decisión | Detalle |
|---|---|---|
| BD-SLV-07 / Q-05 | Corregir | Documento repetido: aviso; en el Gold queda la primera fila (TR-D4) |
| BD-CC-01…03 | Preservar, salvo nulos | Primera aparición, orden por texto; se excluyen los centros de costo nulos |
| BD-GLD-06 / Q-12 | Corregir | Practicante si la modalidad **normalizada** contiene `TERMINO DE CONVENIO` (TR-D24) |
| BD-GLD-07 / Q-02 | Corregir | Sin modalidad cuenta como empleado, con aviso (TR-D24) |
| BD-FLG-02/03 | Preservar | Antigüedad hasta `COALESCE(fin de contrato, fecha de corte)`, en días / 365,25 |
| BD-FLG-04 / Q-15 | Corregir | `tiempo_servicio_texto` nulo sin fecha de entrada o con antigüedad negativa |
| BD-FLG-05…07 | Preservar la fórmula, corregir la fecha | Por año de nacimiento, con el año de la **fecha de corte** (TR-D15, TR-D3) |
| BD-FLG-08…10 | Corregir | Modalidades comparadas normalizadas (TR-D24); umbrales 4,5 y 2,5 en parámetros |
| BD-FLG-12 | Corregir | Flags en un solo `SELECT`, sin auto-joins (TR-D4) |
| BD-DIF-03 | Corregir | Fechas escritas como texto se leen (en `sucio`, el motor marca la alerta de obra que el legacy perdía) |
| BD-OUT-03/04 | Corregir | Salida atómica: `actual/` y `historico/<marca>/` (TR-D7) |

Nuevas columnas: `alerta_contrato_plazo` (obra o incremento) y `alerta_jubilacion` (cualquier `cumple_*`), nunca
nulas, para IND-07 e IND-08.

## 11. Referencias

- `src/modules/bd/steps/step1_capasilver.py:49-137` extracción Excel (encabezados, fin de datos, fechas `DD/MM/YYYY`); `:140-187` armado de tabla texto y nulos; `:190-219` escritura Silver; `:262-263` `FECHA_DOCUMENTO`.
- `src/modules/bd/steps/step1_5_centrosdecosto.py:113-146` extracción/dedupe CC; `:149-226` versionado actual/histórico.
- `src/modules/bd/steps/step2_capagold.py:82-128` selección de columnas; `:131-161` fechas; `:164-195` split; `:198-338` escritura Gold.
- `src/modules/bd/steps/step3_flags_empleados.py:131-161` columnas requeridas; `:164-209` ejecución SQL; `:262-377` escritura flags.
- `src/modules/bd/ui/worker.py:87-184` orquestación; `:105-112` preflight; `:241-295` Step 1; `:301-395` Step 1.5; `:401-566` Step 2; `:572-704` Step 3; `:710-733` resolución de recursos.
- `src/modules/bd/ui/widget.py:50-87` selección de archivo único; `src/modules/bd/ui/config.py:22-29` metadatos del ETL (id `bd`, sin efecto en datos).
- `src/utils/ui/widgets/base_etl_widget.py:60-61,175-182` checkbox `export_excel_gold` y carpeta de salida.
- `src/utils/ui/workers/base_worker.py:80-134` manejo de éxito/error.
- `src/utils/bd_document_date.py:12-16,22-59,71-89` patrón de nombre y `FECHA_DOCUMENTO`.
- `src/utils/validate_source.py:15-19,75-215,246-377` preflight.
- `src/utils/gold_export.py:7-27` exportación Excel opcional.
- `src/utils/structured_config.py:12-28` extensiones de configuración.
- `assets/validate_source/bd.yaml:1-36` contrato de fuente.
- `assets/esquemas/esquema_bd.yaml:1-68` esquema Gold.
- `assets/esquemas/esquema_cc.yaml:1-12` esquema CC.
- `assets/esquemas/reglas_flags.yaml:1-65` reglas documentales (no usadas).
- `assets/queries/queries_flags_gold.sql:23-79` cálculo de flags.
- `tests/test_bd_document_date.py:23-158` casos de prueba de `FECHA_DOCUMENTO`, preservación en Gold/flags y preflight.
