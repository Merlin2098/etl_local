# Reglas transversales y decisiones para el rework

Síntesis de lo que se repite entre dominios, de las dependencias entre ellos y de las
inconsistencias globales detectadas en los documentos de dominio. Los IDs `TR-xx` son propios
de este documento; las referencias a otros IDs apuntan a los archivos de cada dominio.

## 1. Dependencias entre dominios (linaje)

```text
                 ┌──────────────► bd: empleados_gold ──► flags (edad / contrato)
 BD (Excel) ─► bd_silver ─┤
                 ├──────────────► bd: practicantes_gold
                 └──► centros_costo/actual/CC_ACTUAL.parquet ──┐
                                                               ▼
 Exámenes de retiro (Excel) ─► silver ─► gold ──────► JOIN CC (CC_ACTUAL + CC_OLD*) ─► gold enriquecido

 Planilla (N Excel) ─► silver ─► gold ──┐
                                        ├─► JOIN licencias ─► gold (sobrescrito) + gold BI
 Licencias (Excel) ─► silver ───────────┘
 (ídem para régimen minero)

 Relación de ingresos (Excel) ─► silver EMPLEADOS / PRACTICANTES ─► gold por hoja
 Control de practicantes (Excel) ─► silver ─► gold con flags
```

\* `CC_OLD` no lo produce ningún módulo; el usuario lo elige a mano ([examen-retiro.md](examen-retiro.md)).

| ID | Dependencia | Riesgo actual |
|---|---|---|
| TR-01 | `examen_retiro` consume `CC_ACTUAL.parquet` generado por `bd` | Sin verificación de frescura: usa lo que haya en disco |
| TR-02 | La enriquecida de nómina consume la nómina Gold y las licencias Silver por **rutas fijas** | Un archivo de una corrida anterior pasa la validación de outputs, que solo comprueba existencia |
| TR-03 | El archivo de licencias se busca en una ruta fija: `<output>/licencias/CONTROL DE LICENCIAS.xlsx` | Acopla la carpeta de salida con una entrada |
| TR-04 | El Step 3 de `bd` (flags) lee el Gold de empleados del disco | Si la corrida no generó empleados, calcula sobre el archivo viejo ([bd.md](bd.md)) |

## 2. Patrones comunes (candidatos a motor genérico)

| ID | Patrón | Dominios | Variantes a unificar |
|---|---|---|---|
| TR-10 | **Lectura de Excel con cabecera desplazada** (fila N, columna inicial) | Todos | El fin de datos se detecta de 4 formas: primera fila con la clave vacía, primera fila totalmente vacía, hasta el final saltando vacías, o primera celda vacía en la columna A |
| TR-11 | **Lectura de cabeceras** | Todos | Algunos se detienen en la primera celda vacía, otros saltan huecos; unos hacen `trim` y otros no |
| TR-12 | **Normalización de valores**: `"None"`, `"nan"`, `"NaT"`, `#N/D`, `Error` y vacío → null | bd, pdt, control_practicantes | Cada módulo tiene su propia lista |
| TR-13 | **Parseo de fechas**: serial de Excel, texto `DD/MM/YYYY` o `%Y-%m-%d %H:%M:%S` | Todos | Formatos aceptados distintos; si falla, queda null en silencio |
| TR-14 | **Periodo** `YYYY-MM` desde nombre de archivo o desde columnas AÑO/MES | nómina, minero, licencias, pdt | Regex y padding distintos; en pdt, AÑO/MES decimales generan `2025.0-3.0` |
| TR-15 | **`FECHA_DOCUMENTO`** derivada del nombre del archivo | bd, control_practicantes | Se calcula, pero ninguna regla de negocio la usa como "fecha de corte" |
| TR-16 | **`NOMBRE_MES`** en inglés desde MES | nómina, minero, pdt, examen_retiro | Mes inválido → `""` |
| TR-17 | **Selección y tipado según esquema** | Todos los Gold | Cast estricto por columna (nómina) o por valor (minero); falta de columna = aborta |
| TR-18 | **Versionado de salidas** | Todos | Unos escriben en `gold/` y otros en `gold/actual/`; hay dos formatos de timestamp (`%Y%m%d_%H%M%S` y `%d.%m.%Y_%H.%M.%S`) |
| TR-19 | **Excel opcional** junto al Parquet Gold | Todos | La enriquecida de nómina reemplaza el Excel con formato por un volcado plano |
| TR-20 | **Reglas Gold en SQL DuckDB** con tablas registradas por nombre | bd, control_practicantes, examen_retiro, licencias | Fechas relativas a `CURRENT_DATE` o a otra columna (`F. RENOVACION`) |

## 3. Reglas de negocio de dominio (catálogo consolidado)

| ID | Regla | Origen |
|---|---|---|
| TR-30 | Alerta de jubilación: cumple 65 o 70 años **este año o el próximo**, calculado solo por año de nacimiento | [bd.md](bd.md) (BD-FLG-*) |
| TR-31 | Alerta de contrato por obra o servicio específico: ≥ 4.5 años de servicio (máx. legal 5) | [bd.md](bd.md) |
| TR-32 | Alerta de contrato por incremento de actividad: ≥ 2.5 años (máx. legal 3) | [bd.md](bd.md) |
| TR-33 | Tiempo de servicio = días / 365.25, en años decimales y como texto "X años, Y meses" | bd, control_practicantes |
| TR-34 | Practicante: por cumplir 1 año (335–364 días), por cumplir 2 años (640–729), cumplió 1 año (≥ 365) | [control-practicantes.md](control-practicantes.md) |
| TR-35 | Empleado vs practicante según `Modalidad de Contrato` | [bd.md](bd.md) |
| TR-36 | Exclusión de filas de totales/subtotales en planilla | [nomina.md](nomina.md) |
| TR-37 | Columnas dinámicas de subsidios (`^DIAS SUB`, `^SUB`) con columna total | [nomina.md](nomina.md) |
| TR-38 | Licencias con/sin goce agregadas por DNI + periodo (motivos unidos con `\|`) | [licencias.md](licencias.md) |
| TR-39 | Imputaciones: `PROYECTO` vacío → "Staff"; `SERVICIO`/`REGIMEN` vacío → "Personal_Interno" (solo en el script standalone); universidad nula → "POR DEFINIR" | [pdt-relacion-ingresos.md](pdt-relacion-ingresos.md) |
| TR-40 | Exámenes de retiro: excluir cargo `STUDENT INTERNAL`; centro de costo desde CC_ACTUAL y, si falta, desde CC_OLD | [examen-retiro.md](examen-retiro.md) |

## 4. Inconsistencias globales

| ID | Problema | Dónde se ve |
|---|---|---|
| TR-50 | **Preflight y procesamiento comparan cabeceras distinto.** Preflight ignora mayúsculas y espacios repetidos (no tildes); los steps exigen el nombre exacto. Un archivo puede pasar preflight y fallar o perder columnas después. | Todos |
| TR-51 | **Éxito reportado ante fallos parciales.** examen_retiro (falla de Step 2), bd (falla de Step 3 = advertencia) y pdt (falla de Step 2/3) terminan con `success=true`. | bd, pdt, examen_retiro |
| TR-52 | **Sin atomicidad ni rollback.** Si falla un paso intermedio, quedan mezclados archivos nuevos y viejos. | Todos |
| TR-53 | **Constraints declarativos que no se ejecutan.** Claves primarias, `nullable`, rangos y `business_rules` de los esquemas son decorativos. | Todos los esquemas |
| TR-54 | **Configuración muerta.** `reglas_flags.yaml`, `schema_path`/`extract`/`exclude_columns` (anulados por listas explícitas), `esquema_cc.tipos_datos`, `columnas_derivadas`. | Varios |
| TR-55 | **Joins por texto exacto** sin normalizar claves: DNI con `.0` sobrante, PERIODO como fecha o como texto, código de CC. | licencias, examen_retiro |
| TR-56 | **Joins que multiplican filas** cuando la clave está duplicada (3 LEFT JOIN sobre `NUMERO DE DOC` → k⁴ filas). | bd (flags) |
| TR-57 | **Fecha de referencia no determinista**: `CURRENT_DATE` del reloj del sistema en lugar de la fecha del documento. **Confirmado por paridad (2026-11-16):** con datos de corte 2025-11, el legacy reporta 3/2 alertas de jubilación en vez de 9/4. | bd |
| TR-58 | **Duplicación nómina / régimen minero**: misma estructura, distintas reglas (al minero le faltan el filtro de totales, las columnas dinámicas y `validar_constraints`). | nómina, minero |
| TR-59 | **`.xls` declarado como soportado**, aunque el lector (openpyxl) probablemente no lo abre (inferido). | Todos los contratos |

## 5. Decisiones pendientes para el rework

Propuesta inicial. Cada fila debe confirmarse antes de implementar y luego pasar a un ADR.

| ID | Decisión | Propuesta |
|---|---|---|
| TR-D1 | Normalización de cabeceras | Una sola función (mayúsculas, sin tildes, espacios colapsados) usada **igual** en preflight y en el procesamiento; alias de columnas declarados en el contrato |
| TR-D2 | Fin de datos | Configurable por contrato: `until: key_column_empty \| blank_row \| sheet_end` |
| TR-D3 | Fecha de corte | Usar `FECHA_DOCUMENTO` como fecha de referencia de todos los flags (reproducible); `CURRENT_DATE` solo como fallback explícito **Decidido (2026-11-16):** todas las alertas usan la fecha de corte del nombre de la base de personal; se registra como corrección de TR-57, no como falla de paridad. |
| TR-D4 | Joins de flags | Calcular flags en un solo `SELECT` (sin auto-joins); deduplicar o rechazar claves repetidas con un check |
| TR-D5 | Constraints | Convertir PK/nullable/rangos/business_rules en checks SQL ejecutables, con severidad `error \| warn` |
| TR-D6 | Resultado de ejecución | Estado por etapa (`ok \| warn \| failed \| skipped`); el pipeline falla si falla una etapa requerida |
| TR-D7 | Atomicidad | Escribir en un directorio temporal y promover a `actual/` solo si todo el pipeline termina bien |
| TR-D8 | Paso de datos entre etapas | Inputs/outputs declarados en el YAML por nombre lógico, no por posición ni por ruta fija |
| TR-D9 | Nómina vs minero | **Decidido (Plan 8, 2026-11-18):** la planilla de régimen especial es un reporte aparte (`planilla_regimen_especial`) con el molde de reportes, con los mismos nombres canónicos que la planilla, y **no** entra en los indicadores del tablero. Sumarla después es unir su Gold al de la planilla con una columna `regimen`. |
| TR-D10 | Rule packs | Umbrales de jubilación, contratos y practicantes (normativa peruana) como parámetros versionados |
| TR-D11 | Paridad vs corrección | Para cada bug de las secciones 8, decidir: preservar por paridad (marcado), corregir o descartar |
| TR-D12 | Formatos de Excel | Soportar `.xlsx/.xlsm`; `.xls` solo si la librería elegida lo lee de verdad (calamine sí lo hace) |

### Decisiones surgidas del rediseño de UI (2026-11-16)

Aparecieron al revisar los mocks desde los roles de CEO/CHRO y analista. Son **capacidades nuevas** que
el legacy no tiene.

| ID | Decisión | Propuesta |
|---|---|---|
| TR-D13 | Detectar la cabecera desplazada | Si la celda de cabecera esperada (p. ej. A6) no coincide, buscar las columnas requeridas en ±3 filas y ±3 columnas, e informar dónde se encontraron ("están en A7"). El archivo **no se procesa** igual: solo se mejora el mensaje de error. **Implementado (Planes 3 y 6):** el problema PRE-10 trae `celda` (esperada) y `celda_encontrada`, y la app muestra un recorte de la hoja con ambas. |
| TR-D14 | Check "descanso médico sin licencia" | Nuevo check de calidad tras enriquecer la nómina: personas con días de descanso médico > 0 y sin motivo de licencia en el periodo. Severidad `warn`. Requiere validación de negocio. |
| TR-D15 | Alertas de jubilación por año | **Decidido (2026-11-16, corrige la decisión anterior del mismo día):** las alertas son «cumple 65 (o 70) durante el año de corte / el siguiente». Para eso basta el año de nacimiento: `año(nacimiento) = año(fecha_corte) − 65` (este año) y `− 64` (próximo); ídem 70 y 69. La fórmula del legacy (BD-FLG-05…07) era correcta; la diferencia 3/2 frente a 9/4 venía solo de usar `CURRENT_DATE` (TR-57), que corrige TR-D3. No se calcula la edad exacta. |
| TR-D16 | Frescura de datos | Cada salida Gold registra su fuente y su fecha de corte. El tablero muestra la barra "Datos usados" y avisa cuando hay periodos mezclados (IND-C3). |
| TR-D17 | Trazabilidad por indicador | Cada número del tablero expone fuente, fecha de corte e IDs de regla (botón "¿De dónde sale?"). Los IDs de `docs/logica-negocio/` pasan a ser contrato entre el motor y la UI. |
| TR-D18 | Alertas en dos niveles | Resumen agregado de riesgo para dirección, sin nombres; detalle nominal solo en la pestaña "Alertas (solo RR. HH.)". |
| TR-D19 | Resumen ejecutivo en PDF | Exportar una página con los indicadores, los riesgos y la barra de frescura, para directorio. |
| TR-D20 | Clave con valor nulo | **Decidido (2026-11-16):** si la columna clave (p. ej. número de documento) trae un valor que equivale a vacío (`#N/A`, `nan`, `None`, error de Excel), la fila se descarta con un aviso PRE-24 que dice la celda. El archivo se sigue procesando. |
| TR-D21 | Fila con datos y sin clave | **Decidido (2026-11-16):** las filas totalmente vacías se descartan en silencio; las que tienen otros datos pero no tienen clave se descartan con un aviso PRE-24. Las filas de totales avisan hasta que el Plan 4 las reconozca (NOM-BRZ-10). |
| TR-D22 | Montos de la planilla | **Decidido (2026-11-16):** los montos (horas extra, licencias, descanso médico, subsidios en dinero) quedan fuera del contrato y del Gold: ningún indicador los usa y son datos salariales. La paridad compara solo las columnas presentes en ambos lados. |
| TR-D23 | Duplicados en la planilla | **Decidido (2026-11-16):** dos archivos del mismo periodo detienen el proceso (error). Una persona repetida en el mismo periodo se conserva con un aviso que dice las filas (corrige NOM-SLV-10 y NOM-GLD-15). |
| TR-D24 | Modalidad de contrato | **Decidido (2026-11-16):** una persona sin modalidad cuenta como empleado y genera un aviso (corrige BD-GLD-07 / Q-02). Las modalidades se comparan normalizadas (mayúsculas, sin tildes ni espacios sobrantes) en el split de practicantes y en las alertas de contrato (corrige Q-12). |

Las definiciones de los indicadores nuevos están en [indicadores-tablero.md](indicadores-tablero.md).
