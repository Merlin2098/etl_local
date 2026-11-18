# Plan 2 — Paridad con el legacy: plan de implementación

> **Para agentes:** SUB-SKILL REQUERIDA: usar superpowers:subagent-driven-development (recomendado) o
> superpowers:executing-plans para implementar este plan tarea por tarea. Los pasos usan casillas (`- [ ]`).

**Objetivo:** un comando `generar-golden` en `demo/` que ejecuta el legacy (de solo lectura) sobre la data
demo y guarda sus salidas como *fixtures* versionables, con nombres canónicos y sin el nombre del cliente.
Esas salidas son la referencia de paridad para el motor en Rust.

**Arquitectura:**
- El legacy corre en un **subproceso** de Python, con el directorio de trabajo en una carpeta temporal. Así sus
  `sys.path`, `chdir` y logs no contaminan el proceso que lo llama ni escriben en su carpeta.
- **Planilla y licencias:** se invoca `PipelineNominaExecutor`, lo que la app ejecutaba tras el preflight.
- **Base de personal:** se invoca `BDWorker.execute_etl()`, preflight incluido.
- El nombre de hoja que exige el contrato del legacy se **lee en tiempo de ejecución** y se aplica solo a la
  copia temporal; nunca se escribe en el repositorio.
- Las salidas se copian a `demo/golden/fixtures/<modo>/` con nombres canónicos, junto con un `resumen.json`.

**Stack:** Python **3.12** (uv), dependencias del legacy fijadas a su `uv.lock`: `polars==1.40.1`,
`duckdb==1.5.2`, `openpyxl==3.1.5`, `pyyaml==6.0.3`, `pyside6==6.11.0`, `fastexcel==0.20.2`,
`xlsxwriter==3.2.9`, `pyarrow==23.0.1`.

**Spec:** [ADR-0002](../../adr/0002-demo-python-uv.md) (paridad); [nomina.md](../../logica-negocio/nomina.md) §11
(pipeline compuesto); [bd.md](../../logica-negocio/bd.md); [licencias.md](../../logica-negocio/licencias.md);
[validacion-preflight.md](../../logica-negocio/validacion-preflight.md); [AGENTS.md](../../../AGENTS.md) §3.

## Hechos verificados con una prueba descartable (2026-11-16)

- `duckdb==1.5.2` **no publica binarios para Python 3.15**; con 3.12 todo se instala con binarios precompilados.
- El executor y `BDWorker` corren **sin QApplication** y terminan con `success=True` sobre `limpio`.
- El legacy imprime emojis y flechas, y con stdout redirigido en Windows falla por `UnicodeEncodeError` (cp1252). Se necesita `PYTHONIOENCODING=utf-8`.
- El legacy escribe logs en `logs/`, relativo al directorio de trabajo, y al importar crea `__pycache__` en su carpeta. Se necesita `cwd` = carpeta temporal y `PYTHONDONTWRITEBYTECODE=1`.
- Sobre `limpio`, el legacy coincide con el manifiesto en filas de planilla por periodo (7.378), días de licencia con goce, empleados (1.230), practicantes (60) y alertas de contrato (11 y 7). **No coincide** en las alertas de edad, porque usa `CURRENT_DATE` (TR-57).

## Restricciones globales

- **Sin git** (decisión del usuario): no hay pasos de commit.
- **Legacy de solo lectura** (AGENTS.md §3.2): ningún archivo de `HR_Data_Integration_Pipeline/` se crea ni se modifica.
- **Confidencialidad** (AGENTS.md §3.1): el nombre del cliente nunca se escribe en código, tests, fixtures ni JSON. Para detectarlo y ocultarlo se lee en tiempo de ejecución de dos fuentes: la hoja requerida en `assets/validate_source/bd.yaml` y el segmento entre `Planilla_` y `_Consolidado` en los nombres de salida. Se reemplaza por `<CLIENTE>`.
- **Python:** `demo/.python-version` = `3.12`; el grupo de dependencias `paridad` lleva las versiones exactas indicadas arriba.
- **Legacy ausente:** en un clon del repo open source el legacy no existe. La ruta se toma de `RUTA_LEGACY` o, por defecto, de `<repo>/HR_Data_Integration_Pipeline`. Si no está, los tests de paridad se **omiten** con un motivo claro y el comando termina con código 2 y un mensaje.
- **Idioma e identificadores:** como en el Plan 1 (español, ASCII).

## Foco de revisión

1. **Legacy intacto:** después de una corrida completa no aparece ni cambia ningún archivo en su carpeta (`__pycache__`, `logs/`). Test en la Tarea 3.
2. **Fuga del nombre del cliente** en nombres de archivo, en JSON (mensajes de error del preflight) o en el contenido de los parquet. Test en la Tarea 4.
3. **Legacy ausente:** el comando y los tests degradan con un mensaje claro. Test en la Tarea 2.
4. **Segunda corrida:** reemplaza los fixtures anteriores sin dejar restos y sin borrar carpetas ajenas. Test en la Tarea 4.
5. **Fecha de ejecución:** queda registrada, porque las alertas de edad del legacy dependen de ella. Test en la Tarea 4.

---

### Tarea 1: Fijar Python 3.12 y dependencias de paridad

**Archivos:** crear `demo/.python-version`; modificar `demo/pyproject.toml` (grupo `paridad`, paquete `src/paridad_legacy`).

- [ ] **Paso 1:** crear `demo/.python-version` con `3.12` y agregar el grupo `paridad` con las 8 versiones exactas. Agregar `src/paridad_legacy` a `packages` del wheel y el script `generar-golden = "paridad_legacy.cli:main"`.
- [ ] **Paso 2:** `uv sync --project demo --group paridad`. Esperado: `.venv` recreado con 3.12.x.
- [ ] **Paso 3:** `uv run --project demo python --version` (esperado `Python 3.12.x`) y `uv run --project demo pytest demo/tests -q` (esperado: 67 passed, la suite del Plan 1 sigue verde en 3.12).

### Tarea 2: Entorno del legacy (`paridad_legacy/entorno.py`)

**Archivos:** crear `demo/src/paridad_legacy/__init__.py`, `entorno.py`; test `demo/tests/test_paridad_entorno.py`.

**Interfaces que produce:**
- `ubicar_legacy(ruta: Path | None = None) -> Path`: usa `ruta`, luego `RUTA_LEGACY` y luego `<repo>/HR_Data_Integration_Pipeline`. Exige que exista `etl_manager.py`; si no, lanza `LegacyNoDisponible` (subclase de `RuntimeError`) con el mensaje `No se encontró el proyecto legacy en <ruta>. Define RUTA_LEGACY o coloca el proyecto en <ruta por defecto>.`
- `hoja_requerida_base(legacy: Path) -> str`: primer valor de `sheet_rules.required_sheets` en `assets/validate_source/bd.yaml`.
- `nombres_a_ocultar(legacy: Path, rutas: Iterable[Path] = ()) -> set[str]`: la hoja requerida más los segmentos `Planilla_(.+?)_Consolidado` encontrados en `rutas`.
- `ocultar(texto: str, nombres: set[str]) -> str`: reemplaza cada nombre, sin distinguir mayúsculas, por `<CLIENTE>`.
- `entorno_subproceso() -> dict[str, str]`: copia de `os.environ` con `PYTHONIOENCODING=utf-8`, `PYTHONUTF8=1` y `PYTHONDONTWRITEBYTECODE=1`.

- [ ] **Paso 1: tests que fallan** (sin legacy real; usan un legacy falso en `tmp_path` con `etl_manager.py` y un `bd.yaml` con `required_sheets: [ACME]`):
  - `test_ubicar_legacy_por_argumento_y_variable`
  - `test_ubicar_legacy_ausente_lanza_mensaje`: el mensaje contiene `RUTA_LEGACY`.
  - `test_hoja_requerida_base`: devuelve `"ACME"`.
  - `test_nombres_a_ocultar_desde_rutas`: `Planilla_ACME_Consolidado.parquet` → `{"ACME"}`.
  - `test_ocultar_ignora_mayusculas`: `"hoja 'Acme' falta"` → `"hoja '<CLIENTE>' falta"`.
  - `test_entorno_subproceso`: contiene las 3 variables.
- [ ] **Paso 2:** correr y comprobar el FAIL. **Paso 3:** implementar. **Paso 4:** PASS y ruff limpio.

### Tarea 3: Ejecución del legacy en subproceso (`paridad_legacy/ejecutar.py`)

**Archivos:** crear `demo/src/paridad_legacy/ejecutar.py` (incluye el punto de entrada del subproceso); test `demo/tests/test_paridad_ejecutar.py`.

**Interfaces:**
- Usa: `ubicar_legacy`, `hoja_requerida_base`, `entorno_subproceso`.
- Produce:
  - `SALIDAS_LEGACY: dict[str, str]`: nombre canónico → patrón glob relativo a la carpeta de trabajo:

    | Nombre canónico | Patrón |
    |---|---|
    | `planilla_silver` | `planilla/silver/Planilla_*_Consolidado_Silver.parquet` |
    | `licencias_silver` | `planilla/silver/licencias_consolidadas.parquet` |
    | `planilla_gold` | `planilla/gold/actual/Planilla_*_Consolidado.parquet` |
    | `planilla_bi_con_licencias` | `planilla/gold/actual/Planilla_*_BI_Gold_Con_Licencias.parquet` |
    | `base_personal_silver` | `base_personal/silver/bd_silver.parquet` |
    | `centros_costo` | `base_personal/centros_costo/actual/CC_ACTUAL.parquet` |
    | `base_personal_empleados_gold` | `base_personal/gold/bd_empleados_gold.parquet` |
    | `base_personal_practicantes_gold` | `base_personal/gold/bd_practicantes_gold.parquet` |
    | `base_personal_empleados_flags` | `base_personal/gold/bd_empleados_flags_gold.parquet` |
  - `ResultadoLegacy`: dataclass con `dominio: str`, `exito: bool`, `error: str | None` y `salidas: dict[str, Path]`.
  - `correr_legacy(legacy: Path, datos_modo: Path, trabajo: Path) -> list[ResultadoLegacy]`:
    1. Copia `datos_modo/planilla` y `datos_modo/base_personal` a `trabajo`.
    2. Renombra la hoja activa de la base copiada a `hoja_requerida_base(legacy)`.
    3. Lanza `sys.executable -m paridad_legacy.ejecutar <legacy> <trabajo> <dominio>` con `cwd=trabajo`, `env=entorno_subproceso()` y `timeout=900`, una vez por dominio (`planilla`, `base_personal`).
    4. Lee el JSON que el subproceso imprime en su última línea (`{"exito":…, "error":…}`). La salida completa se guarda en `trabajo/<dominio>.log`.
  - `localizar_salidas(trabajo: Path) -> dict[str, Path]`: aplica `SALIDAS_LEGACY`; los patrones sin coincidencia se omiten.

- [ ] **Paso 1: tests que fallan.** Se marcan `@pytest.mark.paridad` y se omiten si `ubicar_legacy()` lanza `LegacyNoDisponible`. Comparten un fixture de módulo que corre `correr_legacy` sobre `generar(ParametrosGeneracion(meses=3), tmp, "limpio")`.
  - `test_ambos_dominios_exitosos`
  - `test_salidas_completas`: las 9 claves de `SALIDAS_LEGACY` están presentes.
  - `test_legacy_intacto`: se toma una instantánea `{ruta: (tamaño, mtime_ns)}` de todo el legacy, salvo `.git/`, antes y después del fixture, y deben ser iguales.
  - `test_logs_en_trabajo`: existen `trabajo/planilla.log` y `trabajo/logs/`.
- [ ] **Paso 2:** FAIL. **Paso 3:** implementar. **Paso 4:** PASS y ruff limpio.

### Tarea 4: Fixtures golden y comando `generar-golden`

**Archivos:** crear `demo/src/paridad_legacy/golden.py`, `demo/src/paridad_legacy/cli.py`; test `demo/tests/test_paridad_golden.py`; modificar `demo/README.md`, `AGENTS.md` §6, `docs/superpowers/plans/2026-11-16-hoja-de-ruta.md`.

**Interfaces:**
- Usa: `generar` (Plan 1), `correr_legacy`, `localizar_salidas`, `nombres_a_ocultar`, `ocultar`.
- Produce:
  - `generar_golden(legacy: Path, destino: Path, modo: str, p: ParametrosGeneracion) -> Path`:
    - genera la data del `modo` en una carpeta temporal;
    - corre el legacy;
    - prepara `destino/<modo>` con el mismo criterio de marcador que el Plan 1 (`.generar-golden`), que solo borra lo que el propio comando creó;
    - copia cada salida como `<nombre canónico>.parquet`;
    - escribe `resumen.json`.
  - `resumen.json` (claves ordenadas, UTF-8, todo texto pasado por `ocultar`):
    - `modo`, `semilla`;
    - `fecha_ejecucion` (ISO, fecha local);
    - `versiones` (`polars`, `duckdb`, `python`);
    - `dominios`: `[{dominio, exito, error}]`;
    - `archivos`: `{nombre: {filas, columnas}}`;
    - `comparacion_manifiesto`: por indicador, `{esperado, legacy, coincide}` para:
      - filas de planilla por periodo;
      - `dias_licencia_con_goce` total;
      - empleados y practicantes;
      - alertas de contrato (obra / incremento);
      - alertas de edad.
  - Modo `casos`: en vez de correr los pipelines, ejecuta en subproceso `validate_all_sources_for_etl(etl, archivo)` del legacy para cada caso y escribe `destino/casos/<CASO>.json` con `{caso, etl, errores: [..]}`. El `etl` es `nomina` para CASO-01…04, `bd` para CASO-05 y `licencias` para CASO-06.
  - `main(argv) -> int`: opciones `--modo {limpio,sucio,casos}` (por defecto `limpio`), `--salida` (por defecto `<demo>/golden/fixtures`) y `--legacy`. Con `LegacyNoDisponible` imprime el mensaje y devuelve 2.

- [ ] **Paso 1: tests que fallan** (`@pytest.mark.paridad`, fixture de módulo con `meses=3`):
  - `test_limpio_coincide_con_manifiesto`: en `comparacion_manifiesto`, `coincide` es verdadero para filas por periodo, días con goce, empleados, practicantes y alertas de contrato.
  - `test_alertas_de_edad_registran_fecha`: existe `fecha_ejecucion`, y la comparación de edad está presente, coincida o no.
  - `test_sin_nombre_del_cliente`: ninguno de `nombres_a_ocultar(legacy)` aparece, sin distinguir mayúsculas, en nombres de archivo, en el texto de `resumen.json` ni en los valores de texto de los parquet (leídos con polars).
  - `test_casos_registran_errores`: CASO-01, 02, 03, 05 y 06 tienen `errores` no vacío. **CASO-04 tiene `errores == []`**: el contrato de nómina del legacy no valida el nombre de archivo (NOM-BRZ-05). Es una brecha que el motor nuevo sí debe cubrir.
  - `test_segunda_corrida_reemplaza`
  - `test_legacy_ausente`: `main(["--legacy", str(tmp_path)]) == 2`.
- [ ] **Paso 2:** FAIL. **Paso 3:** implementar.
- [ ] **Paso 4:** PASS. Correr `uv run --project demo generar-golden --modo limpio`, luego `--modo sucio` y `--modo casos`. Esperado: `demo/golden/fixtures/{limpio,sucio,casos}/` creados.
- [ ] **Paso 5: documentación.**
  - `demo/README.md`: sección "Paridad con el legacy" (requisitos, comandos, qué contiene `resumen.json` y por qué las alertas de edad no coinciden).
  - `AGENTS.md` §6: reemplazar "Generar golden de paridad (pendiente)" por `uv sync --project demo --group paridad` y `uv run --project demo generar-golden --modo limpio`.
  - Hoja de ruta: Plan 2 completado.
