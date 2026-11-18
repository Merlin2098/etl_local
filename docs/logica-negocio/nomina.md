# Lógica de negocio — Dominio `nomina` (Planilla estándar + enriquecimiento con licencias)

> Especificación agnóstica de lenguaje extraída del código Python legado
> (`HR_Data_Integration_Pipeline`). Rutas relativas a la raíz de ese repo.
> El nombre real del cliente se reemplaza por `<CLIENTE>` (también dentro de nombres de archivo).
> "(inferido)" = deducido del comportamiento de librerías/código, no explícito.
> Dominios relacionados: [`licencias.md`](licencias.md) (Bronze→Silver de licencias) y
> [`nomina-regimen-minero.md`](nomina-regimen-minero.md) (variante minera, código duplicado).

---

## 1. Propósito del dominio

Consolidar las planillas mensuales de remuneraciones (un Excel por mes, exportado por el
sistema de planillas) en una única tabla histórica, normalizada y tipada, con:

- columnas derivadas de periodo (`PERIODO`, `AÑO`, `MES`, `NOMBRE_MES`);
- columnas de subsidios detectadas dinámicamente (los meses traen columnas `DIAS SUB…` / `SUB…`
  variables) y dos columnas totalizadoras (`DIAS SUBSIDIOS TOTAL`, `SUBSIDIOS TOTAL`);
- garantía de existencia de la columna `SUBSIDIO MATERNIDAD`;
- enriquecimiento con los motivos de licencias con/sin goce de haber del mismo empleado y periodo
  (`MOTIVO_CON_GOCE`, `MOTIVO_SIN_GOCE`).

La salida Gold se publica en `gold/actual/` con nombre fijo (consumida por Power BI) y se
sobreescribe en cada corrida.

En la app de escritorio el dominio **solo** se ejecuta como pipeline compuesto de 4 stages
("Nómina con Licencias", sección 11). Los scripts `steps/` también tienen un `main()` standalone
con diálogos Tk (sección 11.6).

---

## 2. Fuentes (Bronze)

### 2.1 Planillas mensuales

| ID | Regla | Fuente |
|---|---|---|
| NOM-BRZ-01 | **Selección de archivos (UI):** el usuario elige una **carpeta**; se toman todos los `*.xlsx` y `*.xls` de la raíz (no recursivo). Se excluyen los que empiezan por `~$` (temporales de Excel), `Planilla <CLIENTE> Consolidado` y `Planilla <CLIENTE> BI_Gold`. `.xlsm` no se recoge por glob aunque el contrato de preflight lo admite. | `src/modules/nomina/ui/widget.py:55-65`, `assets/validate_source/nomina.yaml:3-7` |
| NOM-BRZ-02 | La carpeta debe contener además la subcarpeta `licencias/` con el archivo exacto `CONTROL DE LICENCIAS.xlsx`; si falta, la UI no habilita el proceso. | `src/modules/nomina/ui/widget.py:75-79` |
| NOM-BRZ-03 | **Carpeta de trabajo (`output_dir`)** = carpeta padre del primer archivo seleccionado (= la carpeta elegida). Ahí se crean `silver/` y `gold/actual/`. | `src/utils/ui/widgets/base_etl_widget.py:175` |
| NOM-BRZ-04 | **Patrón de nombre / periodo:** el periodo se extrae **del nombre de archivo**, no de una celda. Regex `(\d{4}-\d{2})`, primera coincidencia en cualquier parte del nombre (búsqueda, no anclada). Nombre esperado documentado: `<CLIENTE>_Planilla YYYY-MM Empleados.xlsx`. No se valida que el año o el mes sean válidos (p. ej. `2025-13` produce `PERIODO="2025-13"`). | `src/modules/nomina/steps/step1_consolidar_planillas.py:105-124` |
| NOM-BRZ-05 | Si el nombre no contiene `\d{4}-\d{2}` el archivo se **omite** (advertencia "No se pudo extraer periodo") y se continúa con el resto. El contrato de preflight de nómina **no** define `filename_regex`, por lo que este caso no se detecta antes. | `src/modules/nomina/steps/step1_consolidar_planillas.py:283-288`, `assets/validate_source/nomina.yaml:1-30` |
| NOM-BRZ-06 | **Hoja:** se lee exclusivamente la hoja llamada exactamente `Planilla`. Si no existe → error para ese archivo ("La hoja 'Planilla' no existe…"). Se leen valores calculados (no fórmulas). | `src/modules/nomina/steps/step1_consolidar_planillas.py:142-150` |
| NOM-BRZ-07 | **Encabezados:** fila **6** (1-based), desde la columna A, recorriendo columnas 1..199. Cada valor no nulo se convierte a texto, se recorta (`trim`) y se colapsan los espacios internos múltiples a uno (`\s+` → `" "`). Mayúsculas/minúsculas se conservan. | `src/modules/nomina/steps/step1_consolidar_planillas.py:152-160` |
| NOM-BRZ-08 | **Fin de encabezados:** se detiene cuando el índice de columna actual (1-based) supera `nº_encabezados_leídos + 3` estando en una celda vacía. Con encabezados contiguos equivale a "3 celdas vacías seguidas". Las celdas de encabezado vacías intermedias **no** generan columna (ver NOM-BRZ-11). | `src/modules/nomina/steps/step1_consolidar_planillas.py:161-164` |
| NOM-BRZ-09 | **Datos:** desde la fila **7** hasta el final de la hoja. Se descarta una fila si: (a) todas sus celdas son nulas; (b) la primera celda (columna A) es nula; (c) la primera celda es texto de más de 20 caracteres; (d) es fila de totales/subtotales (NOM-BRZ-10). No hay detección de "fin de datos": se recorren todas las filas. | `src/modules/nomina/steps/step1_consolidar_planillas.py:166-181` |
| NOM-BRZ-10 | **Filas de totales/subtotales:** si la primera celda es texto, se le quitan los caracteres iniciales `+`, `-`, `*` y espacios (`^[+\-*\s]+`), se normaliza (trim + colapsar espacios + casefold) y se descarta la fila si **contiene** alguno de: `ingresos del mes`, `descuentos del mes`, `aportaciones del mes`, `total ingresos`, `total descuentos`, `total aportaciones`, `subtotal`, `totales`. Ej.: `+ INGRESOS DEL MES` se descarta. Si la primera celda no es texto, nunca se considera total. | `src/modules/nomina/steps/step1_consolidar_planillas.py:19-28,81-102`, `tests/test_nomina_subsidio_maternidad.py:33-81` |
| NOM-BRZ-11 | **Alineación posicional:** de cada fila se toman las primeras `N` celdas (N = nº de encabezados) y la celda i se asigna al encabezado i. Como los encabezados vacíos se saltan sin dejar hueco, un encabezado vacío intermedio desalinea todas las columnas siguientes (comportamiento legado, probablemente no deseado). | `src/modules/nomina/steps/step1_consolidar_planillas.py:181,186-189` |
| NOM-BRZ-12 | **Encabezados duplicados:** al construir el diccionario columna→valores, si dos encabezados normalizados son idénticos sobrevive **un solo** nombre de columna con los valores de la **última** aparición (inferido de la semántica de diccionario). | `src/modules/nomina/steps/step1_consolidar_planillas.py:186-189` |
| NOM-BRZ-13 | **Preflight (contrato `nomina`):** para cada archivo seleccionado se verifica: existe; extensión ∈ {`.xlsx`, `.xlsm`, `.xls`} (case-insensitive); se puede abrir; existe la hoja `Planilla`; la celda A6 no está vacía ("Expected header at row 6 col A … empty/shifted"); y en la fila 6 están las columnas obligatorias `APELLIDO PATERNO`, `NOMBRES`, `TIPO DE DOCUMENTO`, `DNI/CEX`. Comparación de encabezados: trim + colapsar espacios + **mayúsculas** (case-insensitive). Lectura de encabezados del preflight: desde A, hasta 250 columnas, se detiene tras 3 vacías consecutivas una vez hallado algún encabezado. `enforce_allowed_sheets: false` (hojas extra permitidas). | `assets/validate_source/nomina.yaml:1-30`, `src/utils/validate_source.py:75-189,339-373` |
| NOM-BRZ-14 | **Precedencia en el contrato:** como `required_columns` explícito está presente, se usa tal cual y se **ignoran** `schema_path`, `extract: schema_non_nullable` y `exclude_columns`. (Hoy el resultado coincide: las no-nulables del esquema menos PERIODO/AÑO/MES son exactamente esas 4.) | `src/utils/validate_source.py:254-256`, `assets/esquemas/esquema_nominas.yaml:8-45` |
| NOM-BRZ-15 | Cualquier error de preflight en **cualquier** archivo aborta toda la corrida antes de ejecutar stages (se agregan todos los errores en un único mensaje). | `src/modules/nomina/ui/worker.py:65-72,166-176`, `src/utils/validate_source.py:192-215` |

### 2.2 Archivo de licencias

Ruta fija `<output_dir>/licencias/CONTROL DE LICENCIAS.xlsx`, validado con el contrato
`licencias` antes de iniciar (`src/modules/nomina/ui/worker.py:74-81`). Estructura y reglas en
[`licencias.md`](licencias.md).

---

## 3. Reglas Bronze → Silver (stage 1: `consolidar_archivos`)

| ID | Regla | Fuente |
|---|---|---|
| NOM-SLV-01 | **Entrada:** lista de rutas (modo pipeline) o una carpeta (modo standalone: glob `*.xlsx` + `*.xls`, excluyendo `~$*` y `Planilla <CLIENTE> Consolidado*`; si no hay archivos → error). Otro tipo de entrada → error "Formato de entrada inválido". | `src/modules/nomina/steps/step1_consolidar_planillas.py:235-266` |
| NOM-SLV-02 | **Tolerancia por archivo:** un error al leer un archivo (hoja ausente, etc.) se registra y se continúa con los demás. Si **ningún** archivo se procesa → error "No se pudo procesar ningún archivo correctamente" (aborta el stage). | `src/modules/nomina/steps/step1_consolidar_planillas.py:278-309` |
| NOM-SLV-03 | **Columna `PERIODO`:** se añade a cada archivo con el valor `YYYY-MM` extraído del nombre (NOM-BRZ-04) y se coloca como primera columna. | `src/modules/nomina/steps/step1_consolidar_planillas.py:203-210` |
| NOM-SLV-04 | **Garantía `SUBSIDIO MATERNIDAD`** (por archivo y otra vez tras concatenar): se buscan columnas cuyo encabezado normalizado (trim + colapsar espacios + casefold) sea `subsidio maternidad`. (a) Ninguna → se crea `SUBSIDIO MATERNIDAD` con todos sus valores nulos (texto). (b) Existe la columna con el nombre exacto → se queda con `coalesce(exacta, variante1, variante2, …)` y se eliminan las variantes. (c) Solo variantes → la primera se renombra a `SUBSIDIO MATERNIDAD` (conserva posición) y las demás se fusionan por `coalesce` y se eliminan. | `src/modules/nomina/steps/step1_consolidar_planillas.py:18,31-78,206,344`, `tests/test_nomina_subsidio_maternidad.py:15-30` |
| NOM-SLV-05 | **Tipos uniformes:** antes de unir los meses, **todas** las columnas excepto `PERIODO` se convierten a texto (conversión no estricta). Las fechas leídas del Excel quedan como texto con formato `YYYY-MM-DD HH:MM:SS[.ffffff]` (inferido) y los números como su representación textual (`30` / `30.0`). | `src/modules/nomina/steps/step1_consolidar_planillas.py:330-339` |
| NOM-SLV-06 | **Unión de meses:** concatenación "diagonal" (unión por nombre de columna; columnas ausentes en un mes quedan nulas). El orden de columnas resultante sigue el orden de primera aparición (inferido). | `src/modules/nomina/steps/step1_consolidar_planillas.py:343` |
| NOM-SLV-07 | **Filtro de nulos:** si existe la columna `DNI/CEX`, se eliminan las filas con `DNI/CEX` nulo (no se eliminan cadenas vacías). Si la columna no existe, solo advertencia. | `src/modules/nomina/steps/step1_consolidar_planillas.py:356-366` |
| NOM-SLV-08 | **Derivadas:** `AÑO = substring(PERIODO, 1, 4)` (texto); `MES = entero(substring(PERIODO, 6, 2))` (entero 32 bits; si no convierte → nulo). | `src/modules/nomina/steps/step1_consolidar_planillas.py:372-377` |
| NOM-SLV-09 | **Orden de columnas Silver:** `PERIODO, AÑO, MES` y luego el resto en su orden existente. | `src/modules/nomina/steps/step1_consolidar_planillas.py:380-383` |
| NOM-SLV-10 | **Sin deduplicación** en Silver: si dos archivos traen el mismo periodo (o un empleado aparece dos veces), las filas se conservan. | `src/modules/nomina/steps/step1_consolidar_planillas.py:218-408` (ausencia) |
| NOM-SLV-11 | **No** se convierte a nulo la cadena vacía ni se recortan espacios en Silver (solo los encabezados se normalizan). | `src/modules/nomina/steps/step1_consolidar_planillas.py:326-383` (ausencia) |

---

## 4. Reglas Silver → Gold (stage 3: `exportar_a_gold`)

Esquema: `assets/esquemas/esquema_nominas.yaml` (versión `2.4`), resuelto por
`resolve_structured_path("assets/esquemas/esquema_nominas")`; si no existe → error.

| ID | Regla | Fuente |
|---|---|---|
| NOM-GLD-01 | **Selección:** se toman, en el **orden del esquema**, solo las columnas del bloque `schema` que existan en Silver. Las faltantes se informan (advertencia) pero no abortan. Los nombres se comparan **exactos** (sensibles a mayúsculas/acentos); no hay renombrado. | `src/modules/nomina/steps/step2_exportar.py:33-51` |
| NOM-GLD-02 | **Columnas del esquema estático (36):** `PERIODO, AÑO, MES, CÓDIGO CORPORATIVO, APELLIDO PATERNO, APELLIDO MATERNO, NOMBRES, TIPO DE DOCUMENTO, DNI/CEX, SEXO, F. NACIMIENTO, ÁREA, CENTRO DE COSTOS, FECHA DE INGRESO, FECHA CESE, MOTIVO CESE, DIAS SUBSIDIO ENFERMEDAD, DIAS GOCE HABER(D), # HRS EXTRAS SIMP.25%, # HRS EXTRAS SIMP.35%, #HRS EXTRAS DOBLES, #HRS SIMP.50%, HRS SIMPLES AL 50%, HORAS EXTRAS 25%, HORAS EXTRAS 35%, HORAS EXTRAS 100%, LICENCIA GOCE HABER, LICENCIA PATERNIDAD, DIAS DIST.LIC.S.GH, DIAS LIC. PATE (P), DIAS DESC MEDICO (P), DESCANSO MEDICO, SUBS. POR ENFERMEDAD, SUBS. POR ACCIDENTE, SUBSIDIO MATERNIDAD`. Tipos y nulabilidad: ver esquema. | `assets/esquemas/esquema_nominas.yaml:7-178` |
| NOM-GLD-03 | **Conversión de tipos (estricta por columna):** `string`→texto; `integer`→entero 64; `float`→decimal 64; `date`→ parsear texto con formato `%Y-%m-%d %H:%M:%S%.f` (no estricto: lo que no parsea queda nulo) y truncar a fecha. Para `integer`/`float` la conversión es **estricta**: si **un solo** valor no convierte (p. ej. `"30.0"` a entero, o `""`), falla la columna entera, se imprime advertencia y la columna **queda como texto sin convertir** (inferido para los casos concretos). | `src/modules/nomina/steps/step2_exportar.py:55-74` |
| NOM-GLD-04 | **Recorte de espacios:** después de convertir, a toda columna de tipo texto se le aplica `trim` (ambos extremos). Las cadenas vacías se mantienen como `""`, no como nulo. | `src/modules/nomina/steps/step2_exportar.py:77-80` |
| NOM-GLD-05 | **`NOMBRE_MES`:** se agrega inmediatamente después de `MES` con el nombre del mes en **inglés** y capitalizado (`January`…`December`). Valores fuera de 1..12 o nulos → nulo. Si `MES` no existe → error (aborta). | `src/modules/nomina/steps/step2_exportar.py:85-94`, `src/utils/month_name.py:6-48`, `tests/test_month_name.py:14-35` |
| NOM-GLD-06 | **Columnas dinámicas — definición:** `dynamic_columns` es una lista ordenada de grupos `{pattern, total_column, include_static_matches}`. Grupo 1: `pattern: ^DIAS SUB` → `DIAS SUBSIDIOS TOTAL`. Grupo 2: `pattern: ^SUB` → `SUBSIDIOS TOTAL`. Ambos con `include_static_matches: true`. Las claves `type`, `nullable`, `description` del grupo **no se usan**. | `assets/esquemas/esquema_nominas.yaml:193-205`, `src/modules/nomina/steps/step2_exportar.py:199-214` |
| NOM-GLD-07 | **Columnas dinámicas — detección:** el regex se aplica **anclado al inicio** del nombre de columna y es **sensible a mayúsculas**. Una columna de **Silver** es dinámica para el grupo si: coincide con el patrón, **no** está en el bloque `schema` y **no** fue reclamada por un grupo anterior (exclusión mutua por orden de definición; se reclaman solo las dinámicas). | `src/modules/nomina/steps/step2_exportar.py:216-235` |
| NOM-GLD-08 | **Columnas estáticas incluidas (`include_static_matches`):** si es `true`, también participan en la suma las columnas **ya presentes en Gold** que coinciden con el patrón (y no fueron reclamadas como dinámicas). Con el esquema actual: grupo 1 ← `DIAS SUBSIDIO ENFERMEDAD`; grupo 2 ← `SUBS. POR ENFERMEDAD`, `SUBS. POR ACCIDENTE`, `SUBSIDIO MATERNIDAD`. Las estáticas no se "reclaman" (lectura compartida). | `src/modules/nomina/steps/step2_exportar.py:224-235` |
| NOM-GLD-09 | **Valores de dinámicas:** cada columna dinámica se copia desde Silver convertida a decimal 64 en modo **no estricto** (no numérico → nulo). No se recortan ni se validan contra el esquema. | `src/modules/nomina/steps/step2_exportar.py:248-252` |
| NOM-GLD-10 | **Fórmula de totales:** para cada grupo con al menos una columna: `total = Σ COALESCE(CAST(col AS DOUBLE), 0.0)` sobre `estáticas ∪ dinámicas` del grupo (por fila). Nunca es nulo si el grupo tiene columnas. Si el grupo no tiene ninguna columna → `total = NULL` (tipo DOUBLE) para todas las filas. Pseudo-SQL abajo. | `src/modules/nomina/steps/step2_exportar.py:269-279` |
| NOM-GLD-11 | **Orden de columnas Gold:** (1) columnas "neutras" (las que no participan en ningún grupo) en su orden actual: orden del esquema + `NOMBRE_MES` tras `MES`; (2) por cada grupo, en orden de definición, un bloque contiguo: estáticas (orden Gold) + dinámicas (orden Silver) + columna total. Resultado típico: `… LICENCIA PATERNIDAD, DIAS DIST.LIC.S.GH, DIAS LIC. PATE (P), DIAS DESC MEDICO (P), DESCANSO MEDICO, DIAS SUBSIDIO ENFERMEDAD, <DIAS SUB… dinámicas>, DIAS SUBSIDIOS TOTAL, SUBS. POR ENFERMEDAD, SUBS. POR ACCIDENTE, SUBSIDIO MATERNIDAD, <SUB… dinámicas>, SUBSIDIOS TOTAL`. (Nota: `DIAS SUBSIDIO ENFERMEDAD` se desplaza desde su posición del esquema al bloque del grupo 1.) | `src/modules/nomina/steps/step2_exportar.py:259-287` |
| NOM-GLD-12 | **`LICENCIA PATERNIDAD`:** columna estática del esquema (`float`, nullable, `min_value: 0`). No tiene lógica especial: se preserva si existe en Silver; no coincide con ningún patrón dinámico (empieza por `LIC`). | `assets/esquemas/esquema_nominas.yaml:133-137`, `tests/test_nomina_gold_licencia_paternidad.py:8-27` |
| NOM-GLD-13 | **`SUBSIDIO MATERNIDAD` en Gold:** siempre existe en Silver (NOM-SLV-04), por eso siempre está en Gold y siempre participa en `SUBSIDIOS TOTAL` (aporta 0 cuando es nula). | `src/modules/nomina/steps/step1_consolidar_planillas.py:344`, `assets/esquemas/esquema_nominas.yaml:173-178,200-205` |
| NOM-GLD-14 | **Validación de constraints** (`validar_constraints`) sobre el Gold final: ver sección 7. **Nunca aborta**: errores y advertencias solo se imprimen. | `src/modules/nomina/steps/step2_exportar.py:97-188,408-419` |

Pseudo-SQL de NOM-GLD-10/11 (por grupo `g` con columnas `c1..cn`):

```sql
SELECT <neutras...>,
       c1, …, cn,  (COALESCE(CAST(c1 AS DOUBLE),0.0) + … + COALESCE(CAST(cn AS DOUBLE),0.0)) AS "<total_g>",
       -- si el grupo no tiene columnas:  NULL::DOUBLE AS "<total_g>"
FROM gold
```

---

## 5. Reglas de negocio

| ID | Regla | Fuente |
|---|---|---|
| NOM-GLD-15 | **Clave lógica:** `(PERIODO, DNI/CEX)` (declarada como `primary_key`). Solo se verifica, no se impone (duplicados se conservan). | `assets/esquemas/esquema_nominas.yaml:179-182`, `src/modules/nomina/steps/step2_exportar.py:105-126` |
| NOM-GLD-16 | **Reglas declarativas no implementadas** (`business_rules`): "FECHA CESE debe ser posterior a FECHA DE INGRESO si ambas existen"; "Si FECHA CESE existe, el empleado está inactivo"; "MOTIVO CESE solo debe tener valor si FECHA CESE existe". Ningún código las evalúa ni deriva un estado activo/inactivo. | `assets/esquemas/esquema_nominas.yaml:184-187` |
| NOM-GLD-17 | **Metadatos no implementados:** `max_value` (p. ej. `MES` ≤ 12), `precision` (2 decimales), `unique_keys`, y el bloque `transformations` (`column_naming`, `date_format`, `string_trimming`, `null_handling`) no se leen. El recorte de espacios se hace siempre, independientemente de `string_trimming`. | `assets/esquemas/esquema_nominas.yaml:16-21,111,179-192`, `src/modules/nomina/steps/step2_exportar.py:26-188` |
| NOM-ENR-01 | **Enriquecimiento con licencias:** por cada fila de nómina se agrega `MOTIVO_CON_GOCE` y `MOTIVO_SIN_GOCE` = concatenación con `" \| "` de los `MOTIVO` de licencias del mismo `DNI/CEX` y `PERIODO`, ordenados alfabéticamente, filtrando `TIPO_LICENCIA = 'CON_GOCE'` / `'SIN_GOCE'`. LEFT JOIN: toda fila de nómina se conserva; sin licencias → nulo. Motivos repetidos **no** se deduplican. Igualdad exacta de texto en ambas claves. SQL completo en `assets/queries/query_licencias_agregadas.sql`. | `assets/queries/query_licencias_agregadas.sql:18-46` |
| NOM-ENR-02 | La agregación previa por `(DNI/CEX, PERIODO)` garantiza que el join **no multiplica** filas de nómina. Las columnas nuevas se agregan **al final** (`n.*` + `MOTIVO_CON_GOCE` + `MOTIVO_SIN_GOCE`). El orden de filas de salida no está garantizado por el motor (inferido). | `assets/queries/query_licencias_agregadas.sql:36-46` |

Pseudocódigo NOM-ENR-01:

```
para cada fila n de nomina:
  cg = licencias donde DNI/CEX = n.DNI/CEX y PERIODO = n.PERIODO y TIPO_LICENCIA = 'CON_GOCE'
  n.MOTIVO_CON_GOCE = join(" | ", ordenar(cg.MOTIVO)) si cg no vacío, si no NULL
  (ídem SIN_GOCE → MOTIVO_SIN_GOCE)
```

---

## 6. Salidas

| ID | Regla | Fuente |
|---|---|---|
| NOM-SLV-12 | **Silver nómina:** `<output_dir>/silver/Planilla_<CLIENTE>_Consolidado_Silver.parquet`. Nombre fijo, sin timestamp, se **sobreescribe**. La carpeta `silver/` se crea si no existe. Si la escritura falla se reintenta una vez con compresión `snappy`; si falla de nuevo → error. Tras escribir se verifica que exista. | `src/modules/nomina/steps/step1_consolidar_planillas.py:394-454` |
| NOM-GLD-18 | **"Versionamiento" Gold (`gestionar_versionamiento_gold`):** pese al nombre, **no hay versiones ni histórico**: solo crea `<base>/gold/actual/` (base = carpeta padre de `silver/`) y la devuelve. | `src/modules/nomina/steps/step2_exportar.py:360-379,437-440` |
| NOM-GLD-19 | **Gold nómina:** `gold/actual/Planilla_<CLIENTE>_Consolidado.parquet` (se sobreescribe). Error de escritura → aborta. | `src/modules/nomina/steps/step2_exportar.py:443,452-463` |
| NOM-GLD-20 | **Excel opcional de visualización:** solo si `export_excel_gold = true` (casilla de la UI). Ruta `gold/actual/Planilla_<CLIENTE>_Consolidado.xlsx`. Formato: hoja `Planilla Gold`; fila 1 = encabezados (negrita, blanco, Calibri 11, relleno `#366092`, centrado con ajuste de texto); bordes finos `#D3D3D3`; números alineados a la derecha, resto a la izquierda; ancho de columna = `min(longitud_máx_texto + 2, 50)`; panel congelado en `A2`. Un error al generar el Excel **solo advierte** (resultado `excel = null`). | `src/modules/nomina/steps/step2_exportar.py:297-357,444,465-483`, `src/utils/gold_export.py:7-27` |
| NOM-ENR-03 | **Gold enriquecido (stage 4):** sobreescribe **el mismo** archivo Gold base `gold/actual/Planilla_<CLIENTE>_Consolidado.parquet` (snappy) con la versión enriquecida, y escribe una copia idéntica `gold/actual/Planilla_<CLIENTE>_BI_Gold_Con_Licencias.parquet`. Si `export_excel_gold`, escribe también ambos `.xlsx` con un volcado **plano** (sin el formato de NOM-GLD-20), sobreescribiendo el Excel formateado del stage 3. | `src/modules/licencias/steps/step2_enriquecer_nomina.py:268-367`, `src/orchestrators/pipeline_nomina_executor.py:302-317` |
| NOM-ENR-04 | Tras una corrida completa **no queda** un Gold "sin licencias": el base queda enriquecido. Todos los archivos tienen nombre fijo y se sobreescriben (sin timestamp). | `src/modules/licencias/steps/step2_enriquecer_nomina.py:339-341` |

---

## 7. Validaciones y errores

| ID | Situación | Efecto | Fuente |
|---|---|---|---|
| NOM-GLD-21 | Duplicados en `(PERIODO, DNI/CEX)` (solo sobre las columnas PK presentes) | "error crítico" **impreso**, no aborta | `src/modules/nomina/steps/step2_exportar.py:114-126,411-414` |
| NOM-GLD-22 | Nulos en columna `nullable: false` (`PERIODO, AÑO, MES, APELLIDO PATERNO, NOMBRES, TIPO DE DOCUMENTO, DNI/CEX`) | "error crítico" impreso, no aborta | `src/modules/nomina/steps/step2_exportar.py:129-143` |
| NOM-GLD-23 | Columna PK ausente | advertencia; se valida con las PK presentes | `src/modules/nomina/steps/step2_exportar.py:109-112` |
| NOM-GLD-24 | Valor fuera de `allowed_values` (solo `SEXO ∈ {M, F}`; nulos ignorados) | advertencia con hasta 5 valores distintos | `src/modules/nomina/steps/step2_exportar.py:146-164` |
| NOM-GLD-25 | Valor `< min_value` (nulos ignorados) | advertencia con conteo | `src/modules/nomina/steps/step2_exportar.py:167-184` |
| — | Preflight fallido (nómina o licencias) | aborta antes de cualquier stage | NOM-BRZ-15 |
| — | Archivo sin periodo en el nombre / ilegible | se omite ese archivo; aborta solo si no queda ninguno | NOM-BRZ-05, NOM-SLV-02 |
| — | Esquema Gold no encontrado; `MES` ausente; fallo al escribir parquet | aborta el stage | `src/modules/nomina/steps/step2_exportar.py:536-539,452-463`, `src/utils/month_name.py:33-36` |
| — | Fallo al generar Excel de visualización (stage 3) | advertencia | NOM-GLD-20 |
| — | Output requerido no generado (validación del pipeline) | stage fallido → pipeline detenido | sección 11 |

Las columnas dinámicas y los totales **no** se validan (la validación solo recorre las claves de `schema`).

---

## 8. Discrepancias

| ID | Discrepancia | Fuente |
|---|---|---|
| NOM-DIF-01 | **Comparación de encabezados inconsistente:** el preflight compara sin distinguir mayúsculas (`UPPER`), pero Silver/Gold seleccionan columnas por nombre **exacto**. Un archivo con `Dni/Cex` pasa el preflight y luego: no se filtra por DNI nulo (advertencia), no entra a Gold y el enriquecimiento falla (columna inexistente). Solo `SUBSIDIO MATERNIDAD` tiene comparación tolerante. | `src/utils/validate_source.py:372-373`, `src/modules/nomina/steps/step2_exportar.py:35`, `src/modules/nomina/steps/step1_consolidar_planillas.py:31-33` |
| NOM-DIF-02 | **Excel formateado sobrescrito:** en el pipeline, el stage 3 genera el Excel con formato de visualización y el stage 4 lo reemplaza en la misma ruta con un volcado plano. El formato de NOM-GLD-20 nunca llega al usuario en el pipeline compuesto. | `src/modules/nomina/steps/step2_exportar.py:466-471`, `src/modules/licencias/steps/step2_enriquecer_nomina.py:343-348` |
| NOM-DIF-03 | **`validar_constraints` llama "errores críticos" a lo que no bloquea** (comentario: "solo warnings, no detiene ejecución"). | `src/modules/nomina/steps/step2_exportar.py:408-419` |
| NOM-DIF-04 | **Parámetros del YAML vacíos:** el YAML declara `params:` sin contenido; el executor ignora el YAML para los parámetros y los inyecta por **índice de stage** (0..3). Reordenar stages en el YAML rompe el pipeline. | `src/orchestrators/pipelines/pipeline_nomina_licencias.yaml:16-50`, `src/orchestrators/pipeline_nomina_executor.py:268-320` |
| NOM-DIF-05 | **Mensajes/docstrings vs. código:** el resumen del worker dice `Silver: Planilla <CLIENTE> Consolidado.parquet` (real: `Planilla_<CLIENTE>_Consolidado_Silver.parquet`) y "Gold base" (real: el base queda enriquecido). Docstrings del enriquecimiento hablan de `gold/nomina/actual/` (real: `gold/actual/`). | `src/modules/nomina/ui/worker.py:142-147`, `src/modules/licencias/steps/step2_enriquecer_nomina.py:7-9,143-145` |
| NOM-DIF-06 | **Filtros de exclusión de archivos** distintos: el widget excluye `Planilla <CLIENTE> Consolidado*` y `Planilla <CLIENTE> BI_Gold*`; el step standalone solo el primero. Ninguno coincide con los nombres reales de salida (con guion bajo), aunque éstos viven en subcarpetas y no afectan el glob de la raíz. | `src/modules/nomina/ui/widget.py:59-65`, `src/modules/nomina/steps/step1_consolidar_planillas.py:254-259` |
| NOM-DIF-07 | **Doble validación de estructura:** la carpeta `licencias/` se valida en el widget, en el preflight del worker y otra vez en `validate_structure` del executor. | `src/modules/nomina/ui/widget.py:75-79`, `src/modules/nomina/ui/worker.py:74-81`, `src/orchestrators/pipeline_nomina_executor.py:131-156` |
| NOM-DIF-08 | **Escaneo de encabezados:** step lee columnas 1..199 con regla "índice > n+3"; preflight lee hasta 250 columnas con "3 vacías seguidas". Pueden diferir con huecos o > 199 columnas. | `src/modules/nomina/steps/step1_consolidar_planillas.py:154-164`, `src/utils/validate_source.py:339-360` |
| NOM-DIF-09 | **Duplicación con régimen minero:** step1/step2 están copiados casi íntegros en `nomina_regimen_minero`, pero la copia minera **no** tiene filtro de totales, garantía de `SUBSIDIO MATERNIDAD`, columnas dinámicas ni `validar_constraints`, y usa casts no estrictos. Detalle en `nomina-regimen-minero.md` §8. | `src/modules/nomina_regimen_minero/steps/*.py` |
| NOM-DIF-10 | **`exportar_a_gold` no aborta si falta el Silver:** solo imprime un bloque DEBUG y luego falla al leer el parquet (el error final es el de lectura). La copia minera sí lanza `FileNotFoundError` explícito. | `src/modules/nomina/steps/step2_exportar.py:512-528` |
| NOM-DIF-11 | Metadatos del esquema: `fecha_actualizacion` (`2025-01-05`) anterior a `fecha_creacion` (`2025-12-29`). | `assets/esquemas/esquema_nominas.yaml:4-5` |

---

## 9. Ambigüedades y preguntas abiertas

1. **Normalización de `DNI/CEX`:** nómina no quita sufijos `.0` (si Excel guarda el DNI como número decimal queda `"12345678.0"`), mientras licencias sí los quita. ¿Puede haber DNI numéricos en la planilla? Si los hay, el join de licencias falla silenciosamente (motivos nulos). Tampoco se rellenan ceros a la izquierda.
2. **Conversión estricta (NOM-GLD-03):** ¿es deseado que un único valor no numérico deje toda la columna como texto? En ese caso `validar_constraints` compara texto con `min_value` numérico, lo que probablemente lanza excepción y aborta el stage (inferido), y la suma de totales con `CAST(texto AS DOUBLE)` fallaría si hay texto no numérico (inferido). Para la reimplementación conviene decidir: ¿nulo por valor inválido o error?
3. **Encabezados vacíos intermedios / duplicados** (NOM-BRZ-11/12): ¿ocurren en los archivos reales? El comportamiento legado desalinea o pierde columnas.
4. **Formato de fechas:** el parseo asume `YYYY-MM-DD HH:MM:SS[.f]` (texto generado desde fechas Excel). Fechas guardadas como texto en otro formato (`dd/mm/yyyy`) quedan nulas sin aviso.
5. **Periodo duplicado:** si se seleccionan dos archivos del mismo mes, se duplican filas; solo se detecta como "error crítico" impreso de PK. ¿Debería abortar o quedarse con el más reciente?
6. **Patrones dinámicos sensibles a mayúsculas:** una columna `Subsidio X` (minúsculas) no entra a ningún total. ¿Intencional?
7. **Columnas estáticas compartidas:** como las estáticas no se reclaman, si un día un patrón posterior también coincide con una estática ya usada, aparecería dos veces en el SELECT (columna duplicada → error probable). Con los patrones actuales no ocurre.
8. **Fila de totales con primera celda numérica** (p. ej. la primera columna es `N°` y la fila de totales trae un número): no se detecta; solo la cubre el filtro de `DNI/CEX` nulo.
9. **NaN en Excel:** la conversión a tabla intermedia para el Excel formateado puede escribir nulos numéricos como `NaN` (inferido); no verificado.

---

## 10. Referencias

- `src/modules/nomina/steps/step1_consolidar_planillas.py:18-454` — lectura Bronze, totales, SUBSIDIO MATERNIDAD, Silver.
- `src/modules/nomina/steps/step2_exportar.py:26-583` — Gold, tipos, dinámicas, constraints, Excel, salidas.
- `src/modules/nomina/ui/widget.py:40-113`, `src/modules/nomina/ui/worker.py:39-186`, `src/modules/nomina/ui/config.py:23-30`.
- `src/orchestrators/pipeline_nomina_executor.py:17-433`, `src/orchestrators/pipelines/pipeline_nomina_licencias.yaml:1-63`.
- `src/modules/licencias/steps/step2_enriquecer_nomina.py:268-384`, `assets/queries/query_licencias_agregadas.sql:18-46`.
- `assets/validate_source/nomina.yaml:1-30`, `src/utils/validate_source.py:75-377`.
- `assets/esquemas/esquema_nominas.yaml:1-205`.
- `src/utils/month_name.py:6-48`, `src/utils/gold_export.py:7-27`.
- `tests/test_nomina_subsidio_maternidad.py:15-81`, `tests/test_nomina_gold_licencia_paternidad.py:8-27`, `tests/test_gold_export.py:72-95`, `tests/test_month_name.py:14-35`.

---

## 11. Pipeline compuesto "Nómina con Licencias"

### 11.1 Disparo y preflight

1. El worker verifica que haya archivos; si no, error "No se seleccionaron archivos de nómina".
2. Preflight `nomina` sobre todas las planillas y preflight `licencias` sobre
   `<output_dir>/licencias/CONTROL DE LICENCIAS.xlsx`. Cualquier fallo → aborta (NOM-BRZ-15).
3. Si el YAML del pipeline no existe → error.
4. El executor carga el YAML (`name`, `version`, `stages`, `config.stop_on_error`) y valida de nuevo
   que exista `licencias/CONTROL DE LICENCIAS.xlsx`; si no → "Estructura de carpetas inválida (falta /licencias/)".

Fuentes: `src/modules/nomina/ui/worker.py:56-108`, `src/orchestrators/pipeline_nomina_executor.py:110-156,347-365`.

### 11.2 Stages y parámetros inyectados

El YAML solo aporta `module`, `function`, `name` y `outputs`; **los `params` están vacíos**. El
executor resuelve la función por import dinámico y construye los argumentos **según la posición**
del stage (`stage_index`), no según el YAML:

| # | Stage (YAML) | Función | Parámetros que inyecta el executor |
|---|---|---|---|
| 0 | Nómina Bronze → Silver | `nomina.step1.consolidar_archivos` | `archivos` = lista seleccionada; `carpeta_trabajo` = `output_dir` |
| 1 | Licencias Bronze → Silver | `licencias.step1.procesar_sin_gui` | `ruta_archivo` = `output_dir/licencias/CONTROL DE LICENCIAS.xlsx`; `carpeta_salida` = `output_dir/silver` |
| 2 | Nómina Silver → Gold | `nomina.step2.exportar_a_gold` | `ruta_parquet_silver` = `output_dir/silver/Planilla_<CLIENTE>_Consolidado_Silver.parquet`; `carpeta_trabajo` = `output_dir`; `export_excel_gold` = casilla UI |
| 3 | Enriquecimiento con Licencias | `licencias.step2.procesar_sin_gui` | `ruta_nomina` = `output_dir/gold/actual/Planilla_<CLIENTE>_Consolidado.parquet`; `ruta_licencias` = `output_dir/silver/licencias_consolidadas.parquet`; `export_excel_gold`; (`nombre_output` no se pasa → default `Planilla_<CLIENTE>_BI_Gold_Con_Licencias`) |

Los stages se comunican **solo a través de archivos** con rutas fijas; los valores de retorno se
guardan en `stages_results` pero no se usan como entrada de otro stage.

Fuentes: `src/orchestrators/pipeline_nomina_executor.py:158-224,268-320`,
`src/orchestrators/pipelines/pipeline_nomina_licencias.yaml:11-63`,
`src/modules/licencias/steps/step2_enriquecer_nomina.py:268-273`.

### 11.3 Detección de fallo por stage

| ID | Regla | Fuente |
|---|---|---|
| NOM-ENR-05 | Un stage falla si: (a) lanza excepción; o (b) devuelve un diccionario con `success = false` (mensaje = `error` o `mensaje` o "El stage reportó success=False"), o sin `success` pero con `error` no vacío. | `src/orchestrators/pipeline_nomina_executor.py:61-86,213-217` |
| NOM-ENR-06 | **Validación de outputs:** tras cada stage exitoso, para cada `outputs[]` se sustituye `${output_dir}` en `path_template` (única variable soportada); si `required` (default `true`) y el archivo no existe → `FileNotFoundError` → el stage cuenta como fallido. Los opcionales solo se registran si existen. | `src/orchestrators/pipeline_nomina_executor.py:222-224,322-345` |
| NOM-ENR-07 | Outputs requeridos: stage 0 `silver/Planilla_<CLIENTE>_Consolidado_Silver.parquet`; stage 1 `silver/licencias_consolidadas.parquet`; stage 2 `gold/actual/Planilla_<CLIENTE>_Consolidado.parquet`; stage 3 `gold/actual/Planilla_<CLIENTE>_Consolidado.parquet` y `gold/actual/Planilla_<CLIENTE>_BI_Gold_Con_Licencias.parquet`. Opcionales (stage 3): los dos `.xlsx`. Nota: la existencia puede satisfacerse con un archivo **de una corrida anterior** (no se compara fecha de modificación). | `src/orchestrators/pipelines/pipeline_nomina_licencias.yaml:18-63` |
| NOM-ENR-08 | **`stop_on_error: true`** (YAML): ante el primer stage fallido el pipeline se detiene y devuelve `success=false`, `error = "Stage '<nombre>' falló: <mensaje>"`, detalles (tipo de excepción, primeras 8 líneas de traza), `completed_stages = índice del fallido`. Con `false` continuaría y reportaría todos los fallidos al final. | `src/orchestrators/pipeline_nomina_executor.py:379-413`, `src/orchestrators/pipelines/pipeline_nomina_licencias.yaml:6-8` |
| NOM-ENR-09 | Archivos ya escritos por stages previos **no** se revierten si un stage posterior falla (inferido: no hay lógica de rollback). | `src/orchestrators/pipeline_nomina_executor.py:347-433` |

### 11.4 Resultado

Éxito → `{success: true, completed_stages: 4, total_stages: 4, duracion_total, stages_results}`
(`src/orchestrators/pipeline_nomina_executor.py:427-433`). Archivos finales en
`<output_dir>`: `silver/Planilla_<CLIENTE>_Consolidado_Silver.parquet`,
`silver/licencias_consolidadas.parquet`, `gold/actual/Planilla_<CLIENTE>_Consolidado.parquet`
(enriquecido), `gold/actual/Planilla_<CLIENTE>_BI_Gold_Con_Licencias.parquet` y, opcionalmente, los
`.xlsx` homónimos.

### 11.5 Diagrama

```
[planillas *.xlsx] --stage0--> silver/Planilla_<CLIENTE>_Consolidado_Silver.parquet --stage2--> gold/actual/Planilla_<CLIENTE>_Consolidado.parquet
[licencias/CONTROL DE LICENCIAS.xlsx] --stage1--> silver/licencias_consolidadas.parquet           |
                                                                    \---------------------stage3 (LEFT JOIN)--> sobrescribe Gold base + copia BI
```

### 11.6 Modo standalone (scripts)

- `step1.main()`: elige carpeta con diálogo, consolida y escribe en `<carpeta>/silver/`
  (`src/modules/nomina/steps/step1_consolidar_planillas.py:462-569`).
- `step2.main()`: el usuario elige el parquet Silver y **cualquier** esquema YAML de una carpeta
  `esquemas/` buscada en el directorio actual y hasta 3 niveles arriba; nunca genera Excel
  (`guardar_resultados` sin `export_excel`) (`src/modules/nomina/steps/step2_exportar.py:591-740`).
- `licencias.step2.main()`: enriquece un Gold elegido; nunca genera la copia BI
  (`src/modules/licencias/steps/step2_enriquecer_nomina.py:179-254`).

## 12. Rework (Plan 4)

Implementado en `config/sql/planilla/` y `config/sql/licencias/` (Silver y Gold) y `crates/core/src/proceso.rs`.
Decisión por regla (TR-D11):

| Regla | Decisión | Detalle |
|---|---|---|
| NOM-BRZ-10 | Preservar | Filas de totales por `filas_resumen` del contrato; se descartan sin aviso |
| NOM-SLV-02 | Corregir | Un archivo con errores detiene todo el proceso (preflight); no se procesa a medias |
| NOM-SLV-04 | Preservar | `dias_subsidio_maternidad` siempre existe; el resto de columnas dinámicas va a `dias_subsidio_otros` |
| NOM-SLV-05 | Descartar | El lector ya entrega tipos; no se pasa todo a texto |
| NOM-SLV-08 | Corregir | `anio` y `mes` enteros |
| NOM-SLV-10 | Corregir | Dos archivos del mismo periodo son error (TR-D23) |
| NOM-GLD-03 | Corregir | Una celda inválida da aviso PRE-20 y queda vacía; ya no deja toda la columna como texto |
| NOM-GLD-05 | Corregir | `nombre_mes` en español |
| NOM-GLD-06…10 | Descartar | No se generan `DIAS SUBSIDIOS TOTAL` ni `SUBSIDIOS TOTAL`: el tablero no los usa (doble conteo) |
| NOM-GLD-15 | Corregir | Persona repetida en un periodo: aviso con las filas (TR-D23) |
| NOM-ENR-01/02 | Corregir | Motivos sin repetir (`DISTINCT`) |
| NOM-ENR-03/04 | Corregir | Un solo Gold de planilla, ya con las licencias |
| Montos | Descartar | Los montos de dinero quedan fuera del contrato y del Gold (TR-D22) |

Paridad: `crates/core/tests/paridad.rs` compara con el legacy (mismas personas por periodo, mismas sumas de días y
horas, mismos motivos sin repetir).
