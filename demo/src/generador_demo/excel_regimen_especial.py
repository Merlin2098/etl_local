"""Writer de la planilla del régimen especial (layout de RM-BRZ-05…07, Plan 8).

Hoja `Planilla`, cabecera en la fila 6, datos desde la fila 7 y una fila `TOTALES` al final, sin DNI.
Lleva montos (TOTAL INGRESOS…NETO) como el legacy; el motor no los lee (TR-D22).
"""

import random
from pathlib import Path

from openpyxl import Workbook

from generador_demo.excel_comun import FORMATO_FECHA, guardar_libro
from generador_demo.parametros import ParametrosGeneracion
from generador_demo.poblacion import Poblacion

FILA_CABECERA = 6
COLUMNAS = [
    "CÓDIGO CORPORATIVO",
    "DNI/CEX",
    "SEXO",
    "F. NACIMIENTO",
    "PUESTO",
    "SEDE",
    "ÁREA",
    "CENTRO DE COSTOS",
    "DESC. CENTRO DE COSTO",
    "FECHA DE INGRESO",
    "DIAS TRABAJADOS",
    "DIAS DESC MEDICO (P)",
    "DIAS GOCE HABER(D)",
    "DIAS DIST.LIC.S.GH",
    "DIAS LIC. PATE (P)",
    "# HRS EXTRAS SIMP.25%",
    "# HRS EXTRAS SIMP.35%",
    "#HRS EXTRAS DOBLES",
    "TOTAL INGRESOS",
    "TOTAL DESCUENTOS",
    "TOTAL APORTACIONES",
    "NETO",
]


def nombre_archivo(periodo: str) -> str:
    return f"Planilla Regimen Especial {periodo}.xlsx"


def escribir_regimen_especial(
    pob: Poblacion, p: ParametrosGeneracion, periodo: str, carpeta: Path
) -> Path:
    rng = random.Random(f"{p.semilla}-especial-{periodo}")
    wb = Workbook()
    hoja = wb.active
    hoja.title = "Planilla"
    hoja["A1"] = "PLANILLA DE REMUNERACIONES - RÉGIMEN ESPECIAL"
    hoja["A2"] = f"PERIODO {periodo}"
    for j, nombre in enumerate(COLUMNAS, start=1):
        hoja.cell(row=FILA_CABECERA, column=j, value=nombre)
    fila = FILA_CABECERA + 1
    total = 0.0
    for x in sorted(pob.regimen_especial, key=lambda x: x.numero_documento):
        descanso = rng.choice([0, 0, 0, 1, 2])
        ingresos = round(x.remuneracion_mensual * rng.uniform(1.0, 1.3), 2)
        descuentos = round(ingresos * 0.13, 2)
        total += ingresos
        valores = [
            x.codigo_empleado,
            x.numero_documento,
            x.sexo,
            x.fecha_nacimiento,
            "OPERADOR DE EQUIPO",
            x.sede,
            x.area,
            x.centro_costo,
            x.nombre_centro_costo,
            x.fecha_entrada,
            30 - descanso,
            descanso,
            rng.choice([0, 0, 1]),
            0,
            0,
            rng.choice([0, 2, 4.5]),
            rng.choice([0, 0, 3]),
            rng.choice([0, 0, 8]),
            ingresos,
            descuentos,
            round(ingresos * 0.09, 2),
            round(ingresos - descuentos, 2),
        ]
        for j, valor in enumerate(valores, start=1):
            celda = hoja.cell(row=fila, column=j, value=valor)
            if j in (4, 10):
                celda.number_format = FORMATO_FECHA
        fila += 1
    hoja.cell(row=fila, column=1, value="TOTALES")
    hoja.cell(row=fila, column=COLUMNAS.index("TOTAL INGRESOS") + 1, value=round(total, 2))
    return guardar_libro(wb, carpeta / nombre_archivo(periodo))
