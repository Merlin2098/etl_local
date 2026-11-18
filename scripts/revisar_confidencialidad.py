"""Revisión de confidencialidad antes de publicar (Plan 9).

Busca, en todo lo que se versionaría, el nombre del cliente y direcciones de correo. El nombre se lee
en tiempo de ejecución desde el legacy (`paridad_legacy.entorno`) y **nunca se imprime**: un hallazgo
dice solo el archivo, la línea y el motivo. Dentro de los `.xlsx` se leen sus XML; los demás binarios
se leen como latin-1 y como UTF-16 (así se cubren también los instaladores).

Uso: uv run --project demo python scripts/revisar_confidencialidad.py [--raiz .] [archivos...]
Sale con 0 si no hay hallazgos, 1 si los hay y 2 si no encuentra el legacy.
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
import zipfile
from dataclasses import dataclass
from pathlib import Path
from typing import Literal

# El dominio empieza con letra y termina en un TLD alfabético: así `echarts@6.1.0` o `128x128@2x.png`
# (versiones y recursos) no cuentan como correos.
CORREO = re.compile(r"[\w.+-]+@[A-Za-z][\w-]*(?:\.[\w-]+)*\.[A-Za-z]{2,}\b")
CORREOS_PERMITIDOS = (
    re.compile(r".*@users\.noreply\.github\.com", re.IGNORECASE),
    re.compile(r".*@example\.com", re.IGNORECASE),
    re.compile(r"noreply@anthropic\.com", re.IGNORECASE),
)
LIBROS = {".xlsx", ".xlsm"}
TEXTO_PROBABLE = 0.30  # fracción máxima de bytes de control para tratar un archivo como texto


@dataclass(frozen=True)
class Hallazgo:
    archivo: str
    motivo: Literal["nombre", "correo"]
    linea: int | None


def describir(h: Hallazgo) -> str:
    donde = f"{h.archivo}:{h.linea}" if h.linea is not None else h.archivo
    return f"{donde}: {h.motivo}"


def _textos(ruta: Path) -> list[tuple[str, bool]]:
    """Textos a revisar del archivo y si sus números de línea tienen sentido."""
    if ruta.suffix.lower() in LIBROS:
        with zipfile.ZipFile(ruta) as libro:
            return [
                (libro.read(n).decode("utf-8", errors="replace"), False)
                for n in libro.namelist()
                if n.endswith((".xml", ".rels"))
            ]
    datos = ruta.read_bytes()
    try:
        return [(datos.decode("utf-8"), True)]
    except UnicodeDecodeError:
        pass
    control = sum(b < 9 or 13 < b < 32 for b in datos[:8192])
    if datos and control / min(len(datos), 8192) <= TEXTO_PROBABLE:
        return [(datos.decode("latin-1"), True)]
    return [(datos.decode("latin-1"), False), (datos.decode("utf-16-le", errors="ignore"), False)]


def _correo_permitido(correo: str) -> bool:
    return any(p.fullmatch(correo) for p in CORREOS_PERMITIDOS)


def revisar(raiz: Path, archivos: list[str], nombres: list[str]) -> list[Hallazgo]:
    buscados = [n.lower() for n in nombres if n.strip()]
    hallazgos: list[Hallazgo] = []
    for archivo in archivos:
        ruta = raiz / archivo
        if not ruta.is_file():
            continue
        vistos: set[tuple[str, int | None]] = set()
        for texto, con_lineas in _textos(ruta):
            for numero, linea in enumerate(texto.splitlines(), start=1):
                pos = numero if con_lineas else None
                minuscula = linea.lower()
                if any(n in minuscula for n in buscados):
                    vistos.add(("nombre", pos))
                if any(not _correo_permitido(c) for c in CORREO.findall(linea)):
                    vistos.add(("correo", pos))
        hallazgos += [
            Hallazgo(archivo, motivo, linea)
            for motivo, linea in sorted(vistos, key=lambda v: (v[1] or 0, v[0]))
        ]
    return hallazgos


def versionables(raiz: Path) -> list[str]:
    salida = subprocess.run(
        ["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z"],
        cwd=raiz, capture_output=True, check=True,
    ).stdout.decode("utf-8")
    return sorted({a for a in salida.split("\0") if a})


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--raiz", type=Path, default=Path("."))
    parser.add_argument("archivos", nargs="*", help="por defecto, todo lo que se versionaría")
    args = parser.parse_args(argv)

    from paridad_legacy.entorno import LegacyNoDisponible, nombres_a_ocultar, ubicar_legacy

    try:
        nombres = list(nombres_a_ocultar(ubicar_legacy()))
    except LegacyNoDisponible:
        print("No se encontró el legacy: no se puede revisar el nombre del cliente", file=sys.stderr)
        return 2

    archivos = args.archivos or versionables(args.raiz)
    hallazgos = revisar(args.raiz, archivos, nombres)
    for h in hallazgos:
        print(describir(h))
    print(f"{len(archivos)} archivos revisados, {len(hallazgos)} hallazgos")
    return 1 if hallazgos else 0


if __name__ == "__main__":
    sys.exit(main())
