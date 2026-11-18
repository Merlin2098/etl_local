# Validación preflight de fuentes (motor genérico)

> Especificación agnóstica de lenguaje extraída del legacy Python (`HR_Data_Integration_Pipeline`).
> Rutas relativas a la raíz del repo legacy. "(inferido)" = deducción no explícita en el código.

## 1. Propósito

Antes de transformar datos, cada ETL valida sus archivos fuente Excel contra un **contrato declarativo** (`assets/validate_source/<id>.yaml`). El preflight comprueba existencia, extensión, nombre de archivo, fecha embebida en el nombre (solo algunos dominios), apertura del libro, hojas, posición de la cabecera y presencia de columnas mínimas. Acumula errores y, si hay alguno, el worker aborta sin ejecutar ningún stage.
(`src/utils/validate_source.py:75-215`; `docs/es/guia-usuario-preflight-fuentes.md:3-15`)

Puntos de invocación: `src/modules/bd/ui/worker.py:109`, `src/modules/pdt/ui/worker.py:88`, `src/modules/control_practicantes/ui/worker.py:81`, `src/modules/nomina/ui/worker.py:68,77`, `src/modules/nomina_regimen_minero/ui/worker.py:69,87`, `src/modules/examen_retiro/ui/worker.py:152`. Los scripts standalone (`main()` de cada step) **no** ejecutan preflight.

## 2. Formato del contrato (clave por clave)

| Clave | Tipo | Default | Semántica | Fuente |
|---|---|---|---|---|
| `id` | string | nombre del archivo de contrato | Identificador. En minúsculas y sin espacios se usa también para elegir el validador de fecha de documento (PRE-07). | `validate_source.py:67-68, 114-115` |
| `name` | string | `id` | Nombre legible; aparece en el resumen de error. | `validate_source.py:69-70, 48-53` |
| `filename_regex` | regex | (sin validación) | Se busca (búsqueda, no match completo; usar `^…$` para anclar) sobre el **nombre base** del archivo. Admite flags inline como `(?i)`. | `validate_source.py:105-112` |
| `file_type.expected_extensions` | lista | `[.xlsx, .xlsm, .xls]` | Extensiones permitidas, comparadas en minúsculas contra el sufijo del archivo. | `validate_source.py:15, 93-103` |
| `sheet_rules.required_sheets` | lista | claves de `sheets` | Hojas que deben existir (nombre exacto, sensible a mayúsculas). | `validate_source.py:134, 139-141` |
| `sheet_rules.allowed_sheets` | lista | `required_sheets` | Hojas permitidas; solo se usa si `enforce_allowed_sheets` es verdadero. | `validate_source.py:135` |
| `sheet_rules.enforce_allowed_sheets` | bool | `false` | Si es verdadero, cualquier hoja fuera de `allowed_sheets` es error. Todos los contratos actuales lo tienen en `false`. | `validate_source.py:136, 143-146` |
| `sheets` | mapa hoja → config | `{}` | Reglas por hoja. Hojas declaradas pero ausentes en el libro se saltan (el error lo da `required_sheets` si corresponde). | `validate_source.py:148-150` |
| `sheets.<h>.header_start.row_index_1_based` | int | `1` | Fila (1-based) donde está la cabecera. | `validate_source.py:153-154` |
| `sheets.<h>.header_start.column_letter` | letras | `A` | Columna donde empieza la cabecera (A=1, Z=26, AA=27…; inválida → excepción). | `validate_source.py:155-156, 218-227` |
| `sheets.<h>.max_scan_cols` | int | `250` | Máximo de columnas a escanear a la derecha del inicio. Ningún contrato lo define. | `validate_source.py:172` |
| `sheets.<h>.required_columns_source` | lista **o** mapa | `{}` → sin columnas | Origen de las columnas requeridas; ver abajo. | `validate_source.py:246-278` |

### 2.1 `required_columns_source` (resolución, en este orden)

1. Si es **lista** → esas columnas (trim, se descartan vacías).
2. Si es mapa con **`required_columns`** no vacía → esa lista (trim). **Gana sobre todo lo demás y `exclude_columns` NO se aplica.**
3. Si no, requiere **`schema_path`** y **`extract`**; si falta alguno → sin columnas requeridas (validación de columnas desactivada en silencio).
4. Se carga el esquema (`schema_path`, resuelto según PRE-03) y se extraen columnas según `extract` (minúsculas):

| `extract` | Columnas extraídas | Sección usada |
|---|---|---|
| `schema_non_nullable` | claves de `schema` cuyo valor es mapa con `nullable: false` (exactamente `false`) | `hojas.<sheet_name>.schema` si `sheet_name` está definido y es un mapa; si no, `schema` raíz |
| `schema_keys` | todas las claves de `schema` | ídem |
| `columns_required_true` | `columns[].name` con `required: true` | `columns` raíz |
| `columns_non_nullable` | `columns[].name` con `nullable: false` | `columns` raíz |
| `required_columns_array` | `required_columns` raíz | raíz |
| otro valor | ninguna (silencioso) | — |

5. **`exclude_columns`** (lista): se eliminan de lo extraído comparando con la normalización de PRE-12.
6. **`sheet_name`**: selecciona la sub-sección `hojas.<sheet_name>` del esquema para los modos `schema_*`.

(`validate_source.py:246-336`)

> Observación: los 7 contratos actuales definen `required_columns` explícito, por lo que `schema_path`/`extract`/`exclude_columns`/`sheet_name` son hoy configuración muerta (documentan el origen, pero no se usan).

## 3. Algoritmo de validación

Entrada: identificador de contrato + uno o varios archivos (ruta, lista, o mapa cuyos valores son rutas/listas, aplanado recursivamente). (`validate_source.py:192-243`)

- **PRE-01 — Carga del contrato.** Se resuelve `assets/validate_source/<id>` probando `.yaml`, `.yml`, `.json`; si no existe → excepción `FileNotFoundError` (no es un error de reporte). Se completan `id`/`name` por defecto. (`validate_source.py:60-72`)
- **PRE-02 — Sin archivos.** Si la lista de archivos está vacía → error "No source files were provided for preflight validation". (`validate_source.py:204-209`)
- **PRE-03 — Resolución de rutas estructuradas.** Dada una ruta base (con o sin extensión), se generan candidatos con `.yaml`, `.yml`, `.json` (en ese orden); para rutas relativas primero bajo la raíz de recursos de la app (empaquetado) y luego relativa al cwd. Se devuelve el primero que exista; si ninguno existe, el primero de la lista. YAML se lee con cargador seguro; JSON con parser JSON; UTF-8. (`src/utils/structured_config.py:12, 19-57, 75-91`)
- **PRE-04 — Por cada archivo**, se ejecutan los pasos PRE-05…PRE-11 en orden, acumulando errores en un reporte por archivo; los reportes se fusionan. (`validate_source.py:211-213`)
- **PRE-05 — Existencia.** Si no existe → error "Source file does not exist" y **se detiene** la validación de ese archivo. (`validate_source.py:89-91`)
- **PRE-06 — Extensión.** Sufijo en minúsculas ∉ `expected_extensions` → error y **se detiene**. (`validate_source.py:93-103`)
- **PRE-07 — Nombre de archivo.** Si hay `filename_regex` y no hay coincidencia → error "Filename does not match regex: …" (continúa). Si `id` ∈ {`bd`, `control_practicantes`} **y** el nombre coincidió con la regex, se ejecuta el validador de fecha de documento: extrae el grupo `fecha` (`DD.MM.YYYY`) y exige que sea una fecha de calendario real; si no → error "Invalid document date in filename: …". El mapa id→validador está **codificado en el programa**, no en el contrato. (`validate_source.py:16-19, 105-120`; `src/utils/bd_document_date.py:14-19, 22-42`)
- **PRE-08 — Apertura.** Se abre el libro en modo solo lectura con valores calculados (no fórmulas). Fallo → error "Unable to open Excel file: …" y **se detiene**. (`validate_source.py:122-128`)
- **PRE-09 — Hojas.** Cada `required_sheets` ausente → "Missing sheet '<h>'". Con `enforce_allowed_sheets`, hojas sobrantes → "Unexpected sheet(s): [...]" (ordenadas). (`validate_source.py:131-146`)
- **PRE-10 — Ancla de cabecera.** Para cada hoja configurada y presente: si la celda (`row_index_1_based`, `column_letter`) está vacía (nula o solo espacios) → error "Expected header at row R col C (sheet 'H') but found empty/shifted". **La validación continúa** con el escaneo. (`validate_source.py:152-166`)
- **PRE-11 — Escaneo de cabecera y columnas requeridas.** Se leen celdas de la fila de cabecera desde la columna de inicio hasta `inicio + max_scan_cols` (inclusive). Celdas vacías se saltan; tras haber encontrado al menos una cabecera, **3 vacías consecutivas** terminan el escaneo (huecos de 1–2 columnas se toleran; vacías iniciales también). Cada valor se convierte a texto con trim. Luego se calculan las columnas requeridas (§2.1, salvo que el llamador pase una lista explícita que la sustituye) y se reportan las que no aparecen tras normalizar (PRE-12) → "Missing required columns in sheet 'H': [...]" (en el orden del contrato). Columnas extra, orden y duplicados no importan. (`validate_source.py:168-185, 339-369`)

Pseudocódigo resumido:

```
for file in files:
  if !exists -> err, next ; if ext not allowed -> err, next
  if regex && !search(regex, basename) -> err
  elif id in DATE_VALIDATORS -> validate date (err if invalid)
  open workbook or (err, next)
  check required/allowed sheets
  for sheet,cfg in sheets where sheet present:
     anchor empty? -> err
     headers = scan(row, col, max_scan_cols, stop after 3 empties)
     missing = [c in required(cfg) if norm(c) not in {norm(h)}] -> err if any
```

## 4. Normalización de cabeceras

- **PRE-12 — `normalizar(x)`** = `mayúsculas( colapsar_espacios( trim( texto(x) ) ) )`, donde colapsar reemplaza toda secuencia de espacios en blanco Unicode (espacio, tab, saltos de línea, espacio duro) por un único espacio. (`validate_source.py:372-373`)
  - **Mayúsculas/minúsculas:** insensible (`Dni` = `DNI`). Mayúsculas Unicode: `ñ`→`Ñ`.
  - **Acentos:** **sensible** — no se eliminan diacríticos: `AÑO` ≠ `ANO`, `CONDICIÓN` ≠ `CONDICION`.
  - **Espacios:** extremos ignorados; internos colapsados (`FECHA  DE\nCESE` = `FECHA DE CESE`); pero `FECHA ING` ≠ `FECHAING`.
  - **Puntuación:** significativa (`F. RENOVACION` ≠ `F RENOVACION`; `N°` ≠ `Nº`).
- **PRE-13 — Celda vacía** = nula o texto que tras trim es `""`. (`validate_source.py:376-377`)
- **PRE-14 — Nombres de hoja:** comparación exacta (sin normalizar). (`validate_source.py:138-141`)
- Los mismos criterios de normalización se usan para `exclude_columns`. (`validate_source.py:268-276`)

## 5. Reporte de errores

- **PRE-15 — Qué es un fallo.** Cualquier error acumulado (no hay niveles warning). El reporte "pasa" si la lista de errores está vacía. (`validate_source.py:37-39`)
- **PRE-16 — Formato.** Cada error: `"<nombre_archivo>: <mensaje>"`. Resumen: primera línea `Preflight validation failed for '<id>' (<name>)` y luego una línea `- <error>` por error. (`validate_source.py:45-53`)
- **PRE-17 — Excepción.** `raise_if_failed()` lanza `SourceValidationError` (subtipo de error de valor) con el resumen como mensaje y el reporte adjunto; los workers la capturan y devuelven un resultado de error con etapa "Preflight / Validate Source". (`validate_source.py:22-27, 55-57`; p.ej. `src/modules/control_practicantes/ui/worker.py:172-182`)
- **PRE-18 — Mensajes posibles** (texto literal, en inglés): `Source file does not exist`; `Invalid file extension '<ext>'. Expected one of [...]`; `Filename does not match regex: <regex>`; `Filename does not match expected pattern '<patrón>'`; `Invalid document date in filename: <fecha>`; `Unable to open Excel file: <detalle>`; `Missing sheet '<h>'`; `Unexpected sheet(s): [...]`; `Expected header at row R col C (sheet 'H') but found empty/shifted`; `Missing required columns in sheet 'H': [...]`; `No source files were provided for preflight validation`. La guía de usuario explica cómo corregir cada uno. (`docs/es/guia-usuario-preflight-fuentes.md:196-209`)

## 6. Contratos por dominio

| Contrato (`id`) | Usado por | Nombre de archivo | Validador de fecha | Hojas requeridas | Cabecera | Columnas requeridas (efectivas) | Origen declarado (no usado) |
|---|---|---|---|---|---|---|---|
| `bd` | módulo `bd` | `(?i)^BD\.\s*DD.MM.YYYY\.\.(xlsx\|xlsm\|xls)$` (p.ej. `BD. 30.03.2026..xlsx`) | Sí | `<CLIENTE>` (hoja con el nombre del cliente) | fila 10, col A | `NUMERO DE DOC`, `CODIGO SAP2`, `NOMBRE COMPLETOS`, `GERENCIA`, `SEXO`, `SEDE3`, `WHITE COLLAR / BLUE COLLAR`, `Modalidad de Contrato`, `Fecha de Termino`, `SERVICIO`, `REGIMEN DE TRABAJO`, `FECH_INGR.`, `FECHA DE NAC.` | `esquema_bd` / `columns_required_true` |
| `nomina` | módulo `nomina` | — | No | `Planilla` | fila 6, col A | `APELLIDO PATERNO`, `NOMBRES`, `TIPO DE DOCUMENTO`, `DNI/CEX` | `esquema_nominas` / `schema_non_nullable` excl. `PERIODO, AÑO, MES` |
| `licencias` | `nomina` y `nomina_regimen_minero` (archivo adicional) | — | No | `LICENCIA CON GOCE`, `LICENCIA SIN GOCE` | fila 2, col A (ambas) | CON GOCE: `PERIODO`, `DNI/CEX`, `APELLIDO PATERNO`, `APELLIDO MATERNO`, `NOMBRES`, `MOTIVO DE LIC.CON GOCE`; SIN GOCE: ídem con `MOTIVO DE LIC.S.G.H` | `esquema_licencias` (sin `extract`) |
| `regimen_minero` | módulo `nomina_regimen_minero` | `(?i)^.*(?:19\|20)\d{2}-\d{2}.*\.(xlsx\|xlsm\|xls)$` (contiene `YYYY-MM`) | No | `Planilla` | fila 6, col A | `DNI/CEX` | `esquema_regimen_minero` / `schema_non_nullable` excl. `PERIODO, AÑO, MES` |
| `control_practicantes` | módulo `control_practicantes` | `(?i)^BD Practicantes\s+DD.MM.YYYY\.(xlsx\|xlsm\|xls)$` | Sí | `Practicantes` | fila 4, col A | `N°`, `DNI`, `APELLIDOS Y NOMBRES`, `CONDICION`, `FECHA ING`, `F. RENOVACION`, `SEDE`, `UNIVERSIDAD`, `JEFE INMEDIATO`, `GERENCIA` | `esquema_control_practicantes` / `required_columns_array` |
| `ingresos` | módulo `pdt` | — | No | `EMPLEADOS`, `PRACTICANTES` | EMPLEADOS: fila 2, **col B**; PRACTICANTES: fila 2, col A | EMPLEADOS: `AÑO`, `MES`, `DNI`, `N° DOCUM.`; PRACTICANTES: `AÑO`, `MES`, `Numero Documento` | `esquema_relacion_ingresos` / `esquema_ingresos_practicantes`, `schema_non_nullable` con `sheet_name` |
| `examen_retiro` | módulo `examen_retiro` | — | No | `DATA` | fila 3, col A | `NOMBRE`, `DNI`, `FECHA DE CESE` | `esquema_examen_retiro` / `schema_non_nullable` excl. `AÑO, MES` |

En todos: extensiones `.xlsx/.xlsm/.xls`, `enforce_allowed_sheets: false`, `allowed_sheets = required_sheets`, `max_scan_cols` por defecto (250).
Fuentes: `assets/validate_source/bd.yaml:1-36`, `nomina.yaml:1-30`, `licencias.yaml:1-42`, `regimen_minero.yaml:1-28`, `control_practicantes.yaml:1-33`, `ingresos.yaml:1-47`, `examen_retiro.yaml:1-28`.

## 7. Limitaciones y propuestas para el rework

1. **Configuración muerta / doble fuente de verdad.** `required_columns` explícito anula `schema_path`+`extract` (y `exclude_columns`). Propuesta: una sola fuente (preferible el esquema) o validar en CI que ambas coincidan. (`validate_source.py:254-256`)
2. **Validadores de fecha codificados** por `id`. Propuesta: clave de contrato p.ej. `document_date: {group: fecha, format: DD.MM.YYYY}` aplicable a cualquier dominio. (`validate_source.py:16-19`)
3. **Normalización inconsistente con los steps.** Preflight es insensible a mayúsculas, pero varios steps buscan columnas con nombre exacto (CP, examen retiro) → falsos "pasa". Propuesta: una única función de normalización compartida por preflight y lectura, y que el lector devuelva el mapeo cabecera→columna canónica.
4. **Escaneo distinto al de los steps** (preflight tolera 2 huecos; algunos steps cortan en el primer hueco). Unificar la detección de cabeceras.
5. **Acentos y símbolos.** No hay plegado de diacríticos ni equivalencias (`N°`/`Nº`, `Ñ`/`N`). Decidir explícitamente; si se pliega, documentarlo en el contrato (`normalize: {accents: fold}`).
6. **`.xls` declarado pero probablemente no soportado** por el lector OOXML → siempre "Unable to open Excel file" (inferido). Quitarlo del contrato o usar un lector compatible.
7. **Sin validación de contenido**: no se revisan tipos, nulos, filas de datos, ni archivos no-Excel (p.ej. parquets CC de examen retiro). Propuesta: extender contrato con reglas por columna (tipo, nullable) y soporte de Parquet/CSV.
8. **Errores fatales vs acumulables**: existencia/extensión/apertura cortan; el resto acumula. Contrato no encontrado lanza excepción en vez de error de reporte. Uniformar con códigos de error estables (además del texto) para i18n.
9. **Regex por búsqueda**: un contrato sin `^…$` acepta coincidencias parciales. Propuesta: match completo por defecto.
10. **Los modos standalone saltan el preflight**; en el rework el preflight debería formar parte del pipeline, no del worker de UI.
11. La guía de usuario enlaza a una ruta absoluta de una máquina personal; reemplazar por ruta relativa. (`docs/es/guia-usuario-preflight-fuentes.md:222`)

## 8. Referencias

- `src/utils/validate_source.py:15-19, 22-57, 60-72, 75-189, 192-215, 218-243, 246-336, 339-377`
- `src/utils/structured_config.py:12-57, 75-91`
- `src/utils/bd_document_date.py:13-50`
- `assets/validate_source/{bd,nomina,licencias,regimen_minero,control_practicantes,ingresos,examen_retiro}.yaml`
- `docs/es/guia-usuario-preflight-fuentes.md:1-222`
- `tests/test_control_practicantes_document_date.py:146-177`
- Invocaciones: `src/modules/bd/ui/worker.py:109`, `src/modules/pdt/ui/worker.py:88`, `src/modules/control_practicantes/ui/worker.py:81`, `src/modules/nomina/ui/worker.py:68,77`, `src/modules/nomina_regimen_minero/ui/worker.py:69,87`, `src/modules/examen_retiro/ui/worker.py:152`

## 9. Reglas del motor nuevo

El motor en Rust (`crates/core`, Plan 3) implementa PRE-05…PRE-11 con el contrato unificado de
[ADR-0003](../adr/0003-formato-contrato-fuente.md), mensajes en español y ubicación por hoja y celda. Diferencias
con el legacy y reglas nuevas:

| Regla | Qué cambia o agrega |
|---|---|
| PRE-07 | `patron_nombre` exige coincidencia **completa** (corrige §7.9). La fecha del nombre se valida por contrato (`fecha_en_nombre`), no por `id` (corrige §7.2). La planilla exige el periodo en el nombre (corrige NOM-BRZ-05) |
| PRE-10 | Si la cabecera no está donde se espera, se buscan las columnas obligatorias en ±3 filas y ±3 columnas y el mensaje dice dónde están (TR-D13). El archivo se rechaza igual |
| PRE-12 | La normalización también **quita tildes** (TR-D1): `CONDICIÓN` = `CONDICION` |
| PRE-19 | **Cabecera repetida:** dos cabeceras de la fila que normalizan igual y corresponden a una columna del contrato. Error en la primera celda: «La columna «X» aparece dos veces (F6 y AH6).» |
| PRE-20 | **Dato ilegible** (aviso): el contenido no se puede leer como el tipo de la columna (`31/02/2025` en una fecha, `tres` en un número). El valor queda vacío y el aviso dice la celda. Corrige BD-DIF-03: una fecha escrita como texto ya no se pierde en silencio |
| PRE-21 | **Falta un dato obligatorio** (error): celda vacía en una columna `nula: false`, en una fila que tiene clave. «Falta NOMBRES en la fila 12.» |
| PRE-22 | **Error de Excel** (aviso): la celda tiene `#N/A`, `#DIV/0!`… o el texto `#N/D`. Se toma como vacía (TR-12) |
| PRE-23 | **Valor inesperado** (aviso): fuera de `valores` (sin distinguir mayúsculas ni tildes) o de `minimo`/`maximo`; el mensaje dice el rango («entre 0 y 31») |
| PRE-24 | **Fila no incluida** (aviso): la fila tiene datos pero la columna `clave` está vacía, o trae un valor que equivale a vacío (`#N/A`, `nan`). Se descarta y el aviso dice la celda (TR-D20, TR-D21) |
| PRE-08 | El mensaje distingue archivo temporal de Excel (`~$…`), página web guardada como `.xls` y contenido que no corresponde a la extensión. El detalle técnico del lector va en un campo aparte (`detalle`), no en el mensaje. Solo se cargan las hojas del contrato: una hoja dañada que no se usa no impide leer el archivo |
| PRE-05 | Una carpeta en lugar de un archivo tiene su propio mensaje |
| NOM-BRZ-10 | **Filas de totales:** con `filas_resumen` en la hoja del contrato, la fila cuya primera celda (sin `+`, `-`, `*` iniciales) contiene uno de esos textos se descarta sin aviso, antes de mirar la clave. Ya no genera PRE-24 |
| PRE-21 | Se agrupa por columna: «Falta NOMBRES en 1.200 filas (B7, B8, B9… B1206).» |

**Filas que se leen:** desde la fila siguiente a la cabecera hasta el final de la hoja (`fin_datos: fin_hoja`, por
defecto). Las filas totalmente vacías se descartan sin aviso.
`fila_vacia` y `clave_vacia` cortan la lectura en la primera fila vacía o sin clave (TR-D2). Una fila con otros
datos y sin clave se descarta con aviso PRE-24 (TR-D21).

Si un contrato da varias cabeceras posibles (`origen`) y el archivo trae más de una, gana la primera según el
orden del contrato (por ejemplo, `#HRS SIMP.50%` antes que `HRS SIMPLES AL 50%`).
