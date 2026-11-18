"""Ubicación del proyecto legacy, ocultamiento del nombre del cliente y entorno del subproceso.

El nombre del cliente nunca se escribe en este repositorio: se lee en tiempo de ejecución
desde el propio legacy (contrato de la base de personal y nombres de sus archivos de salida).
"""

import importlib.util
import os
import re
from collections.abc import Iterable
from pathlib import Path

REPO = Path(__file__).resolve().parents[3]
LEGACY_POR_DEFECTO = REPO / "HR_Data_Integration_Pipeline"
MARCADOR_LEGACY = "etl_manager.py"
CONTRATO_BASE = Path("assets") / "validate_source" / "bd.yaml"
_SEGMENTO_SALIDA = re.compile(r"Planilla_(.+?)_Consolidado")


class LegacyNoDisponible(RuntimeError):
    """El proyecto legacy no está en la ruta indicada (p. ej. en un clon open source)."""


def ubicar_legacy(ruta: Path | None = None) -> Path:
    candidata = Path(ruta or os.environ.get("RUTA_LEGACY") or LEGACY_POR_DEFECTO)
    if not (candidata / MARCADOR_LEGACY).is_file():
        raise LegacyNoDisponible(
            f"No se encontró el proyecto legacy en {candidata}. Define RUTA_LEGACY o coloca "
            f"el proyecto en {LEGACY_POR_DEFECTO}."
        )
    return candidata


DEPENDENCIAS_PARIDAD = ("yaml", "polars", "duckdb", "PySide6", "openpyxl")


def paridad_disponible(
    legacy: Path | None = None, verificar_legacy: bool = True
) -> tuple[bool, str]:
    """¿Se puede correr la paridad aquí? Devuelve (disponible, motivo si no lo está)."""
    faltantes = [m for m in DEPENDENCIAS_PARIDAD if importlib.util.find_spec(m) is None]
    if faltantes:
        return False, (
            f"Faltan dependencias de paridad ({', '.join(faltantes)}). "
            "Instálalas con: uv sync --project demo --group paridad"
        )
    if verificar_legacy:
        try:
            ubicar_legacy(legacy)
        except LegacyNoDisponible as error:
            return False, str(error)
    return True, ""


def hoja_requerida_base(legacy: Path) -> str:
    import yaml  # dependencia del grupo `paridad`

    contrato = yaml.safe_load((legacy / CONTRATO_BASE).read_text(encoding="utf-8"))
    return str(contrato["sheet_rules"]["required_sheets"][0])


def nombres_a_ocultar(legacy: Path, rutas: Iterable[Path] = ()) -> set[str]:
    nombres = {hoja_requerida_base(legacy)}
    for ruta in rutas:
        coincidencia = _SEGMENTO_SALIDA.search(Path(ruta).name)
        if coincidencia:
            nombres.add(coincidencia.group(1))
    return {n for n in nombres if n.strip()}


def ocultar(texto: str, nombres: set[str]) -> str:
    for nombre in sorted(nombres, key=len, reverse=True):
        texto = re.sub(re.escape(nombre), "<CLIENTE>", texto, flags=re.IGNORECASE)
    return texto


def entorno_subproceso() -> dict[str, str]:
    """Variables para correr el legacy: UTF-8 (imprime emojis) y sin escribir bytecode."""
    env = dict(os.environ)
    env.update({"PYTHONIOENCODING": "utf-8", "PYTHONUTF8": "1", "PYTHONDONTWRITEBYTECODE": "1"})
    return env
