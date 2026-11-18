"""Punto de entrada `generar-demo`."""

import argparse
import re
import shutil
import sys
from pathlib import Path

from generador_demo.casos import generar_casos
from generador_demo.excel_altas import escribir_altas
from generador_demo.excel_base_personal import escribir_base_personal
from generador_demo.excel_examenes import escribir_examenes
from generador_demo.excel_licencias import escribir_licencias
from generador_demo.excel_planilla import escribir_planilla
from generador_demo.excel_practicantes import escribir_practicantes
from generador_demo.excel_regimen_especial import escribir_regimen_especial
from generador_demo.manifiesto import construir_manifiesto, escribir_manifiesto
from generador_demo.parametros import ParametrosGeneracion
from generador_demo.poblacion import simular_poblacion
from generador_demo.registros import generar_registros
from generador_demo.suciedad import aplicar_suciedad

SALIDA_POR_DEFECTO = Path(__file__).resolve().parents[2] / "generated"
MODOS = ("limpio", "sucio", "casos")
_PERIODO = re.compile(r"^\d{4}-(0[1-9]|1[0-2])$")
MESES_MAXIMOS = 24  # con más meses, las antigüedades base no caben antes del primer periodo
MARCADOR = ".generar-demo"


def _preparar_destino(destino: Path) -> None:
    """Vacía `destino` solo si lo creó este generador o está vacío.

    El marcador se escribe apenas se crea la carpeta, para que una corrida que se corte a
    medias no deje una carpeta que las siguientes corridas se nieguen a limpiar.
    """
    if destino.exists():
        generado = any(
            (destino / nombre).exists() for nombre in (MARCADOR, "manifiesto.json", "casos.json")
        )
        if not generado and any(destino.iterdir()):
            raise FileExistsError(
                f"La carpeta {destino} ya existe y no fue creada por generar-demo; no se toca."
            )
        shutil.rmtree(destino)
    destino.mkdir(parents=True)
    (destino / MARCADOR).write_text("Carpeta creada por generar-demo.\n", encoding="utf-8")


def generar(p: ParametrosGeneracion, salida: Path, modo: str) -> Path:
    if modo not in MODOS:
        raise ValueError(f"Modo desconocido: {modo}")
    destino = Path(salida) / modo
    _preparar_destino(destino)
    if modo == "casos":
        generar_casos(p, destino)
        return destino

    pob = simular_poblacion(p)
    registros = generar_registros(pob, p)
    carpeta_planilla = destino / "planilla"
    for periodo in pob.periodos:
        escribir_planilla(pob, registros, p, periodo, carpeta_planilla)
    escribir_licencias(pob, registros, carpeta_planilla / "licencias")
    escribir_base_personal(pob, destino / "base_personal")
    # Plan 8: dominios restantes (aplicar_suciedad no los toca).
    for periodo in pob.periodos:
        escribir_regimen_especial(pob, p, periodo, destino / "planilla_regimen_especial")
        escribir_altas(pob, periodo, destino / "altas_personal")
    escribir_practicantes(pob, p, destino / "practicantes")
    escribir_examenes(pob, destino / "examenes_salida")
    if modo == "sucio":
        aplicar_suciedad(destino, p.semilla)
    escribir_manifiesto(construir_manifiesto(pob, registros, p, modo), destino)
    return destino


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        prog="generar-demo",
        description="Genera Excel sintéticos con el layout legacy para la demo y los tests.",
    )
    parser.add_argument("--semilla", type=int, default=42)
    parser.add_argument("--periodo-fin", default="2025-11", help="Último periodo, AAAA-MM")
    parser.add_argument("--meses", type=int, default=6)
    parser.add_argument("--salida", type=Path, default=SALIDA_POR_DEFECTO)
    parser.add_argument("--modo", choices=MODOS, default="limpio")
    args = parser.parse_args(argv)

    if not _PERIODO.match(args.periodo_fin):
        print(f"Periodo inválido: {args.periodo_fin}. Usa el formato AAAA-MM.")
        return 2
    if not 1 <= args.meses <= MESES_MAXIMOS:
        print(f"Cantidad de meses inválida: {args.meses}. Debe estar entre 1 y {MESES_MAXIMOS}.")
        return 2
    p = ParametrosGeneracion(semilla=args.semilla, periodo_fin=args.periodo_fin, meses=args.meses)
    try:
        destino = generar(p, args.salida, args.modo)
    except FileExistsError as error:
        print(error)
        return 1
    except OSError as error:
        print(
            "No se pudo escribir en la carpeta de salida. Cierra los archivos generados que "
            f"tengas abiertos en Excel y vuelve a intentarlo. Detalle: {error}"
        )
        return 1
    print(f"Datos generados en {destino}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
