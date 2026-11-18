# Vocabulario canónico

> Estado: aceptado · Fecha: 2026-11-16

Los nombres de columnas y reportes del legacy son abreviaturas internas: `DIAS DIST.LIC.S.GH`,
`#HRS EXTRAS DOBLES`, `SEDE3`, `WHITE COLLAR / BLUE COLLAR`… Este documento fija **un nombre genérico y
entendible** para cada concepto. Se usa así:

- **Término en la UI:** lo que ve el usuario (español claro, sin siglas).
- **Nombre canónico:** el que usan el motor, los contratos de `config/`, el SQL Gold y el generador demo
  (español, ASCII, `snake_case`; ver AGENTS.md §5).
- **Nombre legacy:** solo como origen del mapeo. Con datos reales, el mapeo columna origen → nombre canónico
  se declara en el contrato de cada fuente; nunca se codifica en Rust.

## 1. Reportes (fuentes)

| Término en la UI | Nombre canónico | Nombre legacy | Nota |
|---|---|---|---|
| Planilla mensual | `planilla` | Nómina / Planilla | "Planilla" equivale a "nómina" en otros países; el glosario de la UI lo aclara |
| Planilla régimen especial | `planilla_regimen_especial` | Nómina régimen minero | Genérico: cualquier régimen con su propio formato de planilla |
| Base de personal | `base_personal` | BD / Base de Datos | — |
| Control de licencias | `licencias` | Control de licencias | — |
| Altas de personal | `altas_personal` | Relación de ingresos (PDT) | **Se evita "ingresos"**: es ambiguo (dinero o personas) |
| Control de practicantes | `practicantes` | Control de practicantes | — |
| Exámenes médicos de salida | `examenes_salida` | Exámenes de retiro | — |

## 2. Movimientos de personal

| Término en la UI | Nombre canónico | Nombre legacy |
|---|---|---|
| Entradas (personas que entraron) | `entradas` | Ingresos, altas |
| Salidas (personas que se fueron) | `salidas` | Ceses, bajas |
| Fecha de entrada | `fecha_entrada` | `FECHA DE INGRESO`, `FECH_INGR.`, `FECHA INICIO`, `FECHA ING` |
| Fecha de salida | `fecha_salida` | `FECHA CESE`, `FECHA DE CESE` |
| Motivo de salida | `motivo_salida` | `MOTIVO CESE`, `CAUSA DE SALIDA` |
| Fin de contrato | `fecha_fin_contrato` | `Fecha de Termino` |

**Regla:** "ingresos" solo significa **dinero** (`total_ingresos`, en la planilla de régimen especial). Para
personas se dice siempre **entradas**.

## 3. Personas y organización

| Término en la UI | Nombre canónico | Nombre legacy |
|---|---|---|
| Número de documento | `numero_documento` | `DNI/CEX`, `NUMERO DE DOC`, `DNI`, `N° DOCUM.` |
| Tipo de documento | `tipo_documento` | `TIPO DE DOCUMENTO` |
| Código de empleado | `codigo_empleado` | `CÓDIGO CORPORATIVO`, `CODIGO SAP2`, `CODIGO SAP` |
| Nombre completo | `nombre_completo` | `NOMBRE COMPLETOS`, `APELLIDOS Y NOMBRES`, o apellidos + nombres |
| Gerencia | `gerencia` | `GERENCIA` |
| Área | `area` | `ÁREA` |
| Centro de costo | `centro_costo` | `CENTRO DE COSTOS`, `CC` |
| Sede | `sede` | `SEDE3`, `SEDE`, `LUGAR DE TRABAJO` |
| Tipo de trabajador: **Administrativo / Operativo** | `tipo_trabajador` | `WHITE COLLAR / BLUE COLLAR`, `WC/BC` |
| Tipo de contrato | `modalidad_contrato` | `Modalidad de Contrato` |
| Fecha de nacimiento | `fecha_nacimiento` | `FECHA DE NAC.`, `F. NACIMIENTO`, `FECHA NACIMIENTO` |
| Periodo (mes) | `periodo` (`YYYY-MM`) | `PERIODO`, `AÑO` + `MES` |
| Fecha de corte | `fecha_corte` | `FECHA_DOCUMENTO` |

## 4. Ausencias (días)

| Término en la UI | Nombre canónico | Nombre legacy |
|---|---|---|
| Descanso médico | `dias_descanso_medico` | `DIAS DESC MEDICO (P)` |
| Licencia con goce de sueldo | `dias_licencia_con_goce` | `DIAS GOCE HABER(D)` (confirmado) |
| Licencia sin goce de sueldo | `dias_licencia_sin_goce` | `DIAS DIST.LIC.S.GH` |
| Licencia por paternidad | `dias_licencia_paternidad` | `DIAS LIC. PATE (P)` |
| Subsidio por enfermedad | `dias_subsidio_enfermedad` | `DIAS SUBSIDIO ENFERMEDAD` |
| Subsidio por maternidad | `dias_subsidio_maternidad` | columna dinámica `DIAS SUB. MATERNIDAD` |
| Otros subsidios | `dias_subsidio_otros` | resto de columnas dinámicas `^DIAS SUB` |

## 5. Horas extra

| Término en la UI | Nombre canónico | Nombre legacy |
|---|---|---|
| Horas extra al 25 % | `horas_extra_25` | `# HRS EXTRAS SIMP.25%` |
| Horas extra al 35 % | `horas_extra_35` | `# HRS EXTRAS SIMP.35%` |
| Horas extra al 50 % | `horas_extra_50` | `#HRS SIMP.50%`, `HRS SIMPLES AL 50%` |
| Horas extra al 100 % (dobles) | `horas_extra_100` | `#HRS EXTRAS DOBLES` |

Las columnas legacy de **montos** de horas extra (`HORAS EXTRAS 25%`, etc.) se llaman `monto_horas_extra_<pct>`
para no confundirlas con las horas.

## 6. Alertas

| Término en la UI | Nombre canónico | Nombre legacy |
|---|---|---|
| Contrato temporal por superar su plazo máximo | `alerta_contrato_plazo` | `alerta_contrato_obra`, `alerta_contrato_incremento` |
| Llega a la edad de jubilación | `alerta_jubilacion` | `cumple_65_*`, `cumple_70_*` |

## 6b. Columnas de las capas Silver y Gold (Plan 4)

Nombres que agrega el motor al procesar; no vienen de una cabecera de Excel.

| Término en la UI | Nombre canónico | Nombre legacy | Nota |
|---|---|---|---|
| Año | `anio` | `AÑO` | Entero (el legacy usaba texto) |
| Mes (número) | `mes` | `MES` | Entero 1–12 |
| Mes | `nombre_mes` | `NOMBRE_MES` | En español: `Enero`…`Diciembre` (el legacy: `January`…) |
| Tipo de licencia | `tipo_licencia` | `TIPO_LICENCIA` | `CON_GOCE` / `SIN_GOCE` |
| Motivo de licencia con goce del mes | `motivo_licencia_con_goce` | `MOTIVO_CON_GOCE` | Motivos sin repetir, separados por ` \| ` |
| Motivo de licencia sin goce del mes | `motivo_licencia_sin_goce` | `MOTIVO_SIN_GOCE` | Ídem |
| Tiempo de servicio | `tiempo_servicio_texto` | `tiempo_servicio_texto` | `"<A> años, <M> meses"` |
| Cumple 65 este año / el próximo | `cumple_65_este_anio`, `cumple_65_proximo_anio` | `cumple_65_esteaño`, `cumple_65_proximoaño` | Por año de nacimiento (TR-D15) |
| Cumple 70 este año / el próximo | `cumple_70_este_anio`, `cumple_70_proximo_anio` | `cumple_70_esteaño`, `cumple_70_proximoaño` | Ídem |
| Alerta de contrato por obra / por incremento | `alerta_contrato_obra`, `alerta_contrato_incremento` | ídem | Componentes de `alerta_contrato_plazo` |
| Archivo y fila de origen | `archivo_origen`, `fila_origen` | — | Trazabilidad en Silver (TR-D17); no van al Gold |

## 7. Términos que la UI no usa

ETL, Bronze/Silver/Gold, Parquet, preflight, pipeline, flag, PDT, WC/BC, SAP. Pueden aparecer solo en
"Ver detalles para soporte" y en la documentación técnica.
