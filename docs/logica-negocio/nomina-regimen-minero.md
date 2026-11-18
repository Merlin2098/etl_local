# Lógica de negocio — Dominio `nomina_regimen_minero` (Planilla del régimen minero)

> Especificación agnóstica de lenguaje extraída del código Python legado
> (`HR_Data_Integration_Pipeline`). Rutas relativas a la raíz de ese repo.
> El nombre real del cliente se reemplaza por `<CLIENTE>` (también dentro de nombres de archivo).
> "(inferido)" = deducido del comportamiento de librerías/código, no explícito.
> Este dominio es una **copia simplificada** del dominio [`nomina`](nomina.md); aquí se describen
> sus reglas completas y, en §8, todas las diferencias.

---

## 1. Propósito del dominio

Consolidar las planillas mensuales del personal bajo **régimen minero** (un Excel por mes) en una
tabla histórica Gold con periodo, datos laborales, días/horas/licencias y montos totales
(`TOTAL INGRESOS`, `TOTAL DESCUENTOS`, `TOTAL APORTACIONES`, `NETO`), sin nombres del trabajador, y
enriquecerla con los motivos de licencias con/sin goce (ver [`licencias.md`](licencias.md)).

Se ejecuta en la app solo como pipeline compuesto "Régimen Minero con Licencias" (4 stages), con un
executor que hereda del de nómina.

---

## 2. Fuentes (Bronze)

| ID | Regla | Fuente |
|---|---|---|
| RM-BRZ-01 | **Selección (UI):** el usuario elige una **carpeta**; se toman `*.xlsx` y `*.xls` de la raíz, excluyendo `~$*` y `Planilla <CLIENTE> Consolidado*`. Por separado elige **un archivo** de licencias (cualquier nombre/ubicación). El botón de proceso se habilita solo si hay planillas **y** archivo de licencias. | `src/modules/nomina_regimen_minero/ui/widget.py:83-123,133-173` |
| RM-BRZ-02 | **Carpeta de trabajo (`output_dir`)** = carpeta padre del primer archivo de planilla. | `src/modules/nomina_regimen_minero/ui/widget.py:191` |
| RM-BRZ-03 | **Periodo desde el nombre de archivo:** regex `(\d{4}-\d{2})`, primera coincidencia en cualquier parte del nombre. Sin coincidencia → archivo omitido con advertencia. | `src/modules/nomina_regimen_minero/steps/step1_consolidar_regimen_minero.py:25-44,153-160` |
| RM-BRZ-04 | **Regex de nombre en preflight** (además de la de extracción): `(?i)^.*(?:19\|20)\d{2}-\d{2}.*\.(xlsx\|xlsm\|xls)$` (búsqueda). Un archivo sin `19xx-MM`/`20xx-MM` en el nombre **aborta** la corrida en el preflight. Nota: el preflight acepta cualquier aparición válida, pero la extracción toma la **primera** `\d{4}-\d{2}`, que podría ser otra. | `assets/validate_source/regimen_minero.yaml:8`, `src/utils/validate_source.py:105-112` |
| RM-BRZ-05 | **Hoja** `Planilla` (exacta); si falta → error para ese archivo. Valores calculados. | `src/modules/nomina_regimen_minero/steps/step1_consolidar_regimen_minero.py:62-70` |
| RM-BRZ-06 | **Encabezados:** fila **6**, columnas 1..199, valor no nulo → texto, `trim`, colapsar espacios internos a uno. Fin: índice de columna > `nº_encabezados + 3` en celda vacía. Huecos de encabezado no dejan columna (desalinean datos posteriores; igual que nómina). | `src/modules/nomina_regimen_minero/steps/step1_consolidar_regimen_minero.py:72-84` |
| RM-BRZ-07 | **Datos:** desde la fila **7** hasta el final. Se descarta una fila si todas sus celdas son nulas, si la columna A es nula o si la columna A es texto de más de 20 caracteres. **No** hay detección de filas de totales/subtotales por texto. Se toman las primeras N celdas (N = nº de encabezados), asignación posicional. | `src/modules/nomina_regimen_minero/steps/step1_consolidar_regimen_minero.py:86-107` |
| RM-BRZ-08 | **Preflight (contrato `regimen_minero`):** por archivo: existe; extensión ∈ {`.xlsx`, `.xlsm`, `.xls`}; nombre cumple RM-BRZ-04; se abre; existe hoja `Planilla`; celda A6 no vacía; columna obligatoria `DNI/CEX` presente en la fila 6 (trim + colapsar espacios + mayúsculas). `required_columns` explícito tiene precedencia sobre `schema_path`/`extract`/`exclude_columns` (que hoy producirían lo mismo: `DNI/CEX`). Cualquier error aborta. | `assets/validate_source/regimen_minero.yaml:1-28`, `src/utils/validate_source.py:75-189,254-256`, `src/modules/nomina_regimen_minero/ui/worker.py:66-75` |
| RM-BRZ-09 | **Licencias:** el archivo elegido debe existir (si no → error "Archivo de licencias no encontrado") y pasar el preflight `licencias`. | `src/modules/nomina_regimen_minero/ui/worker.py:77-93` |

---

## 3. Reglas Bronze → Silver (stage 1: `consolidar_archivos`)

| ID | Regla | Fuente |
|---|---|---|
| RM-SLV-01 | **Entrada:** solo lista de rutas (no acepta carpeta, a diferencia de nómina). | `src/modules/nomina_regimen_minero/steps/step1_consolidar_regimen_minero.py:133-150` |
| RM-SLV-02 | **Tolerancia por archivo:** errores por archivo se registran y se sigue; si ninguno se procesa → error "No se pudo procesar ningún archivo correctamente". | `src/modules/nomina_regimen_minero/steps/step1_consolidar_regimen_minero.py:150-181` |
| RM-SLV-03 | **`PERIODO`** (`YYYY-MM` del nombre) como primera columna de cada archivo. | `src/modules/nomina_regimen_minero/steps/step1_consolidar_regimen_minero.py:120-125` |
| RM-SLV-04 | **Tipos uniformes:** todas las columnas excepto `PERIODO` a texto (no estricto) antes de unir. | `src/modules/nomina_regimen_minero/steps/step1_consolidar_regimen_minero.py:199-208` |
| RM-SLV-05 | **Unión diagonal** por nombre de columna. | `src/modules/nomina_regimen_minero/steps/step1_consolidar_regimen_minero.py:212` |
| RM-SLV-06 | **Filtro:** si existe `DNI/CEX`, se eliminan filas con `DNI/CEX` nulo (las filas de totales suelen caer aquí porque no traen DNI — inferido). Si no existe, advertencia. | `src/modules/nomina_regimen_minero/steps/step1_consolidar_regimen_minero.py:221-231` |
| RM-SLV-07 | **Derivadas:** `AÑO = substring(PERIODO,1,4)` (texto); `MES = entero(substring(PERIODO,6,2))` (nulo si no convierte). | `src/modules/nomina_regimen_minero/steps/step1_consolidar_regimen_minero.py:237-242` |
| RM-SLV-08 | **Orden Silver:** `PERIODO, AÑO, MES`, resto. | `src/modules/nomina_regimen_minero/steps/step1_consolidar_regimen_minero.py:245-248` |
| RM-SLV-09 | Sin deduplicación, sin `trim` de valores, sin garantía de columnas (no hay equivalente a `SUBSIDIO MATERNIDAD`). | `src/modules/nomina_regimen_minero/steps/step1_consolidar_regimen_minero.py:133-264` (ausencia) |

---

## 4. Reglas Silver → Gold (stage 3: `exportar_a_gold`)

Esquema `assets/esquemas/esquema_regimen_minero.yaml` (versión `1.7`).

| ID | Regla | Fuente |
|---|---|---|
| RM-GLD-01 | Si el parquet Silver no existe → `FileNotFoundError` (aborta). Esquema no encontrado → error. | `src/modules/nomina_regimen_minero/steps/step2_exportar_regimen_minero.py:402-412` |
| RM-GLD-02 | **Selección:** columnas del bloque `schema` presentes en Silver, en orden del esquema; faltantes solo se informan. Nombres exactos. | `src/modules/nomina_regimen_minero/steps/step2_exportar_regimen_minero.py:38-56` |
| RM-GLD-03 | **Columnas del esquema (31):** `PERIODO, MES, AÑO, CÓDIGO CORPORATIVO, DNI/CEX, SEXO, F. NACIMIENTO, PUESTO, SEDE, ÁREA, CENTRO DE COSTOS, DESC. CENTRO DE COSTO, FECHA DE INGRESO, DIAS TRABAJADOS, DIAS DESC MEDICO (P), DIAS GOCE HABER(D), DIAS DIST.LIC.S.GH, DIAS LIC. PATE (P), LICENCIA GOCE HABER, # HRS EXTRAS SIMP.25%, # HRS EXTRAS SIMP.35%, HORAS EXTRAS 25%, HORAS EXTRAS 35%, #HRS EXTRAS DOBLES, HORAS EXTRAS 100%, DESCANSO MEDICO, TOTAL INGRESOS, TOTAL DESCUENTOS, TOTAL APORTACIONES, NETO`. **No** incluye apellidos, nombres ni tipo de documento. | `assets/esquemas/esquema_regimen_minero.yaml:8-166` |
| RM-GLD-04 | **Conversión de tipos NO estricta:** `string`→texto, `integer`→entero 64, `float`→decimal 64 — cada valor que no convierte queda **nulo** individualmente (p. ej. `"30.0"` a entero → nulo, inferido). `date` → parseo `%Y-%m-%d %H:%M:%S%.f` no estricto, truncado a fecha. | `src/modules/nomina_regimen_minero/steps/step2_exportar_regimen_minero.py:58-79` |
| RM-GLD-05 | **`trim`** a toda columna de texto tras convertir. | `src/modules/nomina_regimen_minero/steps/step2_exportar_regimen_minero.py:81-85` |
| RM-GLD-06 | **`NOMBRE_MES`** (inglés, `January`…`December`; inválido → nulo) insertado justo después de `MES`. Orden resultante: `PERIODO, MES, NOMBRE_MES, AÑO, …` (coincide con `column_order_final` del YAML por el orden de claves del esquema, aunque ese bloque no se lee). | `src/modules/nomina_regimen_minero/steps/step2_exportar_regimen_minero.py:90-99`, `src/utils/month_name.py:22-48`, `assets/esquemas/esquema_regimen_minero.yaml:218-249` |
| RM-GLD-07 | **Sin columnas dinámicas, sin totalizadores calculados y sin `validar_constraints`**: no se verifican PK, nulos, `allowed_values` ni rangos. | `src/modules/nomina_regimen_minero/steps/step2_exportar_regimen_minero.py:424-426` |

---

## 5. Reglas de negocio

| ID | Regla | Fuente |
|---|---|---|
| RM-GLD-08 | **Clave declarada** `(PERIODO, DNI/CEX)`: no se verifica en ningún punto. | `assets/esquemas/esquema_regimen_minero.yaml:167-170` |
| RM-GLD-09 | **Reglas declarativas no implementadas:** `NETO ≈ TOTAL INGRESOS − TOTAL DESCUENTOS`; `0 ≤ DIAS TRABAJADOS ≤ 31`; `FECHA DE INGRESO` anterior a `PERIODO`; montos totales no negativos; días/horas/licencias no negativos. Los montos se toman tal cual vienen en la planilla (no se recalculan). | `assets/esquemas/esquema_regimen_minero.yaml:172-179` |
| RM-GLD-10 | **Metadatos no usados:** `min_value`, `max_value`, `allowed_values` (`SEXO ∈ {M,F}`), `precision`, `transformations.enrichment`, `column_order_base`, `column_order_final`. | `assets/esquemas/esquema_regimen_minero.yaml:13-249` |
| RM-ENR-01 | **Enriquecimiento con licencias:** idéntico al de nómina (LEFT JOIN por `DNI/CEX` + `PERIODO`, `MOTIVO_CON_GOCE` / `MOTIVO_SIN_GOCE` concatenados con `" \| "` en orden alfabético, columnas al final). Ver `licencias.md` §5. | `assets/queries/query_licencias_agregadas.sql:18-46`, `src/modules/licencias/steps/step2_enriquecer_nomina.py:268-367` |

---

## 6. Salidas

| ID | Regla | Fuente |
|---|---|---|
| RM-SLV-10 | **Silver:** `<output_dir>/silver/Planilla <CLIENTE> Consolidado - Regimen Minero.parquet`, nombre fijo, se sobreescribe (sin reintento con otra compresión). | `src/modules/nomina_regimen_minero/steps/step1_consolidar_regimen_minero.py:254-297` |
| RM-GLD-11 | **"Versionamiento":** solo crea `<carpeta_trabajo>/gold/actual/`; sin histórico ni timestamp. | `src/modules/nomina_regimen_minero/steps/step2_exportar_regimen_minero.py:175-193,429` |
| RM-GLD-12 | **Gold:** `gold/actual/Planilla <CLIENTE> - Regimen Minero.parquet` (sobreescribe). Excel opcional (`export_excel_gold`) `gold/actual/Planilla <CLIENTE> - Regimen Minero.xlsx` con formato: hoja `Planilla`, encabezado negrita blanco tamaño 11 sobre `#366092`, centrado con ajuste, bordes finos (color por defecto), números a la derecha, ancho `min(máx+2, 50)`, congelado `A2`. A diferencia de nómina, un fallo del Excel **aborta** el stage (no hay try/except). | `src/modules/nomina_regimen_minero/steps/step2_exportar_regimen_minero.py:102-172,431-446` |
| RM-ENR-02 | **Gold enriquecido (stage 4):** sobreescribe `gold/actual/Planilla <CLIENTE> - Regimen Minero.parquet` con la versión enriquecida y escribe la copia `gold/actual/Planillas <CLIENTE> - Regimen Minero con licencias.parquet` (nótese "Planillas" en plural). Con `export_excel_gold`, ambos `.xlsx` en volcado plano, reemplazando el Excel formateado. | `src/orchestrators/pipeline_regimen_minero_executor.py:97-112`, `src/modules/licencias/steps/step2_enriquecer_nomina.py:335-367` |

### 6.1 Pipeline "Régimen Minero con Licencias"

Mismo motor que nómina (`PipelineNominaExecutor`): import dinámico, detección de fallo,
validación de outputs, `stop_on_error: true` (ver `nomina.md` §11). La subclase solo cambia:

| ID | Regla | Fuente |
|---|---|---|
| RM-ENR-03 | `validate_structure`: solo comprueba que exista el archivo de licencias elegido (no exige subcarpeta `licencias/`). | `src/orchestrators/pipeline_regimen_minero_executor.py:39-53` |
| RM-ENR-04 | Parámetros por índice (YAML sin `params`): 0 → `archivos`, `carpeta_trabajo=output_dir`; 1 → `ruta_archivo=<licencias elegido>`, `carpeta_salida=output_dir/silver`; 2 → `ruta_parquet_silver=output_dir/silver/Planilla <CLIENTE> Consolidado - Regimen Minero.parquet`, `carpeta_trabajo`, `export_excel_gold`; 3 → `ruta_nomina=output_dir/gold/actual/Planilla <CLIENTE> - Regimen Minero.parquet`, `ruta_licencias=output_dir/silver/licencias_consolidadas.parquet`, `export_excel_gold`, `nombre_output="Planillas <CLIENTE> - Regimen Minero con licencias"`. | `src/orchestrators/pipeline_regimen_minero_executor.py:55-115`, `src/orchestrators/pipelines/pipeline_regimen_minero_licencias.yaml:9-47` |
| RM-ENR-05 | Outputs requeridos: stage 0 Silver minero; stage 1 `silver/licencias_consolidadas.parquet`; stage 2 `gold/actual/Planilla <CLIENTE> - Regimen Minero.parquet`; stage 3 `gold/actual/Planillas <CLIENTE> - Regimen Minero con licencias.parquet` (y `.xlsx` opcional). | `src/orchestrators/pipelines/pipeline_regimen_minero_licencias.yaml:14-47` |

---

## 7. Validaciones y errores

| Situación | Efecto | Fuente |
|---|---|---|
| Preflight (nombre sin `19xx/20xx-MM`, extensión, hoja, A6, `DNI/CEX`) o licencias inválidas | aborta antes de stages | RM-BRZ-04, RM-BRZ-08, RM-BRZ-09 |
| Archivo sin periodo / ilegible en stage 1 | se omite; aborta solo si no queda ninguno | RM-SLV-02 |
| Silver o esquema inexistente | aborta stage 3 | RM-GLD-01 |
| Valores no convertibles | nulo silencioso por celda | RM-GLD-04 |
| Fallo al escribir Excel Gold | aborta stage 3 | RM-GLD-12 |
| Calidad de datos (duplicados, nulos, rangos) | **no se valida** | RM-GLD-07 |

---

## 8. Discrepancias (incluye diferencias con nómina estándar)

| ID | Discrepancia | Fuente |
|---|---|---|
| RM-DIF-01 | **Sin filtro de filas de totales** (`+ INGRESOS DEL MES`, `TOTAL …`, `SUBTOTAL`…): nómina sí lo tiene. En minero solo se eliminan si su `DNI/CEX` es nulo o su primera celda tiene > 20 caracteres. Un total con DNI no nulo llegaría a Gold. | `src/modules/nomina_regimen_minero/steps/step1_consolidar_regimen_minero.py:86-99` vs `src/modules/nomina/steps/step1_consolidar_planillas.py:178-179` |
| RM-DIF-02 | **Casts:** minero no estricto (valor inválido → nulo); nómina estricto por columna (columna inválida → queda texto). Mismo dato de entrada puede producir tipos distintos. | `src/modules/nomina_regimen_minero/steps/step2_exportar_regimen_minero.py:64-70` vs `src/modules/nomina/steps/step2_exportar.py:59-65` |
| RM-DIF-03 | **Sin `SUBSIDIO MATERNIDAD`, sin columnas dinámicas `DIAS SUB…`/`SUB…` ni totales, sin `validar_constraints`** (el esquema minero no define `dynamic_columns`). | `src/modules/nomina_regimen_minero/steps/step2_exportar_regimen_minero.py:376-460`, `assets/esquemas/esquema_regimen_minero.yaml` |
| RM-DIF-04 | **Tipos distintos para la misma columna:** `DIAS GOCE HABER(D)` y `DIAS DIST.LIC.S.GH` son `float` en minero e `integer` en nómina. `DIAS LIC. PATE (P)` es `integer` con `precision: 2` en ambos (incoherente). | `assets/esquemas/esquema_regimen_minero.yaml:78-95`, `assets/esquemas/esquema_nominas.yaml:82-86,138-148` |
| RM-DIF-05 | **Orden `MES`/`AÑO`:** minero `PERIODO, MES, NOMBRE_MES, AÑO`; nómina `PERIODO, AÑO, MES, NOMBRE_MES`. | `assets/esquemas/esquema_regimen_minero.yaml:9-22`, `assets/esquemas/esquema_nominas.yaml:8-21` |
| RM-DIF-06 | **`column_order_base` / `column_order_final` declarados pero no usados**; el orden real sale del orden de claves de `schema`. | `assets/esquemas/esquema_regimen_minero.yaml:187-249` |
| RM-DIF-07 | **Excel:** en minero el error al generar Excel aborta; en nómina solo advierte. El `main()` standalone minero fija `export_excel = False` (nunca genera Excel). Estilos ligeramente distintos (hoja `Planilla` vs `Planilla Gold`, color de bordes). | `src/modules/nomina_regimen_minero/steps/step2_exportar_regimen_minero.py:338-342,437-442` vs `src/modules/nomina/steps/step2_exportar.py:465-483` |
| RM-DIF-08 | **Mensaje del worker** llama "Gold base" a `Planilla <CLIENTE> - Regimen Minero.parquet`, pero el stage 4 lo sobreescribe con la versión enriquecida (el YAML minero no lista ese archivo como output del stage 4; el de nómina sí lo lista). | `src/modules/nomina_regimen_minero/ui/worker.py:153-158`, `src/orchestrators/pipelines/pipeline_regimen_minero_licencias.yaml:37-47` |
| RM-DIF-09 | **Preflight con `filename_regex`** solo en minero; nómina no lo tiene. | `assets/validate_source/regimen_minero.yaml:8`, `assets/validate_source/nomina.yaml` |
| RM-DIF-10 | **Silver:** nómina reintenta la escritura con `snappy`; minero no. `main()` standalone minero llama `guardar_resultados` dos veces (dentro de `consolidar_archivos` y después). | `src/modules/nomina_regimen_minero/steps/step1_consolidar_regimen_minero.py:254-262,361` |
| RM-DIF-11 | **Nombres de archivo inconsistentes:** Silver con espacios (`Planilla <CLIENTE> Consolidado - Regimen Minero`), Gold `Planilla <CLIENTE> - Regimen Minero`, copia BI `Planillas …` (plural); nómina usa guiones bajos. | ver RM-SLV-10, RM-GLD-12, RM-ENR-02 |
| RM-DIF-12 | **Origen de licencias:** archivo libre elegido por el usuario (minero) vs ruta fija `licencias/CONTROL DE LICENCIAS.xlsx` (nómina). | `src/orchestrators/pipeline_regimen_minero_executor.py:1-7,79-83` |

---

## 9. Ambigüedades y preguntas abiertas

1. ¿La ausencia del filtro de totales y de `validar_constraints` en minero es intencional o una
   copia desactualizada? (Parece que la nómina estándar recibió esas mejoras después y no se
   propagaron — inferido.)
2. ¿Por qué el Gold minero excluye nombres y tipo de documento? ¿Requisito de privacidad o
   simplemente columnas que no existen en la planilla minera?
3. Con casts no estrictos, un día `"30.0"` en `DIAS TRABAJADOS` (entero) quedaría nulo (inferido):
   ¿se acepta esa pérdida silenciosa?
4. Si el nombre contiene dos patrones `NNNN-NN` (p. ej. un código antes de la fecha), preflight
   pasa pero la extracción toma el primero: ¿definir una regex única y anclada?
5. ¿Se debe implementar `NETO ≈ TOTAL INGRESOS − TOTAL DESCUENTOS` (con qué tolerancia)?

---

## Decisiones del rework (Plan 8)

Reporte «Planilla régimen especial» (`planilla_regimen_especial`), hecho con el molde de reportes
([config/README.md](../../config/README.md)). No busca paridad con el legacy.

| Pregunta / regla | Decisión |
|---|---|
| §9.1 filtro de totales | Se descartan las filas de totales (`filas_resumen` del contrato): corrige RM-DIF-01. |
| §9.2 nombres | El Gold no lleva nombres, como el legacy (RM-GLD-03). |
| §9.3 casts no estrictos | Los tipos los valida el contrato con mensajes por celda (Plan 3), como en la planilla estándar. |
| §9.4 periodo en el nombre | Un solo patrón anclado: `Planilla Regimen Especial AAAA-MM` (layout demo; cada empresa lo ajusta). |
| §9.5 `NETO ≈ INGRESOS − DESCUENTOS` | No aplica: los montos no se leen (TR-D22). |
| RM-ENR-01 enriquecimiento con licencias | No se hace. |
| TR-D9 perfiles de planilla | Reporte aparte, con su Gold y su Excel; **no** entra en los indicadores del tablero. |
| Persona repetida en el periodo | Aviso `PRE-RE-01` (como TR-D23). |

## 10. Referencias

- `src/modules/nomina_regimen_minero/steps/step1_consolidar_regimen_minero.py:25-297`.
- `src/modules/nomina_regimen_minero/steps/step2_exportar_regimen_minero.py:31-193,376-460`.
- `src/modules/nomina_regimen_minero/ui/widget.py:83-207`, `src/modules/nomina_regimen_minero/ui/worker.py:42-169`, `src/modules/nomina_regimen_minero/ui/config.py`.
- `src/orchestrators/pipeline_regimen_minero_executor.py:15-115`, `src/orchestrators/pipeline_nomina_executor.py:158-433`.
- `src/orchestrators/pipelines/pipeline_regimen_minero_licencias.yaml:1-47`.
- `assets/validate_source/regimen_minero.yaml:1-28`, `src/utils/validate_source.py:75-377`.
- `assets/esquemas/esquema_regimen_minero.yaml:1-249`.
- `src/utils/month_name.py:6-48`, `src/utils/gold_export.py:7-27`.
