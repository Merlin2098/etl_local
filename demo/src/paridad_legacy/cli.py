"""Punto de entrada `generar-golden`."""

import argparse
import subprocess
import sys
from pathlib import Path

from generador_demo.cli import MESES_MAXIMOS
from generador_demo.parametros import ParametrosGeneracion
from paridad_legacy.entorno import LegacyNoDisponible, ubicar_legacy

SALIDA_POR_DEFECTO = Path(__file__).resolve().parents[2] / "golden" / "fixtures"


def _meses(texto: str) -> int:
    try:
        meses = int(texto)
    except ValueError:
        raise argparse.ArgumentTypeError(f"«{texto}» no es un número de meses") from None
    if not 1 <= meses <= MESES_MAXIMOS:
        raise argparse.ArgumentTypeError(f"debe estar entre 1 y {MESES_MAXIMOS}")
    return meses


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        prog="generar-golden",
        description="Corre el legacy (solo lectura) sobre la data demo y guarda sus salidas.",
    )
    parser.add_argument("--modo", choices=("limpio", "sucio", "casos"), default="limpio")
    parser.add_argument("--salida", type=Path, default=SALIDA_POR_DEFECTO)
    parser.add_argument("--legacy", type=Path, default=None)
    parser.add_argument("--semilla", type=int, default=42)
    parser.add_argument("--meses", type=_meses, default=6, help="meses de datos demo (1 a 24)")
    args = parser.parse_args(argv)

    try:
        legacy = ubicar_legacy(args.legacy)
    except LegacyNoDisponible as error:
        print(error)
        return 2

    from paridad_legacy.golden import generar_golden

    try:
        destino = generar_golden(
            legacy,
            args.salida,
            args.modo,
            ParametrosGeneracion(semilla=args.semilla, meses=args.meses),
        )
    except FileExistsError as error:
        print(error)
        return 1
    except OSError as error:
        print(f"No se generó el golden: no se pudo escribir en la carpeta de salida. {error}")
        return 1
    except (RuntimeError, subprocess.SubprocessError) as error:
        print(f"No se generó el golden; los fixtures anteriores se conservan. Detalle: {error}")
        return 1
    print(f"Golden generado en {destino}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
