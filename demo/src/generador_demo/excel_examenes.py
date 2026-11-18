"""Writer de la programación de exámenes médicos de salida (ER §2.1, Plan 8): hoja `DATA`, cabecera en A3."""

from pathlib import Path

from openpyxl import Workbook

from generador_demo.dominios import examenes_salida, fecha_cese, nombre_completo
from generador_demo.excel_comun import FORMATO_FECHA, guardar_libro
from generador_demo.poblacion import Poblacion

NOMBRE_ARCHIVO = "Examenes de salida.xlsx"
FILA_CABECERA = 3
COLUMNAS = ["NOMBRE", "DNI", "FECHA DE CESE", "CAUSA DE SALIDA", "CARGO", "NOMBRE DE CC"]


def escribir_examenes(pob: Poblacion, carpeta: Path) -> Path:
    wb = Workbook()
    hoja = wb.active
    hoja.title = "DATA"
    hoja["A1"] = "PROGRAMACIÓN DE EXÁMENES MÉDICOS DE SALIDA"
    for j, nombre in enumerate(COLUMNAS, start=1):
        hoja.cell(row=FILA_CABECERA, column=j, value=nombre)
    for i, f in enumerate(examenes_salida(pob), start=FILA_CABECERA + 1):
        x = f.persona
        valores = [
            nombre_completo(x),
            x.numero_documento,
            fecha_cese(f, pob),
            x.motivo_salida or "FIN DE CONVENIO",
            f.cargo,
            f.codigo_cc,
        ]
        for j, valor in enumerate(valores, start=1):
            celda = hoja.cell(row=i, column=j, value=valor)
            if j == 3:
                celda.number_format = FORMATO_FECHA
    return guardar_libro(wb, carpeta / NOMBRE_ARCHIVO)
