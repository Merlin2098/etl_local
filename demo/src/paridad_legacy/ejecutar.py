"""Corre el legacy en un subproceso, con su directorio de trabajo en una carpeta temporal.

Planilla + licencias: `PipelineNominaExecutor` (lo que la app ejecutaba tras el preflight).
Base de personal: `BDWorker.execute_etl()`, preflight incluido. Ninguno requiere QApplication.

Uso interno del subproceso: `python -m paridad_legacy.ejecutar <legacy> <trabajo> <dominio>`;
con `preflight:<etl>` como dominio, el segundo argumento es el archivo a validar.
"""

import json
import shutil
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path

from paridad_legacy.entorno import entorno_subproceso, hoja_requerida_base

DOMINIOS = ("planilla", "base_personal")
TIEMPO_MAXIMO_SEGUNDOS = 900

SALIDAS_LEGACY: dict[str, str] = {
    "planilla_silver": "planilla/silver/Planilla_*_Consolidado_Silver.parquet",
    "licencias_silver": "planilla/silver/licencias_consolidadas.parquet",
    "planilla_gold": "planilla/gold/actual/Planilla_*_Consolidado.parquet",
    "planilla_bi_con_licencias": "planilla/gold/actual/Planilla_*_BI_Gold_Con_Licencias.parquet",
    "base_personal_silver": "base_personal/silver/bd_silver.parquet",
    "centros_costo": "base_personal/centros_costo/actual/CC_ACTUAL.parquet",
    "base_personal_empleados_gold": "base_personal/gold/bd_empleados_gold.parquet",
    "base_personal_practicantes_gold": "base_personal/gold/bd_practicantes_gold.parquet",
    "base_personal_empleados_flags": "base_personal/gold/bd_empleados_flags_gold.parquet",
}
_DOMINIO_DE_SALIDA = {nombre: patron.split("/")[0] for nombre, patron in SALIDAS_LEGACY.items()}


@dataclass
class ResultadoLegacy:
    dominio: str
    exito: bool
    error: str | None
    salidas: dict[str, Path] = field(default_factory=dict)


def localizar_salidas(trabajo: Path) -> dict[str, Path]:
    encontradas: dict[str, Path] = {}
    for nombre, patron in SALIDAS_LEGACY.items():
        coincidencias = sorted(Path(trabajo).glob(patron))
        if coincidencias:
            encontradas[nombre] = coincidencias[0]
    return encontradas


def aplicar_hoja_requerida(legacy: Path, ruta: Path) -> None:
    """Renombra la hoja activa de una base de personal al nombre que exige el legacy.

    Ese nombre es propio del cliente: se lee en tiempo de ejecución y se aplica solo a
    copias temporales, nunca a archivos del repositorio.
    """
    from openpyxl import load_workbook

    wb = load_workbook(ruta)
    wb.active.title = hoja_requerida_base(legacy)
    wb.save(ruta)


def _preparar_trabajo(legacy: Path, datos_modo: Path, trabajo: Path) -> None:
    trabajo.mkdir(parents=True, exist_ok=True)
    for carpeta in DOMINIOS:
        shutil.copytree(datos_modo / carpeta, trabajo / carpeta, dirs_exist_ok=True)
    for ruta in (trabajo / "base_personal").glob("*.xlsx"):
        aplicar_hoja_requerida(legacy, ruta)


def correr_legacy(legacy: Path, datos_modo: Path, trabajo: Path) -> list[ResultadoLegacy]:
    _preparar_trabajo(legacy, datos_modo, trabajo)
    resultados: list[ResultadoLegacy] = []
    for dominio in DOMINIOS:
        proceso = subprocess.run(
            [sys.executable, "-m", "paridad_legacy.ejecutar", str(legacy), str(trabajo), dominio],
            cwd=trabajo,
            env=entorno_subproceso(),
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=TIEMPO_MAXIMO_SEGUNDOS,
        )
        (trabajo / f"{dominio}.log").write_text(
            proceso.stdout + "\n--- stderr ---\n" + proceso.stderr, encoding="utf-8"
        )
        resultado = _leer_resultado(proceso.stdout)
        if resultado is None:
            resultado = {
                "exito": False,
                "error": f"El subproceso terminó con código {proceso.returncode} sin resultado.",
            }
        salidas = {
            nombre: ruta
            for nombre, ruta in localizar_salidas(trabajo).items()
            if _DOMINIO_DE_SALIDA[nombre] == dominio
        }
        resultados.append(
            ResultadoLegacy(dominio, bool(resultado["exito"]), resultado["error"], salidas)
        )
    return resultados


def validar_con_legacy(legacy: Path, archivo: Path, etl: str, trabajo: Path) -> list[str]:
    """Corre el preflight del legacy (`validate_all_sources_for_etl`) sobre un archivo."""
    trabajo.mkdir(parents=True, exist_ok=True)
    proceso = subprocess.run(
        [
            sys.executable,
            "-m",
            "paridad_legacy.ejecutar",
            str(legacy),
            str(archivo),
            f"preflight:{etl}",
        ],
        cwd=trabajo,
        env=entorno_subproceso(),
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        timeout=TIEMPO_MAXIMO_SEGUNDOS,
    )
    resultado = _leer_resultado(proceso.stdout)
    if resultado is None or not resultado["exito"]:
        detalle = resultado["error"] if resultado else proceso.stderr[-500:]
        raise RuntimeError(f"No se pudo correr el preflight legacy de {archivo.name}: {detalle}")
    return list(resultado["errores"])


def _leer_resultado(stdout: str) -> dict | None:
    for linea in reversed(stdout.strip().splitlines()):
        if linea.startswith("{") and '"exito"' in linea:
            return json.loads(linea)
    return None


# --------------------------------------------------------------------------- subproceso


def _ejecutar_dominio(legacy: Path, trabajo: Path, dominio: str) -> dict:
    sys.path.insert(0, str(legacy))
    if dominio.startswith("preflight:"):
        from src.utils.validate_source import validate_all_sources_for_etl

        reporte = validate_all_sources_for_etl(dominio.split(":", 1)[1], trabajo)
        return {"exito": True, "error": None, "errores": list(reporte.errors)}
    if dominio == "planilla":
        from src.orchestrators.pipeline_nomina_executor import PipelineNominaExecutor

        carpeta = trabajo / "planilla"
        ejecutor = PipelineNominaExecutor(
            yaml_path=legacy / "src/orchestrators/pipelines/pipeline_nomina_licencias.yaml",
            archivos=sorted(carpeta.glob("*.xlsx")),
            output_dir=carpeta,
            export_excel_gold=False,
        )
        resultado = ejecutor.execute()
    elif dominio == "base_personal":
        from src.modules.bd.ui.worker import BDWorker

        carpeta = trabajo / "base_personal"
        worker = BDWorker(sorted(carpeta.glob("*.xlsx")), carpeta, export_excel_gold=False)
        resultado = worker.execute_etl()
    else:
        raise ValueError(f"Dominio desconocido: {dominio}")
    return {"exito": bool(resultado.get("success")), "error": resultado.get("error")}


def _main_subproceso(argv: list[str]) -> int:
    sys.dont_write_bytecode = True
    legacy, trabajo, dominio = Path(argv[0]), Path(argv[1]), argv[2]
    try:
        resultado = _ejecutar_dominio(legacy, trabajo, dominio)
    except Exception as error:  # el resultado se reporta al proceso padre, no se oculta
        resultado = {"exito": False, "error": f"{type(error).__name__}: {error}"}
    print(json.dumps(resultado, ensure_ascii=False, default=str), flush=True)
    return 0 if resultado["exito"] else 1


if __name__ == "__main__":
    sys.exit(_main_subproceso(sys.argv[1:]))
