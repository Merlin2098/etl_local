# 01 — Análisis del proyecto legacy `HR_Data_Integration_Pipeline`

> Estado: v1 · Fecha: 2026-11-14 · Fuente analizada: [`../HR_Data_Integration_Pipeline/`](../HR_Data_Integration_Pipeline/)
> (143 commits, 2025‑12‑30 → 2026‑05‑18)

## 0. Resumen ejecutivo

- **Qué es:** app de escritorio Windows (PySide6) con 6 módulos ETL + 1 módulo auxiliar
  (licencias) que transforman Excel de RR. HH. en Parquet Bronze → Silver → Gold para Power BI.
- **Lo que vale la pena conservar:** el *modelo* — contratos de validación preflight
  declarativos, capas de datos, reglas Gold en SQL DuckDB, versionado `actual/` + `historico/`,
  ejecución independiente por dominio. Los 4 archivos SQL son portables tal cual.
- **Lo que no vale la pena portar línea a línea:** la capa Python. ~16 k líneas con mucha
  duplicación (lógica de steps reimplementada en workers, 4 patrones de orquestación distintos,
  7 formatos de esquema distintos, scripts *standalone* con Tkinter dentro de una app Qt).
- **Bloqueante para open source:** el historial git contiene **archivos Excel reales de
  planilla y base de datos de personal**, y el código tiene **34+ referencias al nombre del
  cliente**. El repo open source debe nacer con historial nuevo (ver [§8](#8-riesgos-para-publicar-como-open-source)).
- **Recomendación preliminar de stack:** Rust + Tauri si se prioriza rendimiento y la
  continuidad con Polars; Go + Wails si se prioriza velocidad de entrega. En ambos casos el
  centro de gravedad es **DuckDB** (ver [§9](#9-implicancias-para-el-rework)).

## 1. Métricas del código

| Área | Líneas (Python) | Comentario |
|---|---:|---|
| `src/modules/*/steps` | 8 180 | Lógica ETL por dominio (19 archivos) |
| `src/modules/*/ui` | 4 011 | Config + widget + worker por módulo |
| `src/utils` | 2 764 | Registro de ETLs, base worker/widget, validación, paths, logger Qt |
| `src/orchestrators` | 1 012 | 3 ejecutores de pipelines YAML |
| **Total `src`** | **16 210** | |
| `tests` | 779 | 27 tests (fechas de documento, nombre de mes, export Gold, subsidios) |
| `assets` | ~1 700 | 9 esquemas YAML, 7 contratos de fuente, 4 SQL, tema UI |
| `print(` en `src` | 1 256 | Logging mayoritariamente por `print` |

## 2. Arquitectura actual

```text
etl_manager.py ──► src/app_main.py (QMainWindow, pestañas lazy)
                        │
                        ▼
             utils/ui/etl_registry.py  ── descubre src/modules/*/ui/config.py
                        │
          ┌─────────────┼───────────────────────────────┐
          ▼             ▼                               ▼
     widget.py    worker.py (QThread)            base_etl_widget / base_worker
                        │
     ┌──────────────────┼────────────────────┬─────────────────────┐
     ▼                  ▼                    ▼                     ▼
 validate_source   steps/stepN_*.py    orchestrators/*_executor   lazy_loader
 (contrato YAML)   (Polars/openpyxl)   + pipelines/*.yaml         (import diferido)
                        │
                        ▼
              DuckDB + assets/queries/*.sql
                        │
                        ▼
   <output>/silver/*.parquet · <output>/gold/[actual/]*.parquet · gold/historico/*
```

> Nota: la ubicación del Gold "actual" no es uniforme. Algunos módulos escriben en `gold/`
> y otros en `gold/actual/`. El detalle está en [logica-negocio/reglas-transversales.md](logica-negocio/reglas-transversales.md) (TR-18).

### Componentes clave

| Componente | Archivo | Rol |
|---|---|---|
| Shell de escritorio | [src/app_main.py](../HR_Data_Integration_Pipeline/src/app_main.py) | Ventana con una pestaña por ETL, carga diferida, tema, splash |
| Registro de ETLs | [src/utils/ui/etl_registry.py](../HR_Data_Integration_Pipeline/src/utils/ui/etl_registry.py) | Descubrimiento dinámico de módulos por convención de carpetas |
| Worker base | [src/utils/ui/workers/base_worker.py](../HR_Data_Integration_Pipeline/src/utils/ui/workers/base_worker.py) | `QThread` con fases, progreso, resumen de éxito/error |
| Validación preflight | [src/utils/validate_source.py](../HR_Data_Integration_Pipeline/src/utils/validate_source.py) | Lee contrato, verifica extensión, hojas, cabecera y columnas requeridas; *fail‑fast* |
| Orquestador YAML | [src/orchestrators/pipeline_nomina_executor.py](../HR_Data_Integration_Pipeline/src/orchestrators/pipeline_nomina_executor.py) | Carga stages (`module` + `function`), los ejecuta por import dinámico y valida outputs |
| Exportación Gold | [src/utils/gold_export.py](../HR_Data_Integration_Pipeline/src/utils/gold_export.py) | Excel opcional junto al Parquet |
| Fecha de documento | [src/utils/bd_document_date.py](../HR_Data_Integration_Pipeline/src/utils/bd_document_date.py) | Deriva `FECHA_DOCUMENTO` del nombre del archivo Bronze |

## 3. Inventario de dominios (módulos)

| Módulo | Fuente Bronze | Silver | Gold | Lógica destacada |
|---|---|---|---|---|
| `bd` (base de datos de personal) | 1 Excel, cabecera fila 10 | `bd_silver.parquet` (35 cols) | Empleados (14 cols) + Practicantes + Centros de costo + **flags** | Split por modalidad; flags de edad 65/70 y vencimiento de contrato (`queries_flags_gold.sql`) |
| `nomina` | N Excel mensuales, hoja `Planilla`, cabecera fila 6 | Consolidado | Planilla Gold (35 cols base + **columnas dinámicas**) | Detecta columnas `^DIAS SUB`, `^SUB` y genera totales; filtra filas de totales/subtotales; extrae periodo del nombre |
| `nomina_regimen_minero` | N Excel mensuales | Consolidado | Planilla minera Gold | Reglas: NETO ≈ ingresos − descuentos, días trabajados 0–31 |
| `licencias` (sin UI propia) | 1 Excel, hojas *con goce* / *sin goce* | Consolidado | Enriquece nómina Gold | `query_licencias_agregadas.sql`: `STRING_AGG` de motivos por DNI+periodo |
| `pdt` (relación de ingresos) | 1 Excel, hojas EMPLEADOS (desde col. B) / PRACTICANTES | 2 Parquet | Empleados Gold + Practicantes Gold | Limpieza (`PROYECTO` vacío → "Staff", `#N/D` → null), regla de universidad de procedencia |
| `control_practicantes` | 1 Excel, cabecera fila 4 con celdas combinadas | Silver | Gold con tiempo de servicio y flags | `query_control_practicantes_gold.sql`: por cumplir 1 / 2 años |
| `examen_retiro` | 1 Excel, hoja DATA, cabecera fila 3 | Silver | Gold + join con centros de costo | `query_cc_join.sql` contra CC actual y CC antiguo; excluye `STUDENT INTERNAL` |

### Pipelines compuestos (YAML)

| Pipeline | Stages |
|---|---|
| `pipeline_nomina_licencias.yaml` | Nómina B→S · Licencias B→S · Nómina S→G · Enriquecimiento con licencias |
| `pipeline_regimen_minero_licencias.yaml` | Equivalente para régimen minero |
| `pipeline_control_practicantes.yaml` | Silver → Gold con flags |

## 4. Modelo de datos (Gold) — insumo para el dashboard

| Dataset Gold | Clave | Campos útiles para BI |
|---|---|---|
| Empleados (BD) | `NUMERO DE DOC` | Gerencia, sexo, sede, white/blue collar, modalidad, fecha ingreso/término/nacimiento, servicio, régimen, 6 flags booleanos, tiempo de servicio |
| Centros de costo | código CC | Catálogo deduplicado |
| Nómina | `PERIODO` + `DNI/CEX` | Área, centro de costo, ingreso/cese/motivo, horas extra (25/35/50/100 %), días de subsidio, descanso médico, licencias, motivos con/sin goce |
| Régimen minero | `PERIODO` + `DNI/CEX` | Ídem + ingresos, descuentos, neto, días trabajados |
| Relación de ingresos | `PERIODO` + DNI | Ubigeo (depto/prov/distrito), cargo, área, lugar de trabajo, WC/BC, tiempo de contrato |
| Practicantes | DNI | Condición, sede, universidad, jefe, gerencia, días/años/meses de servicio, flags por cumplir |
| Exámenes de retiro | `DNI` + `FECHA DE CESE` | Causa de salida, año/mes, centro de costo enriquecido |

Todos comparten DNI/documento y periodo → permite un **modelo estrella** sencillo
(dimensiones: persona, periodo, centro de costo, área/gerencia, sede).

## 5. Fortalezas (conservar)

1. **Contratos de fuente declarativos** (`assets/validate_source/*.yaml`): cabecera
   por fila/columna, hojas requeridas, columnas requeridas extraídas del esquema. Es la pieza
   más valiosa y la más genérica.
2. **Fail‑fast documentado** (ADR‑0004) con guía de usuario en español.
3. **Reglas Gold en SQL DuckDB**: legibles, testeables y portables a Go/Rust sin cambios.
4. **Versionado dual** `actual/` (rutas estables para el consumidor BI) + `historico/` (auditoría).
5. **Módulos independientes por dominio** (ADR‑0002), alineados con la cadencia real de
   llegada de archivos.
6. **Columnas dinámicas** en nómina (patrones regex + total calculado): resuelve el
   problema real de conceptos que aparecen mes a mes.
7. **ADRs** existentes que explican el *porqué*; sirven como base para los ADRs del rework.

## 6. Deuda técnica y problemas detectados

### 6.1 Arquitectura

| # | Problema | Evidencia | Impacto en el rework |
|---|---|---|---|
| A1 | **Lógica duplicada entre steps y workers.** El worker de `bd` reimplementa los 4 steps (extracción Excel, Silver, Gold, flags) en vez de llamarlos. | [bd/ui/worker.py](../HR_Data_Integration_Pipeline/src/modules/bd/ui/worker.py) (733 líneas) vs `bd/steps/*` | Fuente de verdad ambigua: hay que tomar los steps **y** el worker como referencia al portar |
| A2 | **4 patrones de orquestación distintos**: inline en worker (`bd`), executor YAML (`nomina`, `control_practicantes`), executor ad‑hoc (`regimen_minero`), lazy loader (`pdt`, `examen_retiro`). | `src/orchestrators/`, `src/utils/lazy_loader.py` | Unificar en un único motor de pipelines declarativo |
| A3 | `examen_retiro` **no hereda de `BaseETLWorker`**; replica `run`, resúmenes y manejo de error. | [examen_retiro/ui/worker.py](../HR_Data_Integration_Pipeline/src/modules/examen_retiro/ui/worker.py) | — |
| A4 | Los steps tienen `main()` *standalone* con **Tkinter** (`filedialog`) dentro de una app Qt (18 archivos). | `from tkinter import Tk, filedialog` | Separar núcleo (librería) de UI desde el diseño |
| A5 | Los executors YAML resuelven `module`+`function` por import dinámico y pasan parámetros “dinámicamente desde el executor” (no declarados en YAML). | `pipeline_nomina_licencias.yaml` (`params:` vacíos) | El YAML no es autosuficiente; en el rework los stages deben declarar inputs/outputs explícitos |
| A6 | `get_resource_path` está definido **5 veces**. | `grep "def get_resource_path"` | — |

### 6.2 Configuración y esquemas

| # | Problema | Evidencia |
|---|---|---|
| C1 | **7 formatos distintos de esquema YAML** (`columns` lista, `schema` dict, `hojas`, `column_types`, `columnas_requeridas`/`tipos_datos`, …). | `assets/esquemas/*.yaml` |
| C2 | **`reglas_flags.yaml` no se usa**: las mismas reglas (65/70 años, 4.5/2.5 años de contrato) están **hardcodeadas** en `queries_flags_gold.sql`. | Ningún `src/` referencia `reglas_flags` |
| C3 | `business_rules` y `constraints` de los esquemas son **texto libre**, no se ejecutan. | `esquema_nominas.yaml`, `esquema_regimen_minero.yaml` |
| C4 | El README habla de “JSON contracts/JSON schemas”, pero los archivos son **YAML** (hay un shim `structured_config.py` que acepta ambos). | README vs `assets/` |
| C5 | Nombres de archivos de salida incluyen el nombre del cliente (`Planilla_<Cliente>_Consolidado.parquet`). | pipelines YAML, steps de nómina |

### 6.3 Dependencias y empaquetado

| # | Problema |
|---|---|
| D1 | `pyproject.toml` se llama `aws-terraform-template` (heredado de una plantilla) y declara `requires-python >=3.11`, mientras el README dice 3.13+. |
| D2 | Dependencias declaradas **sin uso directo** en `src`: pandas, numpy, sqlalchemy, psycopg2, jsonschema, great‑expectations, python‑dateutil, boto3, awswrangler. (xlsxwriter, fastexcel y pyarrow sí se usan de forma transitiva vía Polars). El README además menciona Pydantic, que no está ni declarado. |
| D3 | Build con PyInstaller `--onedir` + descubrimiento de *hidden imports*: distribución pesada, solo Windows. |

### 6.4 Calidad

| # | Problema |
|---|---|
| Q1 | 27 tests, centrados en utilidades (fechas, nombre de mes, export). **Sin tests de extremo a extremo** de ningún pipeline ni de los SQL. |
| Q2 | 1 256 `print(` en `src`; el logger Qt convive con prints. |
| Q3 | Mezcla de idiomas en identificadores y columnas (es/en), con tildes y espacios en nombres de columna (`"FECH_INGR."`, `"# HRS EXTRAS SIMP.25%"`). |
| Q4 | Fechas de referencia dependientes de `CURRENT_DATE` en SQL → resultados no reproducibles en tests. |

## 7. Reglas de negocio a preservar (catálogo inicial)

| Regla | Dónde vive hoy | Parametrizable en el rework |
|---|---|---|
| Alerta de edad 65 / 70 años (este año / próximo año) | `queries_flags_gold.sql` | Sí — rule pack “jubilación” |
| Alerta de contrato por obra ≥ 4.5 años (máx. 5) | `queries_flags_gold.sql` | Sí — rule pack “normativa laboral PE” |
| Alerta de contrato por incremento ≥ 2.5 años (máx. 3) | `queries_flags_gold.sql` | Sí |
| Exclusiones: plazo indeterminado, extranjero, término de convenio | `reglas_flags.yaml` (no usado) | Sí |
| Tiempo de servicio = días / 365.25 | SQL flags y practicantes | Sí |
| Practicante profesional por cumplir 1 año (335–364 días) / 2 años | `query_control_practicantes_gold.sql` | Sí |
| Licencias agregadas por DNI+periodo (`STRING_AGG`) | `query_licencias_agregadas.sql` | No (lógica estructural) |
| Totales dinámicos de subsidios (`^DIAS SUB`, `^SUB`) | `esquema_nominas.yaml` + step2 nómina | Sí (patrones en esquema) |
| Exclusión de filas de totales/subtotales en planilla | step1 nómina | Sí |
| `PROYECTO` vacío/0 → "Staff"; `#N/D`/`Error` → null | step1 pdt | Sí (reglas de limpieza declarativas) |
| Excluir cargo `STUDENT INTERNAL` en exámenes de retiro | esquema examen_retiro | Sí |
| `FECHA_DOCUMENTO` derivada del nombre del archivo | `bd_document_date.py` | Sí (regex en contrato) |

## 8. Riesgos para publicar como open source

| Severidad | Riesgo | Acción |
|---|---|---|
| 🔴 Crítico | **El historial git del legacy contiene Excel reales**: planillas mensuales, régimen minero, base de datos de personal, control de practicantes y licencias (`data/…`). Este repositorio no enlaza ese remoto. | El repo nuevo **no debe** heredar ese historial (empezar desde cero, sin `git subtree`/fork). Verificar si el repo de GitHub es público; si lo es, hacerlo privado y purgar historial (`git filter-repo`) y avisar a quien corresponda. |
| 🟠 Alto | **Nombre del cliente** en 22 archivos (34+ ocurrencias): código, nombres de salida, tema, splash, tests, guía de usuario. | No copiar assets/código sin pasar por una lista de *scrubbing*; usar nombres genéricos (`payroll_consolidated.parquet`). |
| 🟠 Alto | Reglas, columnas y layouts de Excel reflejan los formatos internos del cliente. | Rediseñar contratos y datos de demo como **genéricos**; documentar que los layouts son configurables. |
| 🟡 Medio | Un test usa un número de documento de 8 dígitos que no es claramente ficticio (en `test_nomina_subsidio_maternidad.py`; aquí no se cita). | Reemplazar por valores obviamente sintéticos. |
| 🟡 Medio | `LICENSE` con el nombre del autor mal escrito. | Corregir en el repo nuevo. |
| 🟢 Bajo | Pertinencia de publicar lógica derivada de un trabajo para un tercero. | Revisar acuerdos de confidencialidad/propiedad intelectual antes de publicar. |

## 9. Implicancias para el rework

### 9.1 Qué se porta y cómo

| Legacy | Rework |
|---|---|
| `assets/queries/*.sql` | **Se reutilizan** casi tal cual sobre DuckDB embebido; parametrizar `CURRENT_DATE` y umbrales |
| `assets/validate_source/*.yaml` | Se migran a un **formato único de contrato** (fuente + esquema en uno) con validación de esquema propia |
| `assets/esquemas/*.yaml` (7 formatos) | Se unifican; `business_rules` pasan a ser **checks ejecutables** (SQL que devuelve filas inválidas) |
| `reglas_flags.yaml` + SQL hardcodeado | Rule packs que inyectan parámetros al SQL |
| Steps Python (Polars/openpyxl) | Motor genérico: *read Excel → normalize → write Parquet → run SQL* configurado por contrato |
| 4 orquestadores | 1 motor de pipelines declarativo con inputs/outputs explícitos |
| PySide6 + workers | Frontend web (Wails/Tauri) + eventos de progreso del backend |
| Power BI | Dashboard web en `localhost` sobre los Parquet Gold |
| PyInstaller | Binario nativo único |

Observación: buena parte de la lógica Python (lectura con offset de cabecera, limpieza,
tipado, columnas derivadas) puede expresarse en SQL de DuckDB; incluso la lectura de Excel
podría delegarse a la extensión `excel` de DuckDB (`read_xlsx` con `range`), lo que reduciría
el código nativo a orquestación + UI. Conviene validarlo con los layouts más difíciles
(cabeceras combinadas de practicantes, hoja EMPLEADOS que empieza en la columna B).

### 9.2 Recomendación preliminar de stack

- **Rust + Tauri** si el objetivo incluye rendimiento y aprendizaje: `calamine` para Excel,
  Polars nativo (continuidad conceptual con el legacy) y DuckDB; binarios pequeños.
- **Go + Wails** si se prioriza tener una versión funcional pronto: `excelize` + `duckdb-go`;
  código más simple y rápido de escribir.

Dado que la lógica de negocio quedará mayormente en **SQL + configuración**, el lenguaje
nativo pesa menos de lo que parece; la decisión puede tomarse por preferencia y por el
ecosistema del frontend. Se propone registrarla en `docs/adr/0001-stack.md`.

### 9.3 Próximos pasos sugeridos

1. Decidir stack (ADR‑0001 del rework).
2. Diseñar el **formato único de contrato/esquema** y migrar los 7 dominios a él.
3. Construir el **generador de datos sintéticos** por contrato (necesario para tests y demo,
   ya que no se puede usar data real).
4. Crear *golden tests* de paridad: correr el legacy sobre los datos sintéticos y guardar
   los Gold esperados.
5. Implementar motor ETL mínimo con un dominio (`bd`, el más completo: Silver, Gold, CC, flags).
6. Prototipo del dashboard sobre el Gold de `bd` (headcount + alertas).
7. Extender al resto de dominios y a los pipelines compuestos.
