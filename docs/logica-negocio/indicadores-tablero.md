# Indicadores del tablero

> Estado: **aceptado** (set genérico) · Fecha: 2026-11-16
> Criterio: indicadores estándar de RR. HH. que se pueden calcular **solo con la data demo**
> (generada desde los contratos del legacy, ADR-0002), sin depender de definiciones propias de un cliente.
> Los nombres de columnas siguen el [vocabulario canónico](vocabulario.md).

Estos indicadores **no existen en el legacy**: allí los cálculos de tablero vivían en Power BI, fuera del
código. Se definen aquí para que el tablero en `localhost` muestre tasas comparables.

## 1. Decisiones de diseño

| Decisión | Elección | Por qué |
|---|---|---|
| Fuente de los indicadores principales | **Planilla** (`planilla` + `planilla_regimen_especial`) | Es mensual y trae entradas, salidas, ausencias y horas extra en una sola tabla; evita "dos cifras" entre fuentes |
| Fuente de dimensiones | **Base de personal**, unida por `numero_documento` | Aporta `gerencia`, `sede` y `tipo_trabajador`, que la planilla no trae |
| Fuente de alertas | **Base de personal** | Las reglas ya existen en el legacy (BD-FLG-*) |
| Días programados | Personas × **días calendario** del mes | Genérico; no depende de turnos ni calendarios laborales |
| Fecha de referencia | `fecha_corte` del documento o el `periodo`, nunca el reloj del sistema | Reproducible (TR-D3) |
| Fuera del set principal | `altas_personal`, `examenes_salida`, `practicantes` | Se ven en sus pestañas de detalle |

Notación: `mes` = `periodo` del filtro *Periodo*; `ant` = mes anterior. `numero_documento` se normaliza
(texto, sin `.0` final, sin espacios; ver TR-55).

## 2. Indicadores

| ID | Indicador (UI) | Pregunta | Cálculo |
|---|---|---|---|
| IND-01 | Personas en planilla | ¿A cuántas personas pagamos este mes? | `COUNT(DISTINCT numero_documento)` en la planilla de `mes` |
| IND-02 | Entradas | ¿Cuántas personas entraron? | `COUNT(DISTINCT numero_documento)` con `fecha_entrada` dentro de `mes` |
| IND-03 | Salidas | ¿Cuántas personas se fueron? | `COUNT(DISTINCT numero_documento)` con `fecha_salida` dentro de `mes`; desglose por `motivo_salida` |
| IND-04 | Rotación del mes | ¿Qué parte del personal perdimos? | `IND-03 / ((IND-01(mes) + IND-01(ant)) / 2)` |
| IND-05 | Ausentismo | ¿Qué parte del tiempo programado se perdió? | `Σ días de ausencia (§3) / (IND-01 × días calendario de mes)` |
| IND-06 | Horas extra por persona | ¿Cuántas horas extra hace cada persona, en promedio? | `(horas_extra_25 + horas_extra_35 + horas_extra_50 + horas_extra_100) / IND-01`, con desglose por tipo |
| IND-07 | Contratos por superar su plazo | ¿Qué contratos temporales están por superar su plazo máximo? | Conteo de `alerta_contrato_plazo` (BD-FLG-08, BD-FLG-09) |
| IND-08 | Edad de jubilación | ¿Quién llega a 65 o 70 este año o el próximo? | Conteo de `alerta_jubilacion` (BD-FLG-05…07; ver TR-D15) |

Todos responden a los filtros *Gerencia*, *Sede* y *Tipo de trabajador* mediante la unión con la base de
personal. IND-07 e IND-08 usan la última base de personal disponible y no responden al filtro *Periodo*.

## 3. Días de ausencia por motivo (IND-05)

| Motivo en la UI | Columna canónica |
|---|---|
| Descanso médico | `dias_descanso_medico` |
| Licencia con goce de sueldo | `dias_licencia_con_goce` |
| Licencia sin goce de sueldo | `dias_licencia_sin_goce` |
| Licencia por paternidad | `dias_licencia_paternidad` |
| Subsidio por enfermedad | `dias_subsidio_enfermedad` |
| Subsidio por maternidad | `dias_subsidio_maternidad` |
| Otros subsidios | `dias_subsidio_otros` |

Cada columna se suma una sola vez. No se usa el total legacy `DIAS SUBSIDIOS TOTAL`, para no contar
dos veces. Con datos reales, el mapeo desde las columnas de origen se declara en el contrato de la fuente
(ver [vocabulario.md](vocabulario.md) §4).

## 4. Comparaciones y metas

- Cada indicador se compara con `ant`. La flecha indica la dirección y una palabra dice si es mejor o peor;
  nunca solo el color.
- Las metas son configurables. Mientras no existan, el tablero muestra "Sin meta definida".
- Sentido de "mejor": rotación ↓, ausentismo ↓, horas extra por persona ↓, contratos por superar su plazo ↓.
  IND-01 es neutro.

## 5. Checks de calidad (se muestran al usuario)

| ID | Check | Qué muestra el tablero |
|---|---|---|
| IND-C1 | Personas de la planilla sin coincidencia en la base de personal | En el "¿De dónde sale?" de IND-01: "N personas no aparecen al filtrar por gerencia o sede · Ver quiénes" |
| IND-C2 | Fuentes con periodos distintos | Barra "Datos usados" con aviso ⚠ (TR-D16) |
| IND-C3 | `IND-01(mes) − IND-01(ant)` frente a `IND-02 − IND-03` | Informativo: la diferencia se explica por salidas a fin de mes que todavía cobran |

## 6. Requisitos para el generador de data demo

1. **Meses consecutivos:** varios meses de planilla (mínimo 6) con los mismos `numero_documento` de un mes a otro.
2. **Movimientos coherentes:** quien tiene `fecha_entrada` en un mes aparece desde ese mes, y quien tiene
   `fecha_salida` deja de aparecer al mes siguiente.
3. **Base de personal que cubre la planilla,** salvo un pequeño porcentaje a propósito, para ejercitar IND-C1.
4. **Ausencias y horas extra:** todas las columnas de §3 y de horas extra pobladas con distribuciones plausibles,
   incluida al menos una columna dinámica de subsidio.
5. **Casos de alerta:** contratos temporales cerca de su plazo máximo y personas cerca de los 65 y 70 años,
   para que IND-07 e IND-08 tengan casos.
6. **Excel con nombres de columna legacy:** el generador produce los Excel con las columnas del legacy, para
   ejercitar el mapeo hacia el vocabulario canónico.

## 7. Implementación (Plan 5)

Las fórmulas viven en SQL de DuckDB ([ADR-0004](../adr/0004-api-local-tablero.md)) y se calculan en cada
pedido a la API local (`etl-local tablero`), con los filtros recibidos. Los metadatos (nombre, pregunta, unidad,
sentido de "mejor", reglas, etiquetas) están en `config/indicadores.yaml` y las metas en
`config/parametros.yaml` (`metas`).

| ID | Archivo |
|---|---|
| IND-01…06 | `config/sql/indicadores/20_mensuales.sql` (`ind_valor`, `ind_desglose`) |
| IND-07, IND-08 | `config/sql/indicadores/30_alertas.sql` (no responden al periodo) |
| Desglose por gerencia (IND-01, IND-07, IND-08; Plan 7) | `config/sql/indicadores/35_por_gerencia.sql` (`ind_por_gerencia`), campo `por_gerencia` |
| IND-C1, IND-C3 | `config/sql/indicadores/40_checks.sql` |
| IND-C2 | `crates/core/src/tablero.rs` (`Tablero::estado`), desde `ejecucion.json` |
| Filtros | `config/sql/indicadores/10_filtrado.sql` (`planilla_f`, `empleados_f`) |
| Detalle nominal (TR-D18) | `config/sql/indicadores/50_detalle.sql`, solo en `/api/v1/detalle/…` |

Decisiones de implementación:

- **Periodo por defecto:** el último `periodo` de la planilla. Uno que no esté en los datos es un filtro inválido.
- **Filtros de dimensión:** con un filtro de gerencia, sede o tipo de trabajador, las personas de la planilla sin
  registro en la base de personal quedan fuera; IND-C1 dice cuántas son. Un valor que no existe en la base
  (comparación exacta) es un error que lista los valores válidos.
- **Unidades:** las tasas van como fracción (`0.0073` = 0,73 %); la UI las formatea.
- **IND-C2:** hay aviso cuando el mes de la fecha de corte de la base de personal difiere del último periodo de la
  planilla.
- **Sin mes anterior en los datos:** IND-04 e IND-C3 quedan en `null` con una nota, y ningún indicador lleva
  comparación. Todo divisor en cero da `null`, nunca `NaN`.
- **IND-05 e IND-06** suman todas las filas de la planilla del mes, incluida la persona repetida que conserva
  TR-D23; el divisor (IND-01) cuenta personas distintas.
- **Sentido de "mejor":** IND-02, IND-03 e IND-08 son neutros (§4 no les da sentido).
- **Lectura de `actual/`:** en cada pedido la API copia los Parquet Gold a memoria y suelta los archivos, para no
  bloquear un `procesar` que esté reemplazando `actual/` en Windows. Qué fuentes hay lo dice `ejecucion.json`:
  si una fuente procesada no tiene su Parquet, la API responde con un error que pide reintentar.
- **Practicantes:** IND-C1 cruza la planilla con `base_personal_empleados`. Si una planilla real incluye
  practicantes, contarán como "sin base" (pendiente; el demo no tiene practicantes en la planilla).
