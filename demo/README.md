# Generador de data demo

Genera **Excel sintéticos** con el mismo layout que los archivos operativos del sistema anterior
(planilla mensual, base de personal y control de licencias), más un **manifiesto** con los valores
exactos que deberían salir en cada indicador. Sirve para tres cosas:

1. **Demo:** cualquiera puede clonar el repo, generar datos y ver el tablero sin datos reales.
2. **Tests del motor:** lo que el motor calcula debe cuadrar con el manifiesto.
3. **Paridad con el legacy** (Plan 2): el sistema anterior se corre sobre estos mismos archivos.

Todo es ficticio: la empresa es `EMPRESA DEMO S.A.C.`, los números de documento empiezan por `99`
y los nombres los inventa [Faker](https://faker.readthedocs.io/). Ver [ADR-0002](../docs/adr/0002-demo-python-uv.md).

## Uso

Requiere [uv](https://docs.astral.sh/uv/). Desde la raíz del repositorio:

```bash
uv sync --project demo                          # crea demo/.venv
uv run --project demo generar-demo              # datos limpios en demo/generated/limpio/
uv run --project demo pytest demo/tests         # tests del generador
```

| Opción | Por defecto | Qué hace |
|---|---|---|
| `--semilla` | `42` | Misma semilla ⇒ mismos archivos, byte a byte |
| `--periodo-fin` | `2025-11` | Último mes generado (`AAAA-MM`) |
| `--meses` | `6` | Cantidad de meses consecutivos |
| `--salida` | `demo/generated` | Carpeta base de salida |
| `--modo` | `limpio` | `limpio`, `sucio` o `casos` |

El generador **solo borra** carpetas de salida que creó él mismo (las reconoce por su manifiesto).
Si la carpeta existe con otros archivos, se detiene sin tocar nada.

## Qué genera (`--modo limpio`)

```text
demo/generated/limpio/
├── planilla/
│   ├── Planilla 2025-06 Empleados.xlsx      # hoja "Planilla", cabecera en A6
│   ├── …                                     # un archivo por mes
│   └── licencias/CONTROL DE LICENCIAS.xlsx  # hojas con goce / sin goce, cabecera en fila 2
├── base_personal/BD. 12.11.2025..xlsx        # hoja "Personal", cabecera en A10
├── planilla_regimen_especial/               # Plan 8: "Planilla Regimen Especial AAAA-MM.xlsx", hoja "Planilla", A6
├── altas_personal/                          # Plan 8: "Altas de personal AAAA-MM.xlsx", hojas EMPLEADOS y PRACTICANTES
├── practicantes/BD Practicantes 12.11.2025.xlsx  # Plan 8: hoja "Practicantes", cabecera en A4
├── examenes_salida/Examenes de salida.xlsx  # Plan 8: hoja "DATA", cabecera en A3
└── manifiesto.json                           # verdad de referencia
```

Casos plantados a propósito para que los indicadores tengan qué mostrar:

| Caso | Cantidad | Indicador |
|---|---|---|
| Contratos por superar su plazo (11 por obra, 7 por incremento) | 18 | IND-07 |
| Personas que llegan a 65 (9) o 70 años (4) | 13 | IND-08 |
| Personas de la planilla del último mes sin registro en la base de personal | 8 | IND-C1 |
| Persona que entra y sale el mismo mes | 1 | IND-02 / IND-03 |
| Columna dinámica `DIAS SUB. MATERNIDAD` desde el tercer mes | — | IND-05 |
| Practicantes (solo en la base de personal) | 60 | — |
| Planilla de régimen especial: grupo propio que no está en la planilla estándar (trae montos, que el motor no lee) | 40 por mes | — |
| Practicantes profesionales por cumplir 1 año (6), con 1 año cumplido (13, de ellos 5 por cumplir 2) y sin renovación (2) | 50 + 10 pre profesionales | Control de practicantes |
| Exámenes de salida: 2 sin código de centro de costo, 2 con un código inexistente (`CC-999`) y 3 practicantes (`STUDENT INTERNAL`) | — | Exámenes de salida |

## Modo `sucio`

`uv run --project demo generar-demo --modo sucio` genera lo mismo que `limpio` y luego le agrega desorden
realista **sin cambiar los conteos** (el manifiesto es idéntico salvo el campo `modo`):

| ID | Qué cambia | Regla que ejercita |
|---|---|---|
| SUC-01 | Número de documento guardado como número en ~5 % de las filas de la planilla | TR-55 |
| SUC-02 | Espacios de más en 3 encabezados de la planilla | NOM-BRZ-07 |
| SUC-03 | Fechas de entrada como texto `DD/MM/AAAA` en ~10 % de la base de personal | BD-GLD-05 |
| SUC-04 | Celda con error de Excel (se ve como `#N/D`, se guarda como `#N/A`) en el centro de costos de ~2 % de las filas | TR-12 |
| SUC-05 | Una fila vacía a mitad de los datos de la planilla | NOM-BRZ-09 |
| SUC-06 | Número de documento con `.0` al final en licencias | LIC-SLV-02 |

## Modo `casos`

`uv run --project demo generar-demo --modo casos` genera una subcarpeta por **archivo roto**, cada una con
un `esperado.json` que dice qué regla se incumple y dónde (hoja y celda):

| Caso | Problema |
|---|---|
| CASO-01 | La cabecera de la planilla está en A7 en vez de A6 (se insertó una fila) |
| CASO-02 | La hoja de la planilla se llama `Hoja1` |
| CASO-03 | Falta la columna `DNI/CEX` |
| CASO-04 | El nombre de la planilla no trae el periodo |
| CASO-05 | El nombre de la base de personal no sigue el formato `BD. DD.MM.AAAA..xlsx` |
| CASO-06 | El control de licencias no tiene la hoja `LICENCIA SIN GOCE` |
| CASO-07 | Falta la columna `DNI/CEX` en la planilla de régimen especial |
| CASO-08 | Las altas de personal no tienen la hoja `PRACTICANTES` |
| CASO-09 | El control de practicantes se llama `Practicantes.xlsx` (sin fecha de corte) |
| CASO-10 | Falta la columna `FECHA DE CESE` en los exámenes de salida |

## Paridad con el legacy (`generar-golden`)

Corre el sistema anterior (`HR_Data_Integration_Pipeline/`, **solo lectura**) sobre esta misma data y guarda
sus salidas en `demo/golden/fixtures/<modo>/` como referencia para el motor nuevo. Solo funciona donde
exista el proyecto legacy: en un clon open source, el comando y sus tests se omiten con un mensaje.

```bash
uv sync --project demo --group paridad          # dependencias exactas del legacy (Python 3.12)
uv run --project demo generar-golden --modo limpio --meses 3   # también: sucio, casos
```

- **Python 3.12:** `duckdb==1.5.2` (la versión del legacy) no tiene binarios para 3.15; por eso `demo/` fija
  3.12 en `.python-version`.
- **Aislamiento:** el legacy corre en un subproceso con su carpeta de trabajo temporal, sin escribir bytecode
  ni logs en su propia carpeta (hay un test que lo verifica).
- **Confidencialidad:** el nombre del cliente nunca se escribe en este repo. Se lee del legacy en tiempo de
  ejecución, se aplica solo a la copia temporal y se reemplaza por `<CLIENTE>` en todo lo que se guarda.

Los fixtures versionados se generan con `--meses 3`, igual que los de los tests de Rust
(`crates/core/tests/fixtures/`), para que ambos usen la misma población. Cada carpeta `limpio/` y `sucio/` trae 9 parquet con nombres canónicos y un `resumen.json` con la fecha de
ejecución, las versiones y la **comparación con el manifiesto**. `casos/` trae, por caso, los errores que
reportó el preflight del legacy.

**Hallazgos (2026-11-16):**

| Comparación | limpio | sucio | Explicación |
|---|---|---|---|
| Filas de planilla por periodo, días con goce, empleados, practicantes | Coincide | Coincide | — |
| Alertas de contrato (obra / incremento) | 11 / 7 | **10** / 7 | Con fechas de ingreso escritas como texto (SUC-03), el worker legacy las vuelve nulas (BD-DIF-03) y la alerta se pierde |
| Alertas de jubilación (65 / 70) | **3 / 2** (esperado 9 / 4) | igual | El legacy usa `CURRENT_DATE` en vez de la fecha de corte (TR-57): el resultado cambia según el día en que se corre |
| CASO-04 (planilla sin periodo en el nombre) | — | — | El preflight legacy **no lo detecta** (0 errores, NOM-BRZ-05); el motor nuevo sí debe |

**Columnas que cambian con la fecha de ejecución.** En `base_personal_empleados_flags.parquet`, el legacy
usa `CURRENT_DATE`:
- `tiempo_servicio_texto` cambia **cada mes** para los contratos sin fecha de término (unos 700 de 1.230);
- `cumple_65_*` y `cumple_70_*` cambian **cada año**.

Por eso estos fixtures solo son idénticos byte a byte si se regeneran el mismo día. Para comparar con el
motor nuevo, hay que usar `fecha_ejecucion` del `resumen.json` como si fuera la fecha actual. El
`resumen.json` lista estas columnas en `dependencias_de_fecha`.

**Si una corrida falla**, los fixtures anteriores se conservan: todo se arma en una carpeta temporal y solo
se publica si el legacy terminó bien en todos los dominios. Si no, el comando termina con código 1.

## El manifiesto

`manifiesto.json` se calcula desde la población simulada, no releyendo los Excel. Por periodo
incluye personas en planilla, entradas, salidas, personas sin base, días de ausencia por motivo y
horas extra por tipo; para la base de personal, la fecha de corte y los casos plantados. Los nombres
siguen el [vocabulario canónico](../docs/logica-negocio/vocabulario.md).
