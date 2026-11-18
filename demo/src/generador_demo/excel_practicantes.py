"""Writer del control de practicantes (CP §2, Plan 8): hoja `Practicantes`, cabecera en la fila 4."""

from pathlib import Path

from openpyxl import Workbook

from generador_demo.dominios import nombre_completo, practicantes_control, universidad_de
from generador_demo.excel_comun import FORMATO_FECHA, guardar_libro
from generador_demo.parametros import ParametrosGeneracion
from generador_demo.poblacion import Poblacion

FILA_CABECERA = 4
COLUMNAS = [
    "N°",
    "DNI",
    "APELLIDOS Y NOMBRES",
    "CONDICION",
    "FECHA ING",
    "F. RENOVACION",
    "SEDE",
    "UNIVERSIDAD",
    "JEFE INMEDIATO",
    "GERENCIA",
]


def nombre_archivo(pob: Poblacion) -> str:
    return f"BD Practicantes {pob.fecha_corte_base:%d.%m.%Y}.xlsx"


def escribir_practicantes(pob: Poblacion, p: ParametrosGeneracion, carpeta: Path) -> Path:
    wb = Workbook()
    hoja = wb.active
    hoja.title = "Practicantes"
    hoja["A1"] = "CONTROL DE PRACTICANTES"
    for j, nombre in enumerate(COLUMNAS, start=1):
        hoja.cell(row=FILA_CABECERA, column=j, value=nombre)
    for i, f in enumerate(practicantes_control(pob, p), start=1):
        x = f.persona
        valores = [
            i,
            x.numero_documento,
            nombre_completo(x),
            f.condicion,
            x.fecha_entrada,
            f.renovacion,
            x.sede,
            universidad_de(x),
            f"Jefatura de {x.area}",
            x.gerencia,
        ]
        for j, valor in enumerate(valores, start=1):
            celda = hoja.cell(row=FILA_CABECERA + i, column=j, value=valor)
            if j in (5, 6):
                celda.number_format = FORMATO_FECHA
    return guardar_libro(wb, carpeta / nombre_archivo(pob))
