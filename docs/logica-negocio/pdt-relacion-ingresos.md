# Lógica de negocio — Dominio `pdt` (Relación de Ingresos)

> Especificación agnóstica de lenguaje extraída del código legado Python (`HR_Data_Integration_Pipeline`).
> Objetivo: permitir reimplementar el dominio (Go / Rust) sin leer el código original.
> Convenciones: cada regla tiene un ID (`PDT-<CAPA>-NN`) y cita su fuente `ruta/archivo.py:línea`. Lo marcado "(inferido)" no está explícito en el código sino deducido del comportamiento de las librerías usadas (openpyxl / polars).
> Rutas relativas a la raíz del repo legado. El nombre de la empresa cliente se reemplaza por `<CLIENTE>`.

Prefijos de ID:

| Prefijo | Ámbito |
|---|---|
| `PDT-BRZ` | Lectura y contrato de la fuente Bronze (incl. preflight) |
| `PDT-SLV` | Reglas Bronze → Silver |
| `PDT-GLD-E` | Reglas Silver → Gold, EMPLEADOS |
| `PDT-GLD-P` | Reglas Silver → Gold, PRACTICANTES |
| `PDT-NEG` | Reglas de negocio transversales (fórmulas / mapeos) |
| `PDT-OUT` | Salidas |
| `PDT-VAL` | Validaciones, errores y advertencias |

---

## 1. Propósito del dominio

El dominio procesa el archivo Excel mensual de **Relación de Ingresos** de `<CLIENTE>`: el registro de las personas que ingresan a la empresa, en dos hojas:

- **EMPLEADOS**: personal contratado (documento, datos demográficos, ubicación, cargo, área, fecha de inicio, tipo de contrato, proyecto, código SAP, etc.).
- **PRACTICANTES**: practicantes (documento, datos demográficos, cargo, área, sede, fecha de inicio, universidad de procedencia, etc.).

Pipeline de tres pasos (arquitectura medallón):

1. **Step 1 (Bronze → Silver)**: lee las dos hojas, aplica limpieza básica, genera `PERIODO`, y escribe un Parquet Silver por hoja (todas las columnas como texto).
2. **Step 2 (Silver → Gold EMPLEADOS)**: selecciona/tipa columnas según el esquema YAML, imputa `SERVICIO`/`REGIMEN` (solo en el script standalone, ver Discrepancias), agrega `NOMBRE_MES`, escribe Gold actual + histórico.
3. **Step 3 (Silver → Gold PRACTICANTES)**: selecciona/tipa columnas según su esquema, aplica la regla de `Universidad de Procedencia`, agrega `NOMBRE_MES`, escribe Gold actual + histórico.

Los Gold están pensados para consumo en Power BI con rutas estables (`assets/esquemas/esquema_relacion_ingresos.yaml:110`, `step2_exportar_ingresos.py:393-396`).

Existen dos puntos de entrada con el mismo núcleo de lógica:

- **Scripts standalone** (`src/modules/pdt/steps/step{1,2,3}_*.py`, `main()`): cada paso se ejecuta por separado y el usuario elige los archivos mediante diálogo.
- **Worker de UI** (`src/modules/pdt/ui/worker.py`): ejecuta preflight + los 3 pasos en secuencia sobre un único archivo, reutilizando funciones de los steps mediante *lazy loading*. **No llama a `main()` de los steps**; reimplementa la orquestación y diverge en varios puntos (sección 8).

El *lazy loader* (`src/utils/lazy_loader.py:12-110`) solo difiere la importación del módulo del step hasta el primer acceso a un atributo; no altera el comportamiento de los datos. Si la importación falla, lanza `ImportError` (`src/utils/lazy_loader.py:31-40`), lo que el worker trata distinto según el paso (PDT-VAL-07).

---

## 2. Fuentes (Bronze)

### 2.1 Archivo

| ID | Regla | Fuente |
|---|---|---|
| PDT-BRZ-01 | Una ejecución procesa **exactamente un archivo** Excel que contiene ambas hojas. El worker aborta si recibe ≠ 1 archivo ("Se esperaba 1 archivo, se recibieron N"). | `src/modules/pdt/ui/worker.py:76-81`, `src/modules/pdt/ui/widget.py:46-52` |
| PDT-BRZ-02 | Extensiones aceptadas por el contrato de preflight: `.xlsx`, `.xlsm`, `.xls` (comparación en minúsculas). El selector de la UI solo admite `.xlsx` y `.xls` y rechaza otra extensión con un aviso (sin procesar). El script standalone filtra `*.xlsx *.xls` (con opción "All files"). | `assets/validate_source/ingresos.yaml:3-7`, `src/utils/validate_source.py:94-103`, `src/modules/pdt/ui/widget.py:61-67`, `step1_consolidar_ingresos.py:99-102` |
| PDT-BRZ-03 | **No hay patrón de nombre de archivo**: el contrato no define `filename_regex` ni existe validador de fecha de documento para el id `ingresos`. Cualquier nombre es válido. | `assets/validate_source/ingresos.yaml:1-47`, `src/utils/validate_source.py:105-120`, `src/utils/validate_source.py:16-19` |
| PDT-BRZ-04 | La lectura se hace con valores calculados (no fórmulas): se toma el último valor cacheado de cada celda. | `step1_consolidar_ingresos.py:129` |
| PDT-BRZ-05 | La carpeta de trabajo es la **carpeta padre del archivo Bronze**; ahí se crean `silver/` y `gold/`. | `step1_consolidar_ingresos.py:391`, `src/utils/ui/widgets/base_etl_widget.py:175` |

### 2.2 Hojas, encabezados y columnas

| Hoja | Fila de encabezados | Columna de inicio | Datos desde | Columnas esperadas (orden) |
|---|---|---|---|---|
| `EMPLEADOS` | 2 | **B** (índice 2, base 1) | fila 3 | 22 |
| `PRACTICANTES` | 2 | **A** (índice 1, base 1) | fila 3 | 20 |

Fuente: `step1_consolidar_ingresos.py:31-86`; contrato preflight `assets/validate_source/ingresos.yaml:17-20,34-37`.

Columnas esperadas EMPLEADOS (`step1_consolidar_ingresos.py:35-58`):
`AÑO, MES, DNI, N° DOCUM., NOMBRES COMPLETOS, SEXO, DEPARTAMENTO, PROVINCIA, DISTRITO, CARGO, ÁREA, JEFE DIRECTO, LUGAR DE TRABAJO, FECHA INICIO, CC, WC/BC, Tiempo de contrato, NIVEL II, NIVEL III, NIVEL IV, PROYECTO, CODIGO SAP`.

Columnas esperadas PRACTICANTES (`step1_consolidar_ingresos.py:63-84`):
`AÑO, MES, Tipo Documento, Numero Documento, CODIGO SAP, Nombres Completos, Fecha Nacimiento, Sexo, Departamento, Provincia, Distrito, Nacionalidad, Cargo, Área, Jefe responsable de área, Sede de Trabajo, Fecha Inicio, CC, Teléfono, Universidad de Procedencia`.

| ID | Regla | Fuente |
|---|---|---|
| PDT-BRZ-06 | **Detección de encabezados (Step 1)**: desde (fila 2, columna de inicio) se avanza hacia la derecha y se toma cada celda como nombre de columna (convertida a texto y con espacios recortados en ambos extremos) **hasta la primera celda nula**. No hay tolerancia a huecos. Una celda con solo espacios no es nula: produce un encabezado vacío `""` (inferido). | `step1_consolidar_ingresos.py:141-148` |
| PDT-BRZ-07 | Los encabezados leídos **no se comparan por nombre** con la lista esperada; solo se compara la **cantidad**. Si difiere, se emite una advertencia y se continúa. Las columnas reales del Excel (en su orden) son las que pasan a Silver, aunque tengan otros nombres o haya columnas extra. | `step1_consolidar_ingresos.py:152-157` |
| PDT-BRZ-08 | **Inicio de datos**: fila de encabezados + 1 (fila 3). Para cada fila se toman las celdas desde la columna de inicio y tantas como encabezados detectados. | `step1_consolidar_ingresos.py:161-165` |
| PDT-BRZ-09 | **Fin de datos**: la lectura se detiene en la **primera fila en la que todas las celdas del rango de encabezados son nulas o texto en blanco** (tras recortar espacios). Las filas posteriores se ignoran aunque contengan datos. | `step1_consolidar_ingresos.py:167-169` |
| PDT-BRZ-10 | **Conversión a texto**: todo valor no nulo se convierte a su representación textual; los nulos se mantienen nulos. Efectos (inferido, según la representación de Python): entero `2025` → `"2025"`; decimal `5.0` → `"5.0"`; fecha/hora Excel → `"YYYY-MM-DD HH:MM:SS"`; booleano → `"True"`/`"False"`; error de Excel → su literal (p. ej. `"#N/A"`). Los textos **no** se recortan. | `step1_consolidar_ingresos.py:182-186` |
| PDT-BRZ-11 | Si dos encabezados tienen el mismo nombre, la construcción de la tabla por diccionario conserva solo la **última** columna con ese nombre (inferido). | `step1_consolidar_ingresos.py:189-195` |
| PDT-BRZ-12 | Si la hoja no tiene filas de datos, se devuelve una tabla vacía y la hoja se **omite** (no se escribe Silver para ella). | `step1_consolidar_ingresos.py:178-180`, `step1_consolidar_ingresos.py:410-412`, `src/modules/pdt/ui/worker.py:142-146` |
| PDT-BRZ-13 | Si la hoja no existe en el libro, se lanza error "La hoja 'X' no existe. Hojas disponibles: [...]"; en Step 1 se captura por hoja y la hoja se omite. | `step1_consolidar_ingresos.py:131-134`, `step1_consolidar_ingresos.py:423-425` |

### 2.3 Contrato de preflight (solo worker de UI)

El worker ejecuta `validate_all_sources_for_etl("ingresos", archivo)` **antes** de cualquier paso; cualquier error aborta todo el ETL (`src/modules/pdt/ui/worker.py:88-89`, `src/modules/pdt/ui/worker.py:626-638`). Los scripts standalone **no** ejecutan preflight.

| ID | Chequeo | Resultado si falla | Fuente |
|---|---|---|---|
| PDT-BRZ-14 | El archivo existe. | Error "Source file does not exist" (corta el resto de chequeos). | `src/utils/validate_source.py:89-91` |
| PDT-BRZ-15 | Extensión ∈ {`.xlsx`, `.xlsm`, `.xls`}. | Error de extensión inválida (corta). | `src/utils/validate_source.py:94-103` |
| PDT-BRZ-16 | El libro se puede abrir. Nota: la librería usada no abre `.xls` binario antiguo, por lo que un `.xls` fallaría aquí (inferido). | Error "Unable to open Excel file" (corta). | `src/utils/validate_source.py:122-128` |
| PDT-BRZ-17 | Hojas obligatorias `EMPLEADOS` y `PRACTICANTES` presentes (nombre exacto). Hojas adicionales permitidas (`enforce_allowed_sheets: false`). | Error "Missing sheet 'X'" por cada una. | `assets/validate_source/ingresos.yaml:8-15`, `src/utils/validate_source.py:134-146` |
| PDT-BRZ-18 | La celda de inicio de encabezados no está vacía: `EMPLEADOS!B2`, `PRACTICANTES!A2`. | Error "Expected header at row 2 col B/A … but found empty/shifted". | `assets/validate_source/ingresos.yaml:18-20,35-37`, `src/utils/validate_source.py:153-166` |
| PDT-BRZ-19 | Lectura de encabezados para el preflight: desde la celda de inicio hacia la derecha, hasta 250 columnas; las celdas vacías se saltan y se corta tras **3 vacías consecutivas** (una vez encontrado al menos un encabezado). | — | `src/utils/validate_source.py:168-173`, `src/utils/validate_source.py:339-360` |
| PDT-BRZ-20 | Columnas obligatorias presentes. Como el contrato define `required_columns` explícitas, **estas tienen prioridad** y no se derivan del esquema (`schema_path`, `extract`, `exclude_columns` quedan sin efecto). EMPLEADOS: `AÑO, MES, DNI, N° DOCUM.`. PRACTICANTES: `AÑO, MES, Numero Documento`. Comparación normalizada: recorte, colapso de espacios internos a uno solo, y mayúsculas. | Error "Missing required columns in sheet 'X': [...]". | `assets/validate_source/ingresos.yaml:21-33,38-47`, `src/utils/validate_source.py:175-185`, `src/utils/validate_source.py:246-256`, `src/utils/validate_source.py:363-373` |

Todos los errores se acumulan y se reportan juntos con encabezado "Preflight validation failed for 'ingresos' (PDT Relacion de Ingresos Bronze)" (`src/utils/validate_source.py:45-57`).

---

## 3. Reglas Bronze → Silver

Se aplican a cada hoja leída, en este orden (`step1_consolidar_ingresos.py:204-295`). Todas las columnas Silver son texto.

| ID | Regla | Fuente |
|---|---|---|
| PDT-SLV-01 | **PROYECTO → "Staff"**: si existe la columna `PROYECTO` (en la práctica solo EMPLEADOS), todo valor que sea exactamente `"0"`, nulo, o vacío/solo espacios (tras recortar) se reemplaza por `"Staff"`. Los demás valores se dejan tal cual (sin recortar). La comparación con `"0"` es exacta: `"0.0"` o `" 0"` **no** se reemplazan. | `step1_consolidar_ingresos.py:219-235` |
| PDT-SLV-02 | **CODIGO SAP → nulo** (ambas hojas, si existe la columna): se anula el valor cuando (a) **contiene** la subcadena `#N/D` o `#N/A` (sensible a mayúsculas), o (b) **contiene** `error` sin distinguir mayúsculas (p. ej. `"Error"`, `"ERROR de carga"`), o (c) ya es nulo. Los demás valores se conservan. | `step1_consolidar_ingresos.py:241-262` |
| PDT-SLV-03 | **Eliminación de filas totalmente nulas**: se eliminan las filas cuyas columnas son todas nulas. En la práctica no elimina nada (inferido): la lectura ya se detuvo en la primera fila vacía (PDT-BRZ-09) y, en EMPLEADOS, `PROYECTO` ya fue rellenado con `"Staff"`. | `step1_consolidar_ingresos.py:267-273` |
| PDT-SLV-04 | **PERIODO**: si existen `AÑO` y `MES`, se crea `PERIODO = texto(AÑO) + "-" + rellenar_ceros_izq(texto(MES), 2)` y se coloca como **primera columna**; el resto conserva su orden. Ver PDT-NEG-01 para casos borde. | `step1_consolidar_ingresos.py:275-291` |
| PDT-SLV-05 | No se aplica ninguna otra limpieza en Silver: sin recorte de espacios, sin deduplicación, sin validación de valores permitidos, sin tipado. | `step1_consolidar_ingresos.py:204-295` |
| PDT-SLV-06 | **Reporte de calidad por hoja** (solo informativo, impreso/log): registros originales, registros limpios, registros eliminados (= originales − limpios), tasa de retención `limpios / originales × 100` con 2 decimales (solo si originales > 0), y por cada columna con nulos: conteo y porcentaje `nulos / total × 100` (2 decimales). Si ninguna columna tiene nulos: "No hay valores nulos". | `step1_consolidar_ingresos.py:298-326` |
| PDT-SLV-07 | **Métricas de limpieza** (informativas): cantidad de PROYECTO reemplazados (mismo filtro que la regla), cantidad de CODIGO SAP "afectados" y filas vacías eliminadas. El conteo de CODIGO SAP usa un filtro distinto al del reemplazo: igualdad exacta con `#N/D`, `Error` o `#N/A`, **o** contiene `error` sin distinguir mayúsculas, **o** nulo. Por eso cuenta también los nulos preexistentes y no cuenta valores que solo *contienen* `#N/D`/`#N/A` (p. ej. `"X#N/D"`). Las métricas no se persisten. | `step1_consolidar_ingresos.py:216`, `step1_consolidar_ingresos.py:220-224`, `step1_consolidar_ingresos.py:242-246`, `step1_consolidar_ingresos.py:264-265` |
| PDT-SLV-08 | Si ninguna hoja produjo datos, el paso termina sin escribir nada ("No se procesaron datos de ninguna hoja"). Si al menos una produjo, se escriben solo las hojas con datos. | `step1_consolidar_ingresos.py:427-430`, `src/modules/pdt/ui/worker.py:168-175` |

Esquema Silver resultante (inferido de las reglas): `PERIODO` + las columnas reales de la hoja en su orden original, todas texto (nullable).

---

## 4. Reglas Silver → Gold

### 4.0 Mecanismo común de selección y tipado (ambas hojas)

Implementado de forma idéntica en `step2_exportar_ingresos.py:94-147` y `step3_exportar_practicantes.py:97-150`.

| ID | Regla | Fuente |
|---|---|---|
| PDT-GLD-E-01 / PDT-GLD-P-01 | El esquema se toma de `hojas.<HOJA>.schema` del YAML correspondiente. Si falta `hojas` o la hoja, error "El esquema no contiene configuración para <HOJA>". | `step2_exportar_ingresos.py:78-81`, `step3_exportar_practicantes.py:78-84` |
| PDT-GLD-E-02 / PDT-GLD-P-02 | **Todas** las columnas del esquema deben existir en Silver (nombre exacto, sensible a mayúsculas, acentos y espacios). Si falta alguna: error "Columnas faltantes en el DataFrame: [...]" y el paso aborta. | `step2_exportar_ingresos.py:101-105`, `step3_exportar_practicantes.py:104-108` |
| PDT-GLD-E-03 / PDT-GLD-P-03 | **Selección**: la salida contiene **solo** las columnas del esquema, en el **orden del esquema**. Toda otra columna Silver se descarta. | `step2_exportar_ingresos.py:120-140`, `step3_exportar_practicantes.py:123-143` |
| PDT-GLD-E-04 / PDT-GLD-P-04 | **Mapeo de tipos**: `string`→texto, `integer`→entero 64 bits, `float`→decimal 64 bits, `boolean`→booleano, `date`→fecha. Tipo ausente → `string`; tipo desconocido → texto. El nombre del tipo se compara en minúsculas. | `step2_exportar_ingresos.py:108-122`, `step3_exportar_practicantes.py:111-125` |
| PDT-GLD-E-05 / PDT-GLD-P-05 | **Conversión no estricta**: cualquier valor no convertible se vuelve **nulo**, sin error. P. ej. `AÑO = "2025.0"` → nulo (inferido: el parseo texto→entero no acepta decimales). | `step2_exportar_ingresos.py:135`, `step3_exportar_practicantes.py:138` |
| PDT-GLD-E-06 / PDT-GLD-P-06 | **Fechas**: el texto se parsea con formato exacto `%Y-%m-%d %H:%M:%S` (lo que produce PDT-BRZ-10 para celdas fecha de Excel) y se trunca a fecha. Cualquier otro formato (p. ej. `15/01/2025`, `2025-01-15`) → nulo. | `step2_exportar_ingresos.py:125-132`, `step3_exportar_practicantes.py:128-135` |
| PDT-GLD-E-07 / PDT-GLD-P-07 | Restricciones del esquema **no aplicadas por el código**: `nullable`, `pattern`, `min_value`, `max_value`, `allowed_values`, `primary_key`, `business_rules` (texto), `transformations` (`string_trimming`, `date_format`, etc.). Son solo documentación. | `step2_exportar_ingresos.py:94-147`, `step3_exportar_practicantes.py:97-150` |

### 4.1 EMPLEADOS (Step 2)

Esquema: `assets/esquemas/esquema_relacion_ingresos.yaml:8-89` (versión 3.0).

| Columna Gold | Tipo | Nullable (doc) | Notas |
|---|---|---|---|
| PERIODO | string | no | patrón `^\d{4}-\d{2}$` (no validado) |
| AÑO | integer | no | min 2020 (no validado) |
| MES | integer | no | 1–12 (no validado) |
| NOMBRE_MES | string | — | agregada, ver PDT-NEG-03 |
| DNI | string | no | |
| N° DOCUM. | string | no | puede incluir CEX |
| FECHA NACIMIENTO | date | sí | **no está en las columnas esperadas del Bronze** |
| SEXO | string | sí | M/F (no validado) |
| DEPARTAMENTO, PROVINCIA, DISTRITO | string | sí | |
| CARGO, ÁREA, LUGAR DE TRABAJO | string | sí | |
| FECHA INICIO | date | sí | |
| WC/BC | string | sí | |
| Tiempo de contrato | string | sí | |
| SERVICIO | string | no | **no está en las columnas esperadas del Bronze**; imputación PDT-GLD-E-09 |
| REGIMEN | string | no | **no está en las columnas esperadas del Bronze**; imputación PDT-GLD-E-09 |

| ID | Regla | Fuente |
|---|---|---|
| PDT-GLD-E-08 | Columnas Silver **descartadas** en Gold: `NOMBRES COMPLETOS`, `JEFE DIRECTO`, `CC`, `NIVEL II`, `NIVEL III`, `NIVEL IV`, `PROYECTO`, `CODIGO SAP` (y cualquier extra). Por eso la limpieza de `PROYECTO` y `CODIGO SAP` de Silver **no llega a Gold**. | `assets/esquemas/esquema_relacion_ingresos.yaml:10-89`, `step2_exportar_ingresos.py:140` |
| PDT-GLD-E-09 | **Imputación SERVICIO / REGIMEN** (solo script standalone): nulo → `"Personal_Interno"`; texto exactamente vacío `""` → `"Personal_Interno"`. Texto con solo espacios **no** se imputa. Se aplica después del tipado. **El worker de UI no aplica esta regla** (DISC-01). | `step2_exportar_ingresos.py:332-342`, `assets/esquemas/esquema_relacion_ingresos.yaml:96-98` |
| PDT-GLD-E-10 | Se agrega `NOMBRE_MES` inmediatamente después de `MES` (PDT-NEG-03). | `step2_exportar_ingresos.py:348`, `src/modules/pdt/ui/worker.py:317` |
| PDT-GLD-E-11 | **Métricas de calidad** (solo standalone, informativas): total de registros, total de columnas, cantidad de periodos únicos (y su lista separada por comas si son ≤ 12), y nulos por columna con porcentaje (2 decimales). Ver PDT-VAL-10 sobre un fallo posible. | `step2_exportar_ingresos.py:150-177`, `step2_exportar_ingresos.py:351` |
| PDT-GLD-E-12 | Orden final de columnas: el del esquema con `NOMBRE_MES` insertada tras `MES` (19 columnas). | `step2_exportar_ingresos.py:140`, `src/utils/month_name.py:44-48` |

### 4.2 PRACTICANTES (Step 3)

Esquema: `assets/esquemas/esquema_ingresos_practicantes.yaml:8-53` (versión 2.0).

| Columna Gold | Tipo | Nullable (doc) | Notas |
|---|---|---|---|
| PERIODO | string | no | patrón `^\d{4}-\d{2}$` (no validado) |
| AÑO | integer | no | min 2020 (no validado) |
| MES | integer | no | 1–12 (no validado) |
| NOMBRE_MES | string | — | agregada |
| Numero Documento | string | no | |
| Sexo | string | sí | M/F (no validado) |
| Cargo | string | sí | |
| Área | string | sí | |
| Jefe Directo | string | sí | **el Bronze esperado tiene `Jefe responsable de área`** |
| Lugar de Trabajo | string | sí | **el Bronze esperado tiene `Sede de Trabajo`** |
| Universidad de Procedencia | string | sí (pero se imputa) | PDT-NEG-02 |
| Modalidad | string | sí | **no existe en el Bronze esperado** |

| ID | Regla | Fuente |
|---|---|---|
| PDT-GLD-P-08 | Columnas Silver **descartadas**: `Tipo Documento`, `CODIGO SAP`, `Nombres Completos`, `Fecha Nacimiento`, `Departamento`, `Provincia`, `Distrito`, `Nacionalidad`, `Jefe responsable de área`, `Sede de Trabajo`, `Fecha Inicio`, `CC`, `Teléfono` (y extras). No se hace ningún renombrado de columnas. | `assets/esquemas/esquema_ingresos_practicantes.yaml:10-53`, `step3_exportar_practicantes.py:143` |
| PDT-GLD-P-09 | Se aplica la regla de **Universidad de Procedencia** (PDT-NEG-02) después del tipado. | `step3_exportar_practicantes.py:396`, `src/modules/pdt/ui/worker.py:480` |
| PDT-GLD-P-10 | Se agrega `NOMBRE_MES` inmediatamente después de `MES`, **después** de la regla de universidad. | `step3_exportar_practicantes.py:184-197`, `step3_exportar_practicantes.py:399`, `src/modules/pdt/ui/worker.py:484` |
| PDT-GLD-P-11 | **Métricas** (solo standalone): igual que PDT-GLD-E-11 más el conteo de practicantes por cada valor de `Universidad de Procedencia`, ordenado alfabéticamente por universidad. La regla de negocio además informa: nulos transformados y total de `"POR DEFINIR"` resultante. | `step3_exportar_practicantes.py:161-179`, `step3_exportar_practicantes.py:200-240` |
| PDT-GLD-P-12 | Orden final: esquema con `NOMBRE_MES` tras `MES` (12 columnas). | `step3_exportar_practicantes.py:143`, `src/utils/month_name.py:44-48` |

---

## 5. Reglas de negocio (fórmulas, mapeos, casos borde)

### PDT-NEG-01 — PERIODO, AÑO, MES

- `AÑO` y `MES` **se leen tal cual** de la fuente (no se derivan de ninguna fecha) y se guardan como texto en Silver (`step1_consolidar_ingresos.py:182-186`).
- `PERIODO = texto(AÑO) + "-" + lpad(texto(MES), 2, "0")` (`step1_consolidar_ingresos.py:279-285`).
- En Gold, `AÑO` y `MES` se convierten a entero (no estricto) y `PERIODO` se mantiene texto (`assets/esquemas/esquema_relacion_ingresos.yaml:11-26`, `step2_exportar_ingresos.py:135`).

Casos borde (inferidos del comportamiento de las operaciones):

| Entrada AÑO / MES (celda) | Silver `PERIODO` | Gold `AÑO`/`MES` |
|---|---|---|
| 2025 / 3 (enteros) | `2025-03` | 2025 / 3 |
| 2025 / 12 | `2025-12` | 2025 / 12 |
| 2025.0 / 3.0 (decimales) | `2025.0-3.0` (el relleno no aplica: longitud ≥ 2) | nulo / nulo |
| `"2025"` / `"03"` (texto) | `2025-03` | 2025 / 3 |
| AÑO nulo o MES nulo | nulo (concatenación con nulo da nulo) | nulo |
| MES = 13 | `2025-13` (sin validación) | 13 |

### PDT-NEG-02 — Universidad de Procedencia

Regla **completa** implementada: **si el valor es nulo → `"POR DEFINIR"`; cualquier otro valor se conserva sin cambios** (`step3_exportar_practicantes.py:164-172`; documentada en `assets/esquemas/esquema_ingresos_practicantes.yaml:60`).

- **No existe ninguna tabla de mapeo/normalización de nombres de universidades** en el código ni en los YAML del dominio.
- Texto vacío `""` o solo espacios **no** se transforma (solo nulo). Una celda vacía en Excel llega como nulo y sí se transforma; una celda con espacios llega como texto y no (inferido).
- Sin normalización de mayúsculas/acentos/espacios.

### PDT-NEG-03 — NOMBRE_MES

- Mapeo fijo de `MES` (entero) a nombre **en inglés**: 1 January, 2 February, 3 March, 4 April, 5 May, 6 June, 7 July, 8 August, 9 September, 10 October, 11 November, 12 December (`src/utils/month_name.py:6-19`).
- Valor fuera de 1–12 → `""` (cadena vacía), por `default_invalid=""` (`src/utils/month_name.py:38-42`, `step2_exportar_ingresos.py:348`, `step3_exportar_practicantes.py:191`).
- `MES` nulo → nulo (inferido: el reemplazo con valor por defecto no altera nulos).
- Se inserta inmediatamente después de `MES` (`src/utils/month_name.py:44-48`). Si `MES` no existe → error "Column 'MES' not found in DataFrame" (`src/utils/month_name.py:33-36`).

### PDT-NEG-04 — PROYECTO "Staff"

Ver PDT-SLV-01. Semántica (inferido): un empleado sin proyecto asignado (valor `0`, vacío o nulo) pertenece a "Staff". Solo existe en Silver EMPLEADOS.

### PDT-NEG-05 — CODIGO SAP inválido

Ver PDT-SLV-02. Semántica (inferido): los errores de BUSCARV/fórmulas de Excel (`#N/D` en español, `#N/A` en inglés, "Error") significan "sin código SAP". Solo existe en Silver.

### PDT-NEG-06 — SERVICIO / REGIMEN "Personal_Interno"

Ver PDT-GLD-E-09. Solo standalone.

---

## 6. Salidas

| ID | Salida | Ruta (relativa a la carpeta del Bronze) | Formato | Sobrescritura | Fuente |
|---|---|---|---|---|---|
| PDT-OUT-01 | Silver EMPLEADOS | `silver/Relacion Ingresos EMPLEADOS.parquet` | Parquet, compresión snappy | Se sobrescribe en cada ejecución (sin timestamp) | `step1_consolidar_ingresos.py:343-358` |
| PDT-OUT-02 | Silver PRACTICANTES | `silver/Relacion Ingresos PRACTICANTES.parquet` | Parquet snappy | Ídem | `step1_consolidar_ingresos.py:343-358` |
| PDT-OUT-03 | Gold EMPLEADOS actual | `gold/ingresos_empleados_gold.parquet` | Parquet snappy | Se sobrescribe | `step2_exportar_ingresos.py:196-218` |
| PDT-OUT-04 | Gold EMPLEADOS Excel actual (opcional) | `gold/ingresos_empleados_gold.xlsx` | Excel | Se sobrescribe | `step2_exportar_ingresos.py:221-225` |
| PDT-OUT-05 | Gold EMPLEADOS histórico | `gold/historico/ingresos_empleados_gold_<DD.MM.YYYY_HH.MM.SS>.parquet` (+ `.xlsx` opcional) | Parquet snappy / Excel | Nuevo archivo por ejecución | `step2_exportar_ingresos.py:201-205`, `step2_exportar_ingresos.py:233-246` |
| PDT-OUT-06 | Gold PRACTICANTES actual | `gold/ingresos_practicantes_gold.parquet` (+ `.xlsx` opcional) | Parquet snappy / Excel | Se sobrescribe | `step3_exportar_practicantes.py:259-293` |
| PDT-OUT-07 | Gold PRACTICANTES histórico | `gold/historico/ingresos_practicantes_gold_<DD.MM.YYYY_HH.MM.SS>.parquet` (+ `.xlsx` opcional) | Parquet snappy / Excel | Nuevo archivo por ejecución | `step3_exportar_practicantes.py:264-268`, `step3_exportar_practicantes.py:296-309` |

| ID | Regla | Fuente |
|---|---|---|
| PDT-OUT-08 | `gold/` se crea como **hermana** de `silver/` (padre de la carpeta del Parquet Silver); `gold/historico/` dentro de `gold/`. Las carpetas se crean si no existen. | `step2_exportar_ingresos.py:195-202`, `step2_exportar_ingresos.py:354-363`, `src/modules/pdt/ui/worker.py:329-339` |
| PDT-OUT-09 | Timestamp histórico: hora local de la máquina, formato `DD.MM.YYYY_HH.MM.SS`; se calcula una vez por llamada y se comparte entre Parquet y Excel histórico. | `step2_exportar_ingresos.py:205`, `step3_exportar_practicantes.py:268` |
| PDT-OUT-10 | **Excel opcional**: solo se escribe si `export_excel` es verdadero; si no, la ruta devuelta es nula. En el worker el flag viene de una casilla de la UI ("exportar Excel Gold"); en los scripts standalone `main()` no pasa el flag y su valor por defecto es falso → **nunca escriben Excel**. | `src/utils/gold_export.py:23-27`, `src/utils/ui/widgets/base_etl_widget.py:182`, `step2_exportar_ingresos.py:181`, `step2_exportar_ingresos.py:360-363` |
| PDT-OUT-11 | Orden de escritura Gold: Parquet actual → Excel actual → Parquet histórico → Excel histórico. No hay escritura atómica ni rollback (inferido). | `step2_exportar_ingresos.py:218-246` |
| PDT-OUT-12 | Si una hoja se omitió en Step 1 (sin datos o error), su Parquet Silver **no se reescribe**; un archivo de una ejecución anterior queda en disco (inferido). En el worker el paso Gold de esa hoja falla porque la ruta no está en el resultado del Step 1 (PDT-VAL-06); en standalone el usuario podría elegir el archivo anterior. | `step1_consolidar_ingresos.py:351-363`, `src/modules/pdt/ui/worker.py:186-191`, `src/modules/pdt/ui/worker.py:286-289` |

---

## 7. Validaciones y errores

| ID | Situación | Comportamiento | Fuente |
|---|---|---|---|
| PDT-VAL-01 | Worker recibe ≠ 1 archivo | **Aborta** el ETL | `src/modules/pdt/ui/worker.py:76-81` |
| PDT-VAL-02 | Falla cualquier chequeo de preflight (PDT-BRZ-14…20) | **Aborta** el ETL completo antes del Step 1 (etapa "Preflight / Validate Source") | `src/modules/pdt/ui/worker.py:88-89`, `src/modules/pdt/ui/worker.py:626-638` |
| PDT-VAL-03 | Cantidad de encabezados ≠ esperada | **Advierte** y continúa | `step1_consolidar_ingresos.py:154-157` |
| PDT-VAL-04 | Hoja inexistente, vacía o con error de lectura/limpieza | **Advierte**, omite la hoja y continúa con la otra | `step1_consolidar_ingresos.py:406-425`, `src/modules/pdt/ui/worker.py:136-166` |
| PDT-VAL-05 | Ninguna hoja produjo datos | **Aborta** (Step 1) | `step1_consolidar_ingresos.py:428-430`, `src/modules/pdt/ui/worker.py:168-175` |
| PDT-VAL-06 | Worker: error en escritura Silver u otra excepción no capturada por hoja en Step 1 | **Aborta** con error de etapa 1 | `src/modules/pdt/ui/worker.py:220-233` |
| PDT-VAL-07 | Worker Step 2 o Step 3: esquema no encontrado, esquema sin la hoja, Silver no disponible, columnas faltantes, cualquier excepción | **No aborta**: registra el error en el resultado de ese paso y continúa; el resultado global sigue con `success = true`. Si el error es de importación del módulo, se registra como *warning* "Step N no implementado". | `src/modules/pdt/ui/worker.py:249-289`, `src/modules/pdt/ui/worker.py:370-389`, `src/modules/pdt/ui/worker.py:407-452`, `src/modules/pdt/ui/worker.py:540-559`, `src/modules/pdt/ui/worker.py:564` |
| PDT-VAL-08 | Standalone Step 2/3: esquema no encontrado automáticamente | Pide al usuario seleccionarlo; si cancela, termina sin procesar. Busca en `../assets/esquemas/`, `assets/esquemas/`, `../../assets/esquemas/` y el directorio actual, con extensión `.yaml`, `.yml` o `.json`. | `step2_exportar_ingresos.py:38-47`, `step2_exportar_ingresos.py:280-298`, `src/utils/structured_config.py:12` |
| PDT-VAL-09 | Standalone Step 2/3: columnas del esquema faltantes en Silver u otra excepción de procesamiento | **Aborta** con código de salida 1, sin escribir Gold | `step2_exportar_ingresos.py:413-418`, `step3_exportar_practicantes.py:464-469` |
| PDT-VAL-10 | Standalone Step 2/3: `PERIODO` con algún nulo y ≤ 12 periodos únicos | La impresión de métricas une los periodos como texto; un nulo provoca un error (inferido) que **aborta antes de escribir Gold**. El worker no llama a las métricas, así que no le afecta. | `step2_exportar_ingresos.py:158-162`, `step3_exportar_practicantes.py:208-212` |
| PDT-VAL-11 | Valores no convertibles al tipado (enteros, fechas) | **Silencioso**: se vuelven nulos, sin advertencia específica (solo aparecen en el conteo de nulos de las métricas standalone) | `step2_exportar_ingresos.py:125-135` |
| PDT-VAL-12 | Restricciones del esquema (nullable, rangos, patrones, valores permitidos, clave primaria) | **No se validan** (ni abortan ni advierten) | `step2_exportar_ingresos.py:94-147` |

---

## 8. Discrepancias

### 8.1 Steps standalone vs worker de UI

| ID | Discrepancia | Fuente |
|---|---|---|
| DISC-01 | **Imputación SERVICIO/REGIMEN → "Personal_Interno"**: la hace el `main()` del Step 2 standalone, **no el worker**. Desde la UI, los nulos/vacíos de `SERVICIO` y `REGIMEN` llegan sin imputar a Gold, aunque el esquema dice `nullable: false` y declara la regla. | `step2_exportar_ingresos.py:332-342` vs `src/modules/pdt/ui/worker.py:313-317` |
| DISC-02 | **Preflight**: solo el worker lo ejecuta. El standalone no valida hojas obligatorias, celda de inicio ni columnas requeridas. | `src/modules/pdt/ui/worker.py:88-89` |
| DISC-03 | **Métricas Gold** (`generar_metricas_basicas`): solo en standalone. Como consecuencia, el fallo PDT-VAL-10 solo ocurre en standalone. | `step2_exportar_ingresos.py:351`, `step3_exportar_practicantes.py:402` vs `src/modules/pdt/ui/worker.py:313-339`, `src/modules/pdt/ui/worker.py:477-509` |
| DISC-04 | **Exportación Excel Gold**: el worker respeta la casilla de la UI; el standalone nunca escribe Excel. | `src/modules/pdt/ui/worker.py:338`, `step2_exportar_ingresos.py:360-363` |
| DISC-05 | **Severidad de errores en Gold**: en el worker un fallo de Step 2/3 no aborta y el resultado es `success = true`; en standalone aborta con salida 1. | `src/modules/pdt/ui/worker.py:374-389`, `src/modules/pdt/ui/worker.py:564` vs `step2_exportar_ingresos.py:413-418` |
| DISC-06 | **Ubicación del esquema**: el worker lo resuelve con `resolve_structured_path("assets/esquemas/...")` y falla si no existe; el standalone busca en varias rutas relativas y pide al usuario seleccionarlo como último recurso. | `src/modules/pdt/ui/worker.py:249-258` vs `step2_exportar_ingresos.py:38-47,280-298` |
| DISC-07 | **Encadenamiento de archivos**: el worker pasa en memoria las rutas Silver generadas por el Step 1; el standalone requiere que el usuario seleccione el Parquet Silver (puede elegir cualquiera, incluso uno viejo o de la otra hoja). | `src/modules/pdt/ui/worker.py:186-191` vs `step2_exportar_ingresos.py:309-313` |
| DISC-08 | Equivalencias (no son diferencias de datos): el worker llama a `add_month_name_column` directamente en lugar de `agregar_columna_enriquecida` del Step 3; el resultado es idéntico. Bronze → Silver es idéntico (mismas funciones). | `src/modules/pdt/ui/worker.py:484`, `step3_exportar_practicantes.py:191` |

### 8.2 Código vs YAML / documentación

| ID | Discrepancia | Fuente |
|---|---|---|
| DISC-09 | **Columnas del esquema EMPLEADOS ausentes en el Bronze esperado**: `FECHA NACIMIENTO`, `SERVICIO`, `REGIMEN` no están en la lista de 22 columnas esperadas. Como Gold exige todas las columnas del esquema (PDT-GLD-E-02), si el Excel real tiene solo las 22 columnas esperadas, **el Step 2 fallaría**. Solo funciona si el Excel real trae esas columnas adicionales (el Step 1 lee los encabezados reales, no la lista). **Matiz:** Step 1 lee las cabeceras reales del Excel (no la lista `columnas_esperadas`, que solo genera una advertencia si el conteo difiere), así que el fallo ocurre solo si el archivo real carece de esas columnas; dado que el módulo estuvo en uso, es probable que la lista esperada esté desactualizada respecto al layout real (inferido). | `step1_consolidar_ingresos.py:35-58` vs `assets/esquemas/esquema_relacion_ingresos.yaml:35-38,82-89` |
| DISC-10 | **Columnas del esquema PRACTICANTES ausentes en el Bronze esperado**: `Jefe Directo`, `Lugar de Trabajo`, `Modalidad`. El Bronze esperado tiene `Jefe responsable de área` y `Sede de Trabajo` (nombres distintos, posibles equivalentes) y no tiene `Modalidad`. No hay renombrado en el código → con el layout esperado, **el Step 3 fallaría**. **Matiz:** Step 1 lee las cabeceras reales del Excel (no la lista `columnas_esperadas`, que solo genera una advertencia si el conteo difiere), así que el fallo ocurre solo si el archivo real carece de esas columnas; dado que el módulo estuvo en uso, es probable que la lista esperada esté desactualizada respecto al layout real (inferido). | `step1_consolidar_ingresos.py:63-84` vs `assets/esquemas/esquema_ingresos_practicantes.yaml:41-53` |
| DISC-11 | El esquema EMPLEADOS dice que PRACTICANTES "SE MANTIENE SOLO EN SILVER" / "no se procesa a Gold", y el docstring/mensajes del Step 1 y Step 2 también; pero el Step 3 y el worker **sí** generan Gold de PRACTICANTES. | `assets/esquemas/esquema_relacion_ingresos.yaml:104-109`, `step1_consolidar_ingresos.py:332-333`, `step2_exportar_ingresos.py:271-272` vs `step3_exportar_practicantes.py:1-12`, `src/modules/pdt/ui/worker.py:391-559` |
| DISC-12 | **Nombres de archivo Gold**: los YAML indican `gold/Relacion Ingresos EMPLEADOS Gold.parquet` y `gold/Relacion Ingresos PRACTICANTES Gold.parquet`; el código escribe `gold/ingresos_empleados_gold.parquet` e `gold/ingresos_practicantes_gold.parquet`. Los YAML tampoco mencionan el histórico. | `assets/esquemas/esquema_relacion_ingresos.yaml:110-111`, `assets/esquemas/esquema_ingresos_practicantes.yaml:68` vs `step2_exportar_ingresos.py:212`, `step3_exportar_practicantes.py:275` |
| DISC-13 | **CODIGO SAP**: docstring dice "#N/D o Error"; el código anula además `#N/A`, cualquier texto que *contenga* esas subcadenas y cualquier texto que contenga "error" (insensible a mayúsculas). El conteo informado usa un filtro diferente al reemplazo (PDT-SLV-07). | `step1_consolidar_ingresos.py:9`, `step1_consolidar_ingresos.py:242-262` |
| DISC-14 | **Transformaciones declaradas no implementadas**: `string_trimming: true`, `date_format: YYYY-MM-DD`, `null_handling: explicit`, restricciones (`pattern`, `min/max`, `allowed_values`, `primary_key`). | `assets/esquemas/esquema_relacion_ingresos.yaml:90-103`, `assets/esquemas/esquema_ingresos_practicantes.yaml:54-65` |
| DISC-15 | **Preflight vs Step 1 en lectura de encabezados**: el preflight tolera hasta 2 celdas vacías intercaladas y escanea hasta 250 columnas; el Step 1 corta en la primera celda nula. Un encabezado con hueco pasa el preflight pero el Step 1 pierde las columnas posteriores al hueco. | `src/utils/validate_source.py:349-358` vs `step1_consolidar_ingresos.py:141-148` |
| DISC-16 | **Preflight vs esquema**: `required_columns` explícitas anulan la derivación `schema_non_nullable`. Si se derivara, EMPLEADOS exigiría además columnas del esquema no nulas (menos las excluidas `PERIODO`, `SERVICIO`, `REGIMEN`) — resultado equivalente al explícito; en PRACTICANTES también equivalente. La configuración `schema_path`/`extract`/`exclude_columns` es actualmente letra muerta. | `src/utils/validate_source.py:254-256`, `assets/validate_source/ingresos.yaml:21-47` |
| DISC-17 | **Extensiones**: el contrato admite `.xlsm`, la UI no; la UI y el contrato admiten `.xls`, pero la librería de lectura no abre `.xls` (inferido). | `assets/validate_source/ingresos.yaml:4-7`, `src/modules/pdt/ui/widget.py:61` |
| DISC-18 | Comentario "Guardar archivos (en la misma carpeta que el archivo Silver)" es engañoso: Gold se escribe en `../gold/` respecto a la carpeta Silver. | `step2_exportar_ingresos.py:353-363`, `step2_exportar_ingresos.py:195-197` |

---

## 9. Ambigüedades y preguntas abiertas

1. **¿El Excel real trae `FECHA NACIMIENTO`, `SERVICIO`, `REGIMEN` (EMPLEADOS) y `Jefe Directo`, `Lugar de Trabajo`, `Modalidad` (PRACTICANTES)?** Si no, los Steps 2 y 3 fallan (DISC-09/10); la lista `columnas_esperadas` de Step 1 es solo informativa y probablemente está desactualizada. Definir si la reimplementación debe renombrar `Jefe responsable de área`→`Jefe Directo` y `Sede de Trabajo`→`Lugar de Trabajo`, y de dónde sale `Modalidad`.
2. **Imputación SERVICIO/REGIMEN**: ¿se debe aplicar siempre (como dice el esquema y el standalone) o no (como hace la UI hoy)? (DISC-01). ¿Y los valores con solo espacios?
3. **Universidad de Procedencia**: solo existe la regla nulo → `"POR DEFINIR"`; no hay mapeo/normalización de nombres de universidades. ¿Se espera uno? ¿Debe tratarse también `""`/espacios como nulo?
4. **Restricciones del esquema** (no nulos, rangos de AÑO/MES, M/F, patrón de PERIODO, unicidad `PERIODO + DNI` / `PERIODO + Numero Documento`): ¿deben validarse en la reimplementación, y con qué severidad (abortar, advertir, descartar fila)?
5. **AÑO/MES decimales** (`2025.0`) producen `PERIODO` mal formado en Silver y nulos en Gold. ¿Normalizar a entero antes de construir `PERIODO`?
6. **Fechas no estándar**: todo lo que no sea una celda fecha de Excel (p. ej. texto `15/01/2025`) termina nulo en Gold sin aviso. ¿Se requiere soportar más formatos?
7. **Limpieza de PROYECTO y CODIGO SAP** se pierde en Gold porque esas columnas no están en los esquemas. ¿Deben incorporarse a Gold?
8. **Nombres de salidas Gold**: ¿cuál es el contrato correcto para Power BI, el del YAML o el del código? (DISC-12).
9. **NOMBRE_MES en inglés** mientras el resto del dominio está en español: ¿intencional (p. ej. para ordenamiento/formatos de Power BI)?
10. **Fin de datos**: una fila totalmente vacía en medio del rango trunca silenciosamente los datos posteriores. ¿Comportamiento deseado?
11. **Encabezados duplicados o vacíos**: el comportamiento actual (se conserva el último; encabezado `""` posible) no está definido como regla de negocio.
12. **Hojas omitidas**: si una hoja falla en Step 1 queda en disco el Silver anterior. ¿Debe borrarse o marcarse?
13. **Resultado del worker** con `success = true` aunque fallen los Gold: ¿es aceptable o la reimplementación debe reportar éxito parcial/fallo?

No se pudo determinar sin ejecutar ni ver archivos reales: la estructura real de las hojas del Excel productivo (columnas extra existentes), ni el valor exacto que la librería entrega para celdas con error de Excel en un libro en español (se asumió que llega el literal del error como texto).

---

## Decisiones del rework (Plan 8)

Reporte «Altas de personal» (`altas_personal`), hecho con el molde de reportes
([config/README.md](../../config/README.md)). No busca paridad con el legacy.

| Pregunta / regla | Decisión |
|---|---|
| §9.1 columnas reales | El contrato pide solo las columnas que usa el Gold; cada hoja mapea sus encabezados a los mismos nombres canónicos. |
| §9.2 imputación SERVICIO/REGIMEN | No se imputa. |
| §9.3 Universidad «POR DEFINIR» | No se imputa: queda nula. |
| §9.4 restricciones | Las del contrato (clave, tipos, valores de sexo) con mensajes por celda. |
| §9.5 AÑO/MES decimales | El periodo sale del nombre del archivo (`Altas de personal AAAA-MM`), no de columnas. |
| §9.7 PROYECTO y CODIGO SAP | No se leen. |
| §9.8 nombres de salida | Un solo Gold `gold/altas_personal.parquet` con columna `tipo` (`empleado` / `practicante`). |
| §9.9 NOMBRE_MES en inglés | En español, como todo el producto. |
| §9.10 fin de datos | El del lector común (TR-D2, TR-D21). |
| §9.13 éxito parcial | Si un archivo tiene errores no se publica nada (TR-D7). |
| Dos archivos del mismo mes | Detienen el proceso, como la planilla. |

## 10. Referencias

| Archivo | Líneas relevantes | Contenido |
|---|---|---|
| `src/modules/pdt/steps/step1_consolidar_ingresos.py` | 31-86 | Configuración de hojas (fila/columna de encabezados, columnas esperadas) |
| | 109-201 | Lectura de hoja: encabezados, inicio/fin de datos, conversión a texto |
| | 204-295 | Limpieza: PROYECTO, CODIGO SAP, filas vacías, PERIODO |
| | 298-326 | Reporte de calidad |
| | 329-365 | Escritura Silver |
| | 373-468 | Orquestación standalone |
| `src/modules/pdt/steps/step2_exportar_ingresos.py` | 38-47, 71-91 | Búsqueda y carga de esquema EMPLEADOS |
| | 94-147 | Selección y tipado |
| | 150-177 | Métricas |
| | 180-258 | Escritura Gold actual + histórico |
| | 332-348 | Imputación SERVICIO/REGIMEN, NOMBRE_MES |
| `src/modules/pdt/steps/step3_exportar_practicantes.py` | 97-150 | Selección y tipado |
| | 153-181 | Regla Universidad de Procedencia |
| | 184-197 | NOMBRE_MES |
| | 200-240 | Métricas |
| | 243-321 | Escritura Gold |
| | 393-414 | Orden de operaciones standalone |
| `src/modules/pdt/ui/worker.py` | 41-55 | Lazy loader de los 3 steps |
| | 76-91 | Validación de 1 archivo y preflight |
| | 108-233 | Step 1 en worker |
| | 247-389 | Step 2 en worker (sin imputación ni métricas) |
| | 405-559 | Step 3 en worker |
| | 561-650 | Resultado final y manejo de errores |
| `src/modules/pdt/ui/widget.py` | 40-81 | Selección de un único archivo `.xlsx`/`.xls` |
| `src/modules/pdt/ui/config.py` | 22-29 | Metadatos del ETL (`id = pdt`) |
| `src/utils/ui/widgets/base_etl_widget.py` | 174-183 | Carpeta de salida = padre del archivo; flag Excel Gold |
| `src/utils/validate_source.py` | 75-189 | Validación de fuente Excel |
| | 246-278, 339-377 | Columnas requeridas, lectura de encabezados, normalización |
| `src/utils/month_name.py` | 6-48 | Mapeo y columna NOMBRE_MES |
| `src/utils/gold_export.py` | 7-27 | Escritura Excel opcional |
| `src/utils/lazy_loader.py` | 12-110 | Carga perezosa de módulos (sin efecto en datos) |
| `assets/validate_source/ingresos.yaml` | 1-47 | Contrato de preflight |
| `assets/esquemas/esquema_relacion_ingresos.yaml` | 1-114 | Esquema Gold EMPLEADOS v3.0 |
| `assets/esquemas/esquema_ingresos_practicantes.yaml` | 1-69 | Esquema Gold PRACTICANTES v2.0 |
