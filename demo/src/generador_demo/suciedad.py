"""Modo `sucio`: desorden realista que un archivo válido puede traer.

Cada inyector modifica en sitio un archivo ya escrito **sin cambiar los conteos del
manifiesto**: el contenido sigue siendo el mismo, solo cambia su forma. Los IDs se citan
en los tests del motor.
"""

import random
from collections.abc import Callable
from pathlib import Path

from openpyxl import load_workbook
from openpyxl.worksheet.worksheet import Worksheet

from generador_demo.excel_comun import guardar_libro

Inyector = Callable[[Path, random.Random], None]

_FILA_CAB_PLANILLA = 6
_FILA_CAB_BASE = 10


def _columna(hoja: Worksheet, fila_cabecera: int, nombre: str) -> int:
    for celda in hoja[fila_cabecera]:
        if celda.value == nombre:
            return celda.column
    raise KeyError(f"No existe la columna {nombre!r} en la fila {fila_cabecera}")


def _filas_datos(hoja: Worksheet, desde: int) -> list[int]:
    return [
        fila
        for fila in range(desde, hoja.max_row + 1)
        if isinstance(hoja.cell(row=fila, column=1).value, str)
        and not hoja.cell(row=fila, column=1).value.startswith(("+", "-"))
    ]


def _editar(ruta: Path, hoja_nombre: str, cambio: Callable[[Worksheet], None]) -> None:
    wb = load_workbook(ruta)
    cambio(wb[hoja_nombre])
    guardar_libro(wb, ruta)


def suc_01_dni_numerico(ruta: Path, rng: random.Random) -> None:
    """DNI/CEX como número decimal en ~5 % de las filas (TR-55)."""

    def cambio(hoja: Worksheet) -> None:
        col = _columna(hoja, _FILA_CAB_PLANILLA, "DNI/CEX")
        for fila in _filas_datos(hoja, _FILA_CAB_PLANILLA + 1):
            if rng.random() < 0.05:
                celda = hoja.cell(row=fila, column=col)
                celda.value = float(celda.value)

    _editar(ruta, "Planilla", cambio)


def suc_02_espacios_en_encabezados(ruta: Path, rng: random.Random) -> None:
    """Dobles espacios internos y espacios al borde en 3 encabezados (NOM-BRZ-07)."""
    reemplazos = {
        "APELLIDO PATERNO": "APELLIDO  PATERNO",
        "NOMBRES": " NOMBRES ",
        "MOTIVO CESE": "MOTIVO  CESE",
    }

    def cambio(hoja: Worksheet) -> None:
        for celda in hoja[_FILA_CAB_PLANILLA]:
            if celda.value in reemplazos:
                celda.value = reemplazos[celda.value]

    _editar(ruta, "Planilla", cambio)


def suc_03_fechas_como_texto(ruta: Path, rng: random.Random) -> None:
    """Fechas de entrada como texto DD/MM/YYYY en ~10 % de las filas de la base."""

    def cambio(hoja: Worksheet) -> None:
        col = _columna(hoja, _FILA_CAB_BASE, "FECH_INGR.")
        for fila in range(_FILA_CAB_BASE + 1, hoja.max_row + 1):
            celda = hoja.cell(row=fila, column=col)
            if celda.value is not None and rng.random() < 0.10:
                celda.value = celda.value.strftime("%d/%m/%Y")

    wb = load_workbook(ruta)
    cambio(wb[wb.sheetnames[0]])
    guardar_libro(wb, ruta)


def suc_04_nd_en_centro_de_costos(ruta: Path, rng: random.Random) -> None:
    """Celda con error de Excel en CENTRO DE COSTOS en ~2 % de las filas (TR-12).

    Excel en español muestra `#N/D`, pero guarda el error como `#N/A` con tipo error
    (`t="e"`); openpyxl lo escribe así al asignar la cadena `#N/A`.
    """

    def cambio(hoja: Worksheet) -> None:
        col = _columna(hoja, _FILA_CAB_PLANILLA, "CENTRO DE COSTOS")
        filas = _filas_datos(hoja, _FILA_CAB_PLANILLA + 1)
        elegidas = [f for f in filas if rng.random() < 0.02] or filas[:1]
        for fila in elegidas:
            hoja.cell(row=fila, column=col).value = "#N/A"

    _editar(ruta, "Planilla", cambio)


def suc_05_fila_vacia_intercalada(ruta: Path, rng: random.Random) -> None:
    """Una fila totalmente vacía a mitad de los datos (NOM-BRZ-09a)."""

    def cambio(hoja: Worksheet) -> None:
        filas = _filas_datos(hoja, _FILA_CAB_PLANILLA + 1)
        hoja.insert_rows(filas[len(filas) // 2])

    _editar(ruta, "Planilla", cambio)


def suc_06_dni_con_punto_cero(ruta: Path, rng: random.Random) -> None:
    """DNI/CEX como texto con sufijo `.0` en ~50 % de las filas de licencias (LIC-SLV-02)."""
    wb = load_workbook(ruta)
    for hoja in wb.worksheets:
        filas = list(range(3, hoja.max_row + 1))
        elegidas = [f for f in filas if rng.random() < 0.5] or filas[:1]
        for fila in elegidas:
            celda = hoja.cell(row=fila, column=1)
            if celda.value is not None:
                celda.value = f"{celda.value}.0"
    guardar_libro(wb, ruta)


INYECTORES: dict[str, Inyector] = {
    "SUC-01": suc_01_dni_numerico,
    "SUC-02": suc_02_espacios_en_encabezados,
    "SUC-03": suc_03_fechas_como_texto,
    "SUC-04": suc_04_nd_en_centro_de_costos,
    "SUC-05": suc_05_fila_vacia_intercalada,
    "SUC-06": suc_06_dni_con_punto_cero,
}

# Qué archivos de la salida modifica cada inyector (patrón relativo a la carpeta del modo).
OBJETIVOS: dict[str, str] = {
    "SUC-01": "planilla/Planilla *.xlsx",
    "SUC-02": "planilla/Planilla *.xlsx",
    "SUC-03": "base_personal/BD.*.xlsx",
    "SUC-04": "planilla/Planilla *.xlsx",
    "SUC-05": "planilla/Planilla *.xlsx",
    "SUC-06": "planilla/licencias/*.xlsx",
}


def aplicar_suciedad(destino: Path, semilla: int) -> None:
    """Aplica todos los inyectores, en orden de ID, a los archivos de `destino`."""
    rng = random.Random(semilla + 2)
    for id_inyector in sorted(INYECTORES):
        for ruta in sorted(destino.glob(OBJETIVOS[id_inyector])):
            INYECTORES[id_inyector](ruta, rng)
