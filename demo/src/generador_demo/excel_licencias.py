"""Writer del control de licencias con el layout legacy (LIC-BRZ-01…05).

Dos hojas, cabecera en la fila 2 y datos desde la fila 3. `DNI/CEX` va en la columna A
porque el legacy descarta las filas con la columna A vacía (LIC-BRZ-04).
"""

from pathlib import Path

from openpyxl import Workbook

from generador_demo.excel_comun import guardar_libro
from generador_demo.poblacion import Poblacion
from generador_demo.registros import RegistroMes

NOMBRE_ARCHIVO = "CONTROL DE LICENCIAS.xlsx"
_COMUNES = ["DNI/CEX", "PERIODO", "APELLIDO PATERNO", "APELLIDO MATERNO", "NOMBRES"]
HOJAS = (
    ("LICENCIA CON GOCE", "MOTIVO DE LIC.CON GOCE", "con_goce"),
    ("LICENCIA SIN GOCE", "MOTIVO DE LIC.S.G.H", "sin_goce"),
)


def escribir_licencias(pob: Poblacion, registros: list[RegistroMes], carpeta: Path) -> Path:
    por_doc = {x.numero_documento: x for x in pob.personas}
    ordenados = sorted(registros, key=lambda r: (r.periodo, r.numero_documento))

    wb = Workbook()
    wb.remove(wb.active)
    for nombre_hoja, columna_motivo, sufijo in HOJAS:
        hoja = wb.create_sheet(nombre_hoja)
        hoja["A1"] = f"CONTROL DE LICENCIAS - {nombre_hoja}"
        for j, nombre in enumerate([*_COMUNES, columna_motivo], start=1):
            hoja.cell(row=2, column=j, value=nombre)
        fila = 3
        for r in ordenados:
            if getattr(r, f"dias_licencia_{sufijo}") <= 0:
                continue
            x = por_doc[r.numero_documento]
            valores = [
                x.numero_documento,
                r.periodo,
                x.apellido_paterno,
                x.apellido_materno,
                x.nombres,
                getattr(r, f"motivo_licencia_{sufijo}"),
            ]
            for j, valor in enumerate(valores, start=1):
                hoja.cell(row=fila, column=j, value=valor)
            fila += 1

    return guardar_libro(wb, carpeta / NOMBRE_ARCHIVO)
