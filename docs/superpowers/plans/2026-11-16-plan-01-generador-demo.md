# Plan 1 — Generador de data demo: plan de implementación

> **Para agentes:** SUB-SKILL REQUERIDA: usar superpowers:subagent-driven-development (recomendado) o
> superpowers:executing-plans para implementar este plan tarea por tarea. Los pasos usan casillas (`- [ ]`).

**Objetivo:** un proyecto Python en `demo/` que genera, con semilla fija, Excel sintéticos con el layout
del legacy (planilla mensual, base de personal y control de licencias) y un manifiesto JSON con la verdad
de referencia de cada indicador.

**Arquitectura:** primero se simula una población en memoria con nombres canónicos (personas, entradas,
salidas, ausencias, horas extra). Después, unos *writers* traducen esa población a los Excel con los
nombres de columna legacy. Un módulo aparte ensucia los archivos sin cambiar los conteos (modo `sucio`)
o genera un archivo roto por cada caso de error (modo `casos`). El manifiesto se calcula desde la
población, no releyendo los Excel.

**Stack:** Python ≥ 3.12, uv, `openpyxl`, `faker`, `pytest`, `ruff`.

**Spec (leer junto con este plan):**
- [ADR-0002](../../adr/0002-demo-python-uv.md): decisión y reglas de `demo/`.
- [indicadores-tablero.md](../../logica-negocio/indicadores-tablero.md) §2, §3 y §6: qué debe permitir medir la data.
- [vocabulario.md](../../logica-negocio/vocabulario.md): nombres canónicos y su equivalente legacy.
- Layouts legacy: [nomina.md](../../logica-negocio/nomina.md) (NOM-BRZ-04…11, NOM-SLV-04),
  [bd.md](../../logica-negocio/bd.md) (BD-BRZ-02, 05, 10, 11; BD-GLD-06; BD-FLG-05…09),
  [licencias.md](../../logica-negocio/licencias.md) (LIC-BRZ-01…05).

## Restricciones globales

- **Desviación consciente del ADR-0002:** el ADR pide que el generador lea los contratos YAML del motor, pero ese formato (ADR-0003) todavía no existe. En este plan los layouts viven en las tablas de columnas de los writers (`COLUMNAS_PLANILLA`, `COLUMNAS_BASE_PERSONAL`). Cuando se apruebe ADR-0003, el Plan 3 incluye la migración del generador a leer los contratos.
- **Sin git:** el repo aún no está inicializado y el usuario decidió no hacerlo todavía. Los planes no incluyen pasos de commit.
- **Idioma:** español en identificadores, comentarios y mensajes; identificadores en ASCII, sin tildes ni ñ (AGENTS.md §5).
- **Nombres:** canónicos de `vocabulario.md` en todo el código; los nombres legacy solo aparecen en las tablas de columnas de los writers.
- **Confidencialidad:** prohibido el nombre del cliente. La empresa ficticia es `EMPRESA DEMO S.A.C.`. Los `numero_documento` son `"99"` + 6 dígitos; los nombres de personas salen de Faker (`es_ES`) con la semilla.
- **Determinismo:** misma semilla y mismos parámetros ⇒ archivos idénticos byte a byte. Todo azar sale de un `random.Random(semilla)` y de `Faker.seed_instance(semilla)`; está prohibido `random` global y `datetime.now()`.
- **Aislamiento:** `demo/` tiene su propio `pyproject.toml` y `uv.lock`, y la salida por defecto `demo/generated/` ya está en `.gitignore`.
- **Valores por defecto:** `semilla=42`, `periodo_fin="2025-11"`, `meses=6`, `personas_iniciales=1200`, `entradas_por_mes=(10, 30)`, `salidas_por_mes=(8, 25)`, `practicantes=60`, `personas_sin_base=8`, `dia_corte_base=12`, `casos_contrato_plazo=18` (11 por obra, 7 por incremento), `casos_jubilacion=13` (9 de 65 años, 4 de 70), `indice_inicio_subsidio_maternidad=2`.
- **Calidad:** `uv run --project demo ruff check` y `ruff format --check` sin errores en cada tarea.

## Foco de revisión

1. **Entra y sale el mismo mes:** debe aparecer en la planilla de ese mes y contar en entradas y en salidas. Test en la Tarea 1.
2. **Meses de 28/30/31 días:** los días de ausencia nunca superan los días calendario del mes ni los días activos de la persona. Test en la Tarea 2 con un rango que incluya febrero.
3. **Byte a byte:** openpyxl escribe fechas de creación en las propiedades del libro. Hay que fijarlas, o dos corridas con la misma semilla difieren. Test en la Tarea 3.
4. **Segunda corrida sobre la misma salida:** si cambian los meses, deben desaparecer los archivos de la corrida anterior (riesgo TR-02). Test en la Tarea 5.
5. **Suciedad que no altera conteos:** cada inyector del modo `sucio` mantiene el manifiesto válido. Test en la Tarea 6.

---

### Tarea 1: Proyecto `demo/` y simulación de la población

**Archivos:**
- Crear: `demo/pyproject.toml`, `demo/src/generador_demo/__init__.py`, `demo/src/generador_demo/parametros.py`, `demo/src/generador_demo/poblacion.py`
- Test: `demo/tests/test_poblacion.py`

**Interfaces:**
- Produce `parametros.py`:
  - `ParametrosGeneracion`: dataclass congelada con los valores por defecto de las Restricciones globales.
  - `periodos(p) -> list[str]`: `meses` periodos `YYYY-MM` consecutivos que terminan en `periodo_fin`.
- Produce `poblacion.py`:
  - `Persona`: dataclass congelada con estos campos canónicos:
    - identificación: `numero_documento`, `tipo_documento` (`"DNI"`), `codigo_empleado` (`"E"` + 5 dígitos);
    - nombre y datos personales: `apellido_paterno`, `apellido_materno`, `nombres`, `sexo` (`"M"`/`"F"`), `fecha_nacimiento: date`;
    - organización: `gerencia`, `area`, `centro_costo` (`"CC-"` + 4 dígitos), `nombre_centro_costo`, `categoria_centro_costo`, `sede`;
    - contrato: `tipo_trabajador` (`"WHITE COLLAR"`/`"BLUE COLLAR"`), `modalidad_contrato`, `servicio`, `regimen_trabajo`, `remuneracion_mensual: float`;
    - movimientos: `fecha_entrada: date`, `fecha_fin_contrato: date | None`, `fecha_salida: date | None`, `motivo_salida: str | None`;
    - `es_practicante: bool`, `en_base_personal: bool`.
  - `Poblacion`: dataclass con `periodos: list[str]`, `personas: list[Persona]`, `fecha_corte_base: date`, y los métodos:
    - `en_planilla(periodo) -> list[Persona]`: no practicantes, con `fecha_entrada <= fin_de_mes` y (`fecha_salida is None` o `fecha_salida >= inicio_de_mes`);
    - `en_base_personal() -> list[Persona]`: activos en `fecha_corte_base` con `en_base_personal`, más los practicantes.
  - `simular_poblacion(p: ParametrosGeneracion) -> Poblacion`.

- [ ] **Paso 1: crear `demo/pyproject.toml`.** Nombre `generador-demo`; `requires-python = ">=3.12"`; dependencias `openpyxl>=3.1` y `faker>=30`; grupo dev con `pytest>=8` y `ruff>=0.6`; script `generar-demo = "generador_demo.cli:main"`; layout `src/`. Correr `uv sync --project demo`. Esperado: entorno creado sin errores.

- [ ] **Paso 2: escribir los tests que fallan** en `demo/tests/test_poblacion.py`:

```python
def test_periodos_por_defecto():
    assert periodos(ParametrosGeneracion()) == ["2025-06","2025-07","2025-08","2025-09","2025-10","2025-11"]

def test_misma_semilla_misma_poblacion():
    assert simular_poblacion(ParametrosGeneracion()) == simular_poblacion(ParametrosGeneracion())

def test_documentos_unicos_y_ficticios():
    pob = simular_poblacion(ParametrosGeneracion())
    docs = [x.numero_documento for x in pob.personas]
    assert len(docs) == len(set(docs))
    assert all(len(d) == 8 and d.startswith("99") for d in docs)

def test_movimientos_coherentes():
    # quien sale en un periodo no aparece en el siguiente; quien entra aparece desde su mes
    ...  # recorrer pob.periodos y verificar ambas reglas para cada persona

def test_entra_y_sale_mismo_mes():
    # Foco de revisión 1: la simulación planta al menos 1 caso; aparece en la planilla de ese mes
    ...

def test_casos_plantados_en_base_personal():
    pob = simular_poblacion(ParametrosGeneracion())
    base = pob.en_base_personal()
    corte = pob.fecha_corte_base
    obra = [x for x in base if x.modalidad_contrato == "CONTRATO POR OBRA O SERVICIO ESPECIFICO"
            and (corte - x.fecha_entrada).days / 365.25 >= 4.5]
    incr = [x for x in base if x.modalidad_contrato == "CONTRATO POR INCREMENTO DE ACTIVIDAD"
            and (corte - x.fecha_entrada).days / 365.25 >= 2.5]
    assert (len(obra), len(incr)) == (11, 7)
    jub65 = [x for x in base if x.fecha_nacimiento.year in (corte.year - 65, corte.year + 1 - 65)]
    jub70 = [x for x in base if x.fecha_nacimiento.year in (corte.year - 70, corte.year + 1 - 70)]
    assert (len(jub65), len(jub70)) == (9, 4)

def test_practicantes_y_sin_base():
    pob = simular_poblacion(ParametrosGeneracion())
    assert sum(x.es_practicante for x in pob.personas) == 60
    assert all(x.modalidad_contrato == "TERMINO DE CONVENIO" for x in pob.personas if x.es_practicante)
    ultimo = pob.en_planilla("2025-11")
    assert sum(not x.en_base_personal for x in ultimo) == 8
```

- [ ] **Paso 3: correr los tests y comprobar que fallan.** Comando: `uv run --project demo pytest demo/tests/test_poblacion.py -v`. Esperado: FAIL por `ImportError`.

- [ ] **Paso 4: implementar `parametros.py` y `poblacion.py`.**
  - **Población inicial:** `personas_iniciales` con `fecha_entrada` anterior al primer periodo.
  - **Movimientos por periodo:** entradas y salidas en los rangos de los parámetros. Las fechas caen dentro del mes y los motivos de salida salen de `["RENUNCIA", "FIN DE CONTRATO", "DESPIDO", "JUBILACION"]`.
  - **Casos plantados:** contratos con antigüedad ≥ 4,5 / 2,5 años y nacimientos en los años objetivo, siempre activos en la fecha de corte. Ninguna otra persona puede cumplir esos criterios por azar: al sortear antigüedad y edad, se excluyen esos rangos.
  - **Modalidades:** `PLAZO INDETERMINADO`, `CONTRATO POR OBRA O SERVICIO ESPECIFICO`, `CONTRATO POR INCREMENTO DE ACTIVIDAD`, más `TERMINO DE CONVENIO` para practicantes.
  - **Dimensiones:** gerencias `Operaciones, Mantenimiento, Administración, Comercial, Seguridad, Tecnología`, con pesos `48, 23, 11, 8, 6, 4`. Sedes `Sede Central, Planta Norte, Planta Sur`.

- [ ] **Paso 5: correr los tests y ruff.** Comando: `uv run --project demo pytest demo/tests/test_poblacion.py -v` y luego `uv run --project demo ruff check demo`. Esperado: PASS y 0 errores.

---

### Tarea 2: Registros mensuales de ausencias y horas extra

**Archivos:**
- Crear: `demo/src/generador_demo/registros.py`
- Test: `demo/tests/test_registros.py`

**Interfaces:**
- Usa: `Poblacion`, `Persona`, `ParametrosGeneracion` (Tarea 1).
- Produce:
  - `RegistroMes`: dataclass congelada con:
    - `numero_documento`, `periodo`;
    - días (`int`): `dias_descanso_medico`, `dias_licencia_con_goce`, `dias_licencia_sin_goce`, `dias_licencia_paternidad`, `dias_subsidio_enfermedad`, `dias_subsidio_maternidad`;
    - horas (`float`): `horas_extra_25`, `horas_extra_35`, `horas_extra_50`, `horas_extra_100`;
    - motivos (`str | None`): `motivo_licencia_con_goce`, `motivo_licencia_sin_goce`.
  - `generar_registros(pob: Poblacion, p: ParametrosGeneracion) -> list[RegistroMes]`: un registro por cada (persona, periodo) en `pob.en_planilla(periodo)`.
  - `CAMPOS_DIAS: tuple[str, ...]` y `CAMPOS_HORAS: tuple[str, ...]`: los nombres de los campos anteriores, que reutilizan las Tareas 3 y 5.

- [ ] **Paso 1: escribir los tests que fallan:**
  - `test_dias_no_superan_mes_ni_dias_activos`: con `ParametrosGeneracion(periodo_fin="2025-03", meses=3)` (incluye febrero), la suma de `CAMPOS_DIAS` ≤ los días activos de la persona en el mes.
  - `test_licencias_con_motivo`: `dias_licencia_con_goce > 0` ⇔ `motivo_licencia_con_goce is not None`; lo mismo para sin goce.
  - `test_maternidad_y_paternidad_por_sexo`: maternidad > 0 solo si `sexo == "F"`; paternidad > 0 solo si `sexo == "M"`.
  - `test_subsidio_maternidad_desde_indice`: en los periodos con índice < `indice_inicio_subsidio_maternidad`, todos los `dias_subsidio_maternidad == 0`, y en el último periodo hay al menos un caso > 0.
  - `test_horas_extra_operativos`: el promedio de horas extra de `BLUE COLLAR` es mayor que el de `WHITE COLLAR`.

- [ ] **Paso 2: correr los tests y comprobar que fallan.** Comando: `uv run --project demo pytest demo/tests/test_registros.py -v`. Esperado: FAIL por `ImportError`.

- [ ] **Paso 3: implementar `generar_registros`.** Probabilidades mensuales por persona:
  - descanso médico 8 % (1–5 días);
  - licencia con goce 5 % (1–3 días);
  - licencia sin goce 2 % (1–5 días);
  - subsidio por enfermedad 1,5 % (5–20 días);
  - maternidad 0,4 % de las mujeres (mes completo activo);
  - paternidad 0,6 % de los hombres (1–10 días).

  Si la suma pasa de los días activos, se recorta en ese mismo orden. Motivos con goce: `["MATRIMONIO", "FALLECIMIENTO DE FAMILIAR", "CAPACITACION"]`; sin goce: `["ASUNTOS PERSONALES", "ESTUDIOS"]`. Horas extra: 0–20 para operativos y 0–4 para administrativos, repartidas entre 25 / 35 / 50 / 100 %.

- [ ] **Paso 4: correr los tests y ruff.** Esperado: PASS y 0 errores.

---

### Tarea 3: Writer de la planilla mensual (layout legacy)

**Archivos:**
- Crear: `demo/src/generador_demo/excel_comun.py`, `demo/src/generador_demo/excel_planilla.py`
- Test: `demo/tests/test_excel_planilla.py`

**Interfaces:**
- Usa: `Poblacion`, `RegistroMes`, `CAMPOS_DIAS` (Tareas 1–2).
- Produce:
  - `guardar_libro(wb: Workbook, ruta: Path) -> Path` en `excel_comun.py`. Fija `wb.properties.created` y `wb.properties.modified` en `datetime(2025, 1, 1)` y `creator` en `"generador-demo"` antes de guardar (Foco de revisión 3).
  - `COLUMNAS_PLANILLA: list[tuple[str, Callable[[Persona, RegistroMes], object]]]`: encabezado legacy y extractor. Orden exacto del esquema legacy, sin `PERIODO`, `AÑO`, `MES`:
    - identificación: `CÓDIGO CORPORATIVO, APELLIDO PATERNO, APELLIDO MATERNO, NOMBRES, TIPO DE DOCUMENTO, DNI/CEX, SEXO, F. NACIMIENTO`;
    - organización y movimientos: `ÁREA, CENTRO DE COSTOS, FECHA DE INGRESO, FECHA CESE, MOTIVO CESE`;
    - días y horas: `DIAS SUBSIDIO ENFERMEDAD, DIAS GOCE HABER(D), # HRS EXTRAS SIMP.25%, # HRS EXTRAS SIMP.35%, #HRS EXTRAS DOBLES, #HRS SIMP.50%, HRS SIMPLES AL 50%`;
    - montos: `HORAS EXTRAS 25%, HORAS EXTRAS 35%, HORAS EXTRAS 100%, LICENCIA GOCE HABER, LICENCIA PATERNIDAD`;
    - más días: `DIAS DIST.LIC.S.GH, DIAS LIC. PATE (P), DIAS DESC MEDICO (P)`;
    - montos de subsidios: `DESCANSO MEDICO, SUBS. POR ENFERMEDAD, SUBS. POR ACCIDENTE, SUBSIDIO MATERNIDAD`.

    Los montos se calculan así: días × `remuneracion_mensual` / 30, y horas × `remuneracion_mensual` / 240 × (1 + porcentaje). Las dos columnas de 50 % reciben el mismo valor.
  - `escribir_planilla(pob, registros, p: ParametrosGeneracion, periodo, carpeta: Path) -> Path`: escribe `carpeta / f"Planilla {periodo} Empleados.xlsx"`. Agrega la columna dinámica cuando `pob.periodos.index(periodo) >= p.indice_inicio_subsidio_maternidad`.

- [ ] **Paso 1: escribir los tests que fallan:**
  - `test_nombre_y_hoja`: el archivo cumple la regex `(\d{4}-\d{2})` (NOM-BRZ-04) y la única hoja se llama `Planilla`.
  - `test_cabecera_en_fila_6`: los valores de A6 en adelante son los encabezados de `COLUMNAS_PLANILLA`, en orden. Si el índice del periodo ≥ `indice_inicio_subsidio_maternidad`, al final aparece la columna dinámica `DIAS SUB. MATERNIDAD`; si no, no aparece.
  - `test_filas_de_datos`: desde la fila 7, la cantidad de filas con A no vacía y que no son totales es igual a `len(pob.en_planilla(periodo))`.
  - `test_filas_de_totales`: las dos últimas filas tienen en A `"+ TOTAL INGRESOS"` y `"- TOTAL DESCUENTOS"` (filtradas por NOM-BRZ-10).
  - `test_titulo_sin_cliente`: A1 = `"PLANILLA DE REMUNERACIONES"`, A2 = `f"Periodo: {periodo}"` y A3 = `"Empresa: EMPRESA DEMO S.A.C."`.
  - `test_fechas_son_fechas`: las celdas de `F. NACIMIENTO` y `FECHA DE INGRESO` son `datetime` o `date`.
  - `test_byte_a_byte`: dos escrituras con la misma semilla en carpetas distintas producen el mismo SHA-256.

- [ ] **Paso 2: correr los tests y comprobar que fallan.** Esperado: FAIL.

- [ ] **Paso 3: implementar `excel_comun.py` y `excel_planilla.py`.** Las filas 4–5 van vacías y las personas se ordenan por `numero_documento`.

- [ ] **Paso 4: correr los tests y ruff.** Esperado: PASS y 0 errores.

---

### Tarea 4: Writers de la base de personal y del control de licencias

**Archivos:**
- Crear: `demo/src/generador_demo/excel_base_personal.py`, `demo/src/generador_demo/excel_licencias.py`
- Test: `demo/tests/test_excel_base_licencias.py`

**Interfaces:**
- Usa: `Poblacion`, `RegistroMes`, `guardar_libro` (Tareas 1–3).
- Produce:
  - `COLUMNAS_BASE_PERSONAL`, en este orden:
    - las 13 requeridas: `NUMERO DE DOC, CODIGO SAP2, NOMBRE COMPLETOS, GERENCIA, SEXO, SEDE3, WHITE COLLAR / BLUE COLLAR, Modalidad de Contrato, Fecha de Termino, SERVICIO, REGIMEN DE TRABAJO, FECH_INGR., FECHA DE NAC.`;
    - más `CC, NOMBRE CC, CATEGORIA CC` (BD-BRZ-11).

    `NOMBRE COMPLETOS` = `f"{apellido_paterno} {apellido_materno} {nombres}"`.
  - `escribir_base_personal(pob, carpeta, nombre_hoja: str = "Personal") -> Path`: el archivo se llama `f"BD. {corte:%d.%m.%Y}..xlsx"` (dos puntos antes de la extensión, BD-BRZ-02). Cabecera en A10, datos desde la fila 11, filas 1–9 de título.
  - `escribir_licencias(pob, registros, carpeta) -> Path`: escribe `carpeta / "CONTROL DE LICENCIAS.xlsx"` con dos hojas:
    - `LICENCIA CON GOCE`: cabecera en la fila 2 con `DNI/CEX, PERIODO, APELLIDO PATERNO, APELLIDO MATERNO, NOMBRES, MOTIVO DE LIC.CON GOCE`;
    - `LICENCIA SIN GOCE`: misma cabecera, pero con `MOTIVO DE LIC.S.G.H` en la última columna.

    Va una fila por registro con días > 0, y `PERIODO` como texto `YYYY-MM`. `DNI/CEX` va en la columna A porque el legacy filtra por esa columna (LIC-BRZ-04).

- [ ] **Paso 1: escribir los tests que fallan:**
  - `test_nombre_base_personal`: el nombre del archivo coincide con `^BD\.\s*\d{2}\.\d{2}\.\d{4}\.\.xlsx$`.
  - `test_cabecera_base_en_a10`: la fila 10 es igual a los encabezados de `COLUMNAS_BASE_PERSONAL`.
  - `test_filas_base`: las filas de datos son `len(pob.en_base_personal())`, y las que tienen `TERMINO DE CONVENIO` son 60.
  - `test_sin_base_ausentes`: ninguno de los 8 documentos con `en_base_personal == False` aparece en la base.
  - `test_licencias_hojas_y_cabecera`: existen las dos hojas y la cabecera está en la fila 2, empezando por `DNI/CEX`.
  - `test_licencias_cuadran_con_registros`: las filas de `LICENCIA CON GOCE` son tantas como los registros con `dias_licencia_con_goce > 0`; lo mismo para sin goce.

- [ ] **Paso 2: correr los tests y comprobar que fallan.** Esperado: FAIL.

- [ ] **Paso 3: implementar los dos writers.** Las fechas se escriben como celdas de fecha y `Fecha de Termino` queda vacía para `PLAZO INDETERMINADO`.

- [ ] **Paso 4: correr los tests y ruff.** Esperado: PASS y 0 errores.

---

### Tarea 5: Manifiesto, CLI `generar-demo` (modo `limpio`) y documentación

**Archivos:**
- Crear: `demo/src/generador_demo/manifiesto.py`, `demo/src/generador_demo/cli.py`, `demo/README.md`
- Modificar: `AGENTS.md` §6 (comandos)
- Test: `demo/tests/test_manifiesto_cli.py`

**Interfaces:**
- Usa: todo lo anterior.
- Produce:
  - `construir_manifiesto(pob, registros, p, modo: str) -> dict` con esta forma:

```json
{"semilla": 42, "modo": "limpio", "periodos": {"2025-11": {
  "personas_planilla": 0, "entradas": 0, "salidas": 0, "personas_sin_base": 0,
  "dias_ausencia": {"dias_descanso_medico": 0, "...": 0},
  "horas_extra": {"horas_extra_25": 0.0, "...": 0.0}}},
 "base_personal": {"fecha_corte": "2025-11-12", "empleados": 0, "practicantes": 0,
  "casos_contrato_plazo": 18, "casos_jubilacion": 13}}
```

    `entradas` y `salidas` cuentan personas no practicantes con la fecha dentro del periodo (IND-02, IND-03).
  - `generar(p: ParametrosGeneracion, salida: Path, modo: str) -> Path`: limpia y recrea `salida / modo` (Foco de revisión 4) y escribe:
    - `planilla/*.xlsx`;
    - `planilla/licencias/CONTROL DE LICENCIAS.xlsx` (layout que espera el legacy, NOM-BRZ-02);
    - `base_personal/BD. ….xlsx`;
    - `manifiesto.json` (UTF-8, `indent=2`, `sort_keys=True`).

    Devuelve `salida / modo`.
  - `main(argv: list[str] | None = None) -> int`: opciones `--semilla`, `--periodo-fin`, `--meses`, `--salida` (por defecto `demo/generated`) y `--modo {limpio,sucio,casos}`. Si `--periodo-fin` no cumple `^\d{4}-(0[1-9]|1[0-2])$`, imprime `Periodo inválido: <valor>. Usa el formato AAAA-MM.` y devuelve 2.

- [ ] **Paso 1: escribir los tests que fallan:**
  - `test_manifiesto_cuadra_con_excel`: con `generar(..., tmp_path, "limpio")`, para cada periodo, las filas de datos de la planilla releída son iguales a `personas_planilla`, y la suma de la columna `DIAS GOCE HABER(D)` es igual a `dias_ausencia["dias_licencia_con_goce"]`.
  - `test_estructura_de_salida`: existen los 6 archivos de planilla, el de licencias, la base de personal y el manifiesto.
  - `test_segunda_corrida_limpia`: corre con `meses=6` y luego con `meses=3` sobre la misma salida; solo quedan 3 archivos de planilla.
  - `test_periodo_invalido`: `main(["--periodo-fin", "2025-13"]) == 2`.
  - `test_main_ok`: `main(["--salida", str(tmp_path)]) == 0`.

- [ ] **Paso 2: correr los tests y comprobar que fallan.** Esperado: FAIL.

- [ ] **Paso 3: implementar `manifiesto.py` y `cli.py`.** Por ahora, `--modo sucio` y `--modo casos` imprimen `Modo no disponible todavía` y devuelven 2; la Tarea 6 los reemplaza.

- [ ] **Paso 4: escribir `demo/README.md`** (para qué sirve, comandos, estructura de salida, qué es el manifiesto). En `AGENTS.md` §6, reemplazar las filas pendientes de demo por:
  - `uv run --project demo generar-demo --semilla 42 --modo limpio`
  - `uv run --project demo pytest demo/tests`

- [ ] **Paso 5: correr toda la suite, ruff y una corrida real.** Comandos:
  - `uv run --project demo pytest demo/tests -v`
  - `uv run --project demo ruff check demo`
  - `uv run --project demo generar-demo`

  Esperado: PASS, 0 errores y `demo/generated/limpio/` con la estructura descrita.

---

### Tarea 6: Modo `sucio` y modo `casos`

**Archivos:**
- Crear: `demo/src/generador_demo/suciedad.py`, `demo/src/generador_demo/casos.py`
- Modificar: `demo/src/generador_demo/cli.py` (conectar los modos)
- Test: `demo/tests/test_sucio_casos.py`

**Interfaces:**
- Usa: `generar`, los writers y `guardar_libro`.
- Produce:
  - `INYECTORES: dict[str, Callable[[Path, random.Random], None]]`: cada uno modifica en sitio un archivo ya escrito **sin cambiar los conteos del manifiesto**.
    - `SUC-01`: en la planilla, `DNI/CEX` como número `float` en ~5 % de las filas (TR-55).
    - `SUC-02`: en la planilla, dobles espacios internos y espacios al borde en 3 encabezados (NOM-BRZ-07).
    - `SUC-03`: en la base de personal, fechas como texto `DD/MM/YYYY` en ~10 % de las filas.
    - `SUC-04`: en la planilla, `#N/D` en `CENTRO DE COSTOS` en ~2 % de las filas.
    - `SUC-05`: en la planilla, una fila totalmente vacía intercalada a mitad de los datos (NOM-BRZ-09a).
    - `SUC-06`: en licencias, `DNI/CEX` como texto con sufijo `.0` (LIC-SLV-02).
  - `generar_casos(p, carpeta: Path) -> list[Path]`: una subcarpeta por caso, cada una con un único archivo roto y un `esperado.json` (`{"caso": id, "regla": "...", "hoja": "...", "celda": "..."}`):
    - `CASO-01`: cabecera de la planilla en A7 (fila vacía insertada; TR-D13);
    - `CASO-02`: la hoja de la planilla se llama `Hoja1`;
    - `CASO-03`: falta la columna `DNI/CEX` en la planilla;
    - `CASO-04`: planilla con nombre sin periodo (`Planilla Empleados.xlsx`);
    - `CASO-05`: base de personal con nombre `Base de Datos 12.11.2025.xlsx`;
    - `CASO-06`: licencias sin la hoja `LICENCIA SIN GOCE`.
  - `generar(..., modo="sucio")`: genera como `limpio`, aplica todos los `INYECTORES` y escribe el manifiesto con `"modo": "sucio"` y los mismos conteos. `modo="casos"` llama a `generar_casos`.

- [ ] **Paso 1: escribir los tests que fallan:**
  - `test_cada_inyector_deja_huella`: un test parametrizado por id que comprueba la marca de cada inyector (por ejemplo, para `SUC-01`, existe alguna celda `DNI/CEX` de tipo `float`).
  - `test_sucio_mantiene_conteos`: los manifiestos de `sucio` y `limpio` con la misma semilla son iguales salvo en la clave `modo` (Foco de revisión 5).
  - `test_casos_completos`: hay 6 subcarpetas y cada `esperado.json` tiene las 4 claves.
  - `test_caso_01_cabecera_en_a7`: A6 está vacía y A7 = `"CÓDIGO CORPORATIVO"`.
  - `test_caso_03_sin_dni`: `"DNI/CEX"` no aparece en la fila 6.

- [ ] **Paso 2: correr los tests y comprobar que fallan.** Esperado: FAIL.

- [ ] **Paso 3: implementar `suciedad.py`, `casos.py` y conectar `cli.py`.** Cada inyector usa el `random.Random` que recibe; ninguno crea uno propio.

- [ ] **Paso 4: correr toda la suite, ruff y las tres corridas.** Comandos:
  - `uv run --project demo pytest demo/tests -v`
  - `uv run --project demo ruff check demo`
  - `uv run --project demo generar-demo --modo sucio`
  - `uv run --project demo generar-demo --modo casos`

  Esperado: PASS, 0 errores y las carpetas `sucio/` y `casos/` creadas. Agregar `generar-demo --modo sucio|casos` a `demo/README.md`.
