# Lógica de negocio — Dominio `licencias` (Control de licencias con/sin goce de haber)

> Especificación agnóstica de lenguaje extraída del código Python legado
> (`HR_Data_Integration_Pipeline`). Rutas relativas a la raíz de ese repo.
> El nombre real del cliente se reemplaza por `<CLIENTE>` (también dentro de nombres de archivo).
> "(inferido)" = deducido del comportamiento de librerías/código, no explícito.

---

## 1. Propósito del dominio

Leer el libro de control de licencias que mantiene RR. HH. (una hoja para licencias **con** goce
de haber y otra para licencias **sin** goce de haber), unificarlas en una tabla Silver con un
indicador `TIPO_LICENCIA`, y usarla para enriquecer la nómina Gold (estándar o régimen minero)
con los motivos de licencia por empleado y periodo.

Este módulo **no tiene UI propia**: solo se ejecuta dentro de los pipelines compuestos
"Nómina con Licencias" (stages 1 y 3) y "Régimen Minero con Licencias" (stages 1 y 3). Los scripts
tienen además un `main()` standalone con diálogos Tk.

| Paso | Función usada por los pipelines | Sección |
|---|---|---|
| Bronze → Silver | `step1_consolidar_licencias.procesar_sin_gui(ruta_archivo, carpeta_salida)` | 3 |
| Enriquecimiento (ENR) | `step2_enriquecer_nomina.procesar_sin_gui(ruta_nomina, ruta_licencias, export_excel_gold, nombre_output)` | 4–5 |

---

## 2. Fuentes (Bronze)

| ID | Regla | Fuente |
|---|---|---|
| LIC-BRZ-01 | **Ubicación del archivo:** pipeline nómina → ruta fija `<output_dir>/licencias/CONTROL DE LICENCIAS.xlsx` (nombre exacto). Pipeline régimen minero → archivo elegido libremente por el usuario (cualquier nombre, filtro `*.xlsx *.xls`). No hay regex de nombre ni periodo en el nombre. | `src/orchestrators/pipeline_nomina_executor.py:284-289`, `src/orchestrators/pipeline_regimen_minero_executor.py:79-83`, `src/modules/nomina_regimen_minero/ui/widget.py:133-157`, `assets/validate_source/licencias.yaml:1-42` |
| LIC-BRZ-02 | **Hojas:** `LICENCIA CON GOCE` → `TIPO_LICENCIA = "CON_GOCE"`; `LICENCIA SIN GOCE` → `TIPO_LICENCIA = "SIN_GOCE"` (nombres exactos). Otras hojas se ignoran. | `src/modules/licencias/steps/step1_consolidar_licencias.py:445-451` |
| LIC-BRZ-03 | **Encabezados:** fila **2** (todas las celdas de la fila). Se conservan solo los encabezados no vacíos tras `trim`, recordando su índice de columna original (los huecos no desalinean). Se aplica `trim` pero **no** se colapsan espacios internos ni se cambia mayúsculas. | `src/modules/licencias/steps/step1_consolidar_licencias.py:158-167` |
| LIC-BRZ-04 | **Datos:** desde la fila **3** hasta el final de la hoja (sin detección de fin). Se descarta toda fila cuya **columna A** sea nula o vacía tras `trim` ("filtro de seguridad: DNI"); el código asume que la columna A es el DNI. | `src/modules/licencias/steps/step1_consolidar_licencias.py:181-193` |
| LIC-BRZ-05 | **Columnas de origen requeridas** (preflight): en ambas hojas `PERIODO`, `DNI/CEX`, `APELLIDO PATERNO`, `APELLIDO MATERNO`, `NOMBRES`; más `MOTIVO DE LIC.CON GOCE` (hoja con goce) o `MOTIVO DE LIC.S.G.H` (hoja sin goce). | `assets/validate_source/licencias.yaml:16-42` |
| LIC-BRZ-06 | **Preflight (contrato `licencias`):** el archivo existe; extensión ∈ {`.xlsx`, `.xlsm`, `.xls`}; se abre; existen **ambas** hojas requeridas (`LICENCIA CON GOCE`, `LICENCIA SIN GOCE`); celda A2 no vacía en cada una; columnas requeridas presentes en la fila 2 (comparación trim + colapsar espacios + mayúsculas). Hojas adicionales permitidas (`enforce_allowed_sheets: false`). Al haber `required_columns` explícitas, `schema_path` se ignora. Cualquier error aborta el pipeline antes del primer stage. | `assets/validate_source/licencias.yaml:1-42`, `src/utils/validate_source.py:75-189,254-256`, `src/modules/nomina/ui/worker.py:74-81`, `src/modules/nomina_regimen_minero/ui/worker.py:77-93` |

---

## 3. Reglas Bronze → Silver

Esquema: `assets/esquemas/esquema_licencias.yaml`.

| ID | Regla | Fuente |
|---|---|---|
| LIC-SLV-01 | **Mapeo de columnas (`column_mapping`):** si existe la columna exacta, se renombra: `MOTIVO DE LIC.CON GOCE` → `MOTIVO`; `MOTIVO DE LIC.S.G.H` → `MOTIVO`. Coincidencia exacta y sensible a mayúsculas/espacios. | `assets/esquemas/esquema_licencias.yaml:3-5`, `src/modules/licencias/steps/step1_consolidar_licencias.py:211-216` |
| LIC-SLV-02 | **`DNI/CEX`:** se convierte a texto y se elimina un sufijo final `.0` (regex `\.0$`), para DNI guardados como número decimal. No se rellenan ceros a la izquierda ni se recortan espacios aquí. | `src/modules/licencias/steps/step1_consolidar_licencias.py:219-225` |
| LIC-SLV-03 | **`PERIODO`:** se convierte a texto tal cual; se espera que la celda ya contenga `YYYY-MM` (no se deriva de fechas ni se valida formato). | `src/modules/licencias/steps/step1_consolidar_licencias.py:227-229`, `assets/esquemas/esquema_licencias.yaml:7-10` |
| LIC-SLV-04 | **Filtro `MOTIVO`:** se eliminan filas con `MOTIVO` nulo o vacío tras `trim`. | `src/modules/licencias/steps/step1_consolidar_licencias.py:231-236` |
| LIC-SLV-05 | **`TIPO_LICENCIA`:** columna literal según la hoja de origen (`CON_GOCE` / `SIN_GOCE`). | `src/modules/licencias/steps/step1_consolidar_licencias.py:238-239` |
| LIC-SLV-06 | **Filtros finales:** se eliminan filas con `DNI/CEX` vacío tras `trim` o `PERIODO` vacío tras `trim`; las filas con valor nulo en esas columnas también quedan fuera (inferido: una comparación con nulo no pasa el filtro). | `src/modules/licencias/steps/step1_consolidar_licencias.py:241-248` |
| LIC-SLV-07 | **Tolerancia por hoja:** si una hoja no existe, no tiene encabezados, no tiene filas válidas **o lanza cualquier excepción** (p. ej. falta `DNI/CEX` o `PERIODO`, tipos mixtos), esa hoja se **omite** con un mensaje y se continúa. Solo si **ambas** hojas resultan omitidas → error "No se pudo procesar ninguna hoja válida". | `src/modules/licencias/steps/step1_consolidar_licencias.py:151-154,169-172,201-203,254-256,453-455` |
| LIC-SLV-08 | **Unión:** concatenación diagonal (por nombre de columna) de las hojas válidas. | `src/modules/licencias/steps/step1_consolidar_licencias.py:458-459` |
| LIC-SLV-09 | **Selección:** se conservan solo las columnas del esquema presentes, en este orden: `PERIODO, DNI/CEX, APELLIDO PATERNO, APELLIDO MATERNO, NOMBRES, MOTIVO, TIPO_LICENCIA`. El resto se descarta. | `src/modules/licencias/steps/step1_consolidar_licencias.py:466-471`, `assets/esquemas/esquema_licencias.yaml:6-38` |
| LIC-SLV-10 | **Validación de esquema (bloqueante):** para cada columna del esquema: (a) debe existir; (b) si `nullable: false` (todas lo son) no puede tener nulos; (c) tipo: `string` debe ser texto, `integer` entero, `date` fecha/fecha-hora (no hay columnas de esos dos tipos hoy); (d) `allowed_values` de `TIPO_LICENCIA` ∈ {`CON_GOCE`, `SIN_GOCE`}; `min`/`max` solo para enteros. Cualquier error → "Validación de esquema falló" (aborta el stage y el pipeline). | `src/modules/licencias/steps/step1_consolidar_licencias.py:54-127,473-480` |
| LIC-SLV-11 | **Sin deduplicación** ni recorte de espacios en nombres/motivos. Una misma licencia repetida en el Excel queda repetida. | `src/modules/licencias/steps/step1_consolidar_licencias.py:423-500` (ausencia) |

---

## 4. Reglas Silver → Gold

El dominio no tiene Gold propio. Su único consumo es el enriquecimiento de la nómina Gold
(sección 5).

---

## 5. Reglas de negocio (enriquecimiento — ENR)

| ID | Regla | Fuente |
|---|---|---|
| LIC-ENR-01 | **Entradas:** `nomina` = parquet Gold indicado en `ruta_nomina` (estándar o régimen minero); `licencias` = `silver/licencias_consolidadas.parquet`. La consulta se lee del archivo `assets/queries/query_licencias_agregadas.sql`; si no existe → error. | `src/modules/licencias/steps/step2_enriquecer_nomina.py:291-301` |
| LIC-ENR-02 | **Agregación por tipo:** por cada `(DNI/CEX, PERIODO)` y tipo, `MOTIVO_<TIPO> = STRING_AGG(MOTIVO, ' \| ' ORDER BY MOTIVO)`: motivos ordenados alfabéticamente, separador literal espacio-barra-espacio, duplicados conservados. | `assets/queries/query_licencias_agregadas.sql:18-35` |
| LIC-ENR-03 | **Join:** `nomina LEFT JOIN cg ON DNI/CEX = DNI/CEX AND PERIODO = PERIODO`, ídem `sg`. Igualdad exacta de texto. Se conservan todas las filas y columnas de nómina (`n.*`), y se agregan al final `MOTIVO_CON_GOCE` y `MOTIVO_SIN_GOCE` (nulo = sin licencias de ese tipo). No se multiplican filas (los CTE están agregados por clave). | `assets/queries/query_licencias_agregadas.sql:36-46` |
| LIC-ENR-04 | Licencias cuyo `(DNI/CEX, PERIODO)` no existe en nómina **se pierden** sin aviso (no hay conteo de huérfanas). Solo se informan cuántas filas de nómina recibieron motivo con/sin goce. | `src/modules/licencias/steps/step2_enriquecer_nomina.py:320-331` |
| LIC-ENR-05 | Columnas de licencias que no se usan en el enriquecimiento: `APELLIDO PATERNO`, `APELLIDO MATERNO`, `NOMBRES` (solo se validan en Silver). | `assets/queries/query_licencias_agregadas.sql:18-46` |

---

## 6. Salidas

| ID | Regla | Fuente |
|---|---|---|
| LIC-SLV-12 | **Silver:** `<carpeta_salida>/licencias_consolidadas.parquet` (en pipelines `carpeta_salida = <output_dir>/silver`), compresión `snappy`, nombre fijo, se **sobreescribe**. La carpeta se crea si no existe. Standalone: `<carpeta del Excel>/silver/licencias_consolidadas.parquet`. | `src/modules/licencias/steps/step1_consolidar_licencias.py:259-289,482-486` |
| LIC-ENR-06 | **Gold base sobreescrito:** el resultado enriquecido se escribe (snappy) en la **misma ruta** del parquet de nómina recibido (`<carpeta de ruta_nomina>/<stem>.parquet`), reemplazando el Gold sin licencias. Si `export_excel_gold`, se escribe `<stem>.xlsx` con un volcado plano (sin formato). | `src/modules/licencias/steps/step2_enriquecer_nomina.py:335-348` |
| LIC-ENR-07 | **Copia BI:** además escribe `<carpeta>/<nombre_output>.parquet` (y `.xlsx` si `export_excel_gold`) con el mismo contenido. `nombre_output` por defecto `Planilla_<CLIENTE>_BI_Gold_Con_Licencias`; el pipeline minero pasa `Planillas <CLIENTE> - Regimen Minero con licencias`. | `src/modules/licencias/steps/step2_enriquecer_nomina.py:268-273,356-367`, `src/orchestrators/pipeline_regimen_minero_executor.py:107-112` |
| LIC-ENR-08 | Sin versionado ni timestamp en ninguna salida. | `src/modules/licencias/steps/step2_enriquecer_nomina.py:339-367` |

---

## 7. Validaciones y errores

| Situación | Efecto | Fuente |
|---|---|---|
| Preflight fallido (archivo, hojas, A2, columnas) | aborta pipeline antes de stages | LIC-BRZ-06 |
| Hoja ausente / sin encabezados / sin datos / excepción al leer | se omite esa hoja (mensaje) | LIC-SLV-07 |
| Ambas hojas omitidas | error, aborta stage | `src/modules/licencias/steps/step1_consolidar_licencias.py:453-455` |
| Columna de esquema faltante, nulos, tipo incorrecto, `TIPO_LICENCIA` inválido | error, aborta stage | LIC-SLV-10 |
| Filas con columna A vacía, `MOTIVO` vacío, `DNI/CEX` o `PERIODO` vacíos | se descartan silenciosamente (conteo impreso solo para columna A) | LIC-BRZ-04, LIC-SLV-04, LIC-SLV-06 |
| Query SQL no encontrada; parquet de entrada ausente; columna de join ausente en nómina | error, aborta stage de enriquecimiento | `src/modules/licencias/steps/step2_enriquecer_nomina.py:291-318` |
| Output requerido del stage no existe | stage fallido (validación del executor) | `src/orchestrators/pipeline_nomina_executor.py:322-345` |

---

## 8. Discrepancias

| ID | Discrepancia | Fuente |
|---|---|---|
| LIC-DIF-01 | **Hojas: preflight vs. procesamiento.** El preflight exige **ambas** hojas; el procesamiento tolera que falte una. En la práctica la tolerancia solo aplica a errores de lectura, que además se tragan en silencio (una hoja `SIN GOCE` rota desaparece del resultado sin abortar). | `assets/validate_source/licencias.yaml:8-15`, `src/modules/licencias/steps/step1_consolidar_licencias.py:254-256` |
| LIC-DIF-02 | **Docstrings desactualizados:** step1 promete `/actual/Licencias_Consolidadas.parquet` y `/historico/…_YYYYMMDD_HHMMSS.parquet` (versionado); el código escribe solo `silver/licencias_consolidadas.parquet` sin histórico. step2 menciona `gold/nomina/actual/`; la ruta real es la carpeta del parquet recibido (`gold/actual/`). | `src/modules/licencias/steps/step1_consolidar_licencias.py:11-13`, `src/modules/licencias/steps/step2_enriquecer_nomina.py:7-9,143-145` |
| LIC-DIF-03 | **Normalización de `DNI/CEX` asimétrica:** licencias quita `.0`; nómina (estándar y minera) no. | `src/modules/licencias/steps/step1_consolidar_licencias.py:219-225`, `src/modules/nomina/steps/step1_consolidar_planillas.py:330-339` |
| LIC-DIF-04 | **Normalización de encabezados distinta:** licencias solo aplica `trim`; nómina colapsa espacios internos; preflight además ignora mayúsculas. Un encabezado `MOTIVO DE LIC.CON  GOCE` (doble espacio) pasa preflight pero no se mapea a `MOTIVO`, la hoja queda sin `MOTIVO` y luego falla la validación (columna faltante) o, si la otra hoja sí la tiene, quedan nulos → error de nulabilidad. | `src/modules/licencias/steps/step1_consolidar_licencias.py:165-166,214-216`, `src/utils/validate_source.py:372-373` |
| LIC-DIF-05 | **`APELLIDO MATERNO` no nulable** en licencias (bloqueante) pero nulable en el esquema de nómina. Un empleado sin apellido materno con licencia aborta todo el pipeline. | `assets/esquemas/esquema_licencias.yaml:19-22`, `assets/esquemas/esquema_nominas.yaml:30-33` |
| LIC-DIF-06 | **Funciones duplicadas:** `guardar_resultados` y `enriquecer_nomina_con_licencias` (usados por `main()`) duplican la lógica de `procesar_sin_gui` pero sin la copia BI. `get_resource_path` está redefinido localmente. | `src/modules/licencias/steps/step2_enriquecer_nomina.py:29-41,59-176` |
| LIC-DIF-07 | Las funciones headless (`procesar_sin_gui`) están definidas **después** del bloque `if __name__ == "__main__"` en ambos scripts (sin efecto funcional en import). | `src/modules/licencias/steps/step1_consolidar_licencias.py:412-423`, `src/modules/licencias/steps/step2_enriquecer_nomina.py:257-268` |

---

## 9. Ambigüedades y preguntas abiertas

1. **¿La columna A es realmente el DNI?** El filtro de seguridad usa la columna A, pero el
   contrato lista `PERIODO` como primera columna requerida. Si A es `PERIODO`, el filtro de "DNI
   vacío" en realidad filtra por periodo vacío.
2. **Formato de `PERIODO` en el Excel:** si la celda es una fecha real de Excel (no texto
   `YYYY-MM`), su representación textual será `YYYY-MM-DD HH:MM:SS…` (inferido) y nunca casará con
   la nómina → motivos nulos sin aviso.
3. **Licencias que abarcan varios meses:** el modelo es un registro por (empleado, periodo,
   motivo). No hay fechas de inicio/fin ni días; ¿se espera una fila por mes en el Excel?
4. **Huérfanas:** ¿debería reportarse cuántas licencias no casaron con ninguna fila de nómina?
5. **Motivos duplicados** en la concatenación: ¿deseable `DISTINCT`?
6. **Mismo archivo de licencias para nómina y régimen minero:** ambos pipelines escriben
   `silver/licencias_consolidadas.parquet`; si comparten `output_dir` se pisan (inferido; en la UI
   las carpetas suelen ser distintas).
7. **Tipos mixtos en una columna** (p. ej. `DNI/CEX` con números y textos): la construcción de la
   tabla podría lanzar excepción y la hoja se omitiría en silencio (inferido).

---

## 10. Rework (Plan 4)

| Regla | Decisión | Detalle |
|---|---|---|
| LIC-SLV-05 | Preservar | `tipo_licencia` según la hoja, configurable en `config/parametros.yaml` (`licencias.tipos`) |
| LIC-SLV-07 / LIC-DIF-01 | Corregir | Una hoja que falta es error de preflight (PRE-09); no se salta en silencio |
| LIC-SLV-10 / LIC-DIF-05 | Corregir | `apellido_materno` puede venir vacío; ya no detiene el proceso |
| LIC-ENR-04 | Corregir | Licencias sin persona en la planilla del periodo: aviso con la cantidad |
| §9 Q5 | Corregir | Motivos sin repetir en la planilla Gold |

## 11. Referencias

- `src/modules/licencias/steps/step1_consolidar_licencias.py:54-127` (validación), `:130-256` (lectura de hoja), `:259-295` (exportación standalone), `:423-500` (headless).
- `src/modules/licencias/steps/step2_enriquecer_nomina.py:59-176` (variante standalone), `:268-384` (headless).
- `assets/esquemas/esquema_licencias.yaml:1-38`, `assets/validate_source/licencias.yaml:1-42`.
- `assets/queries/query_licencias_agregadas.sql:18-46`.
- `src/orchestrators/pipeline_nomina_executor.py:284-317`, `src/orchestrators/pipeline_regimen_minero_executor.py:79-112`.
- `src/utils/validate_source.py:75-377`, `src/utils/gold_export.py:7-27`.
- `tests/test_gold_export.py:72-95` (Excel del enriquecido solo cuando se solicita).
