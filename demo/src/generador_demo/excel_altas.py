"""Writer de las altas de personal (layout de la relación de ingresos, PDT §2, Plan 8).

Dos hojas con cabecera en la fila 1. Cada hoja usa sus propios encabezados, como el legacy.
"""

from pathlib import Path

from openpyxl import Workbook

from generador_demo.dominios import altas_del_mes, cargo_de, nombre_completo, universidad_de
from generador_demo.excel_comun import FORMATO_FECHA, guardar_libro
from generador_demo.poblacion import Poblacion

EMPLEADOS = [
    "DNI",
    "APELLIDOS Y NOMBRES",
    "SEXO",
    "FECHA NACIMIENTO",
    "CARGO",
    "AREA",
    "SEDE",
    "FECHA INICIO",
    "TIPO DE CONTRATO",
    "PROYECTO",
    "CODIGO SAP",
]
PRACTICANTES = [
    "Numero Documento",
    "Apellidos y Nombres",
    "Sexo",
    "Fecha Nacimiento",
    "Cargo",
    "Area",
    "Sede",
    "Fecha Inicio",
    "Universidad de Procedencia",
]


def nombre_archivo(periodo: str) -> str:
    return f"Altas de personal {periodo}.xlsx"


def _escribir(hoja, columnas: list[str], filas: list[list[object]]) -> None:
    for j, nombre in enumerate(columnas, start=1):
        hoja.cell(row=1, column=j, value=nombre)
    for i, valores in enumerate(filas, start=2):
        for j, valor in enumerate(valores, start=1):
            celda = hoja.cell(row=i, column=j, value=valor)
            if j in (4, 8):
                celda.number_format = FORMATO_FECHA


def escribir_altas(pob: Poblacion, periodo: str, carpeta: Path) -> Path:
    empleados, practicantes = altas_del_mes(pob, periodo)
    wb = Workbook()
    wb.remove(wb.active)
    _escribir(
        wb.create_sheet("EMPLEADOS"),
        EMPLEADOS,
        [
            [
                x.numero_documento,
                nombre_completo(x),
                x.sexo,
                x.fecha_nacimiento,
                cargo_de(x),
                x.area,
                x.sede,
                x.fecha_entrada,
                x.modalidad_contrato,
                "Staff",
                f"SAP{x.codigo_empleado[1:]}",
            ]
            for x in empleados
        ],
    )
    _escribir(
        wb.create_sheet("PRACTICANTES"),
        PRACTICANTES,
        [
            [
                x.numero_documento,
                nombre_completo(x),
                x.sexo,
                x.fecha_nacimiento,
                "PRACTICANTE",
                x.area,
                x.sede,
                x.fecha_entrada,
                universidad_de(x),
            ]
            for x in practicantes
        ],
    )
    return guardar_libro(wb, carpeta / nombre_archivo(periodo))
