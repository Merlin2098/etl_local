"""Writer de la base de personal con el layout legacy (BD-BRZ-02, 05, 10, 11).

Archivo `BD. DD.MM.YYYY..xlsx`, título en las filas 1–9, cabecera en A10 y datos desde la
fila 11. La hoja se llama `Personal`: el legacy exigía el nombre del cliente, que aquí
no se usa.
"""

from collections.abc import Callable
from datetime import date
from pathlib import Path

from openpyxl import Workbook

from generador_demo.excel_comun import FORMATO_FECHA, guardar_libro
from generador_demo.poblacion import Persona, Poblacion

FILA_CABECERA = 10

COLUMNAS_BASE_PERSONAL: list[tuple[str, Callable[[Persona], object]]] = [
    ("NUMERO DE DOC", lambda x: x.numero_documento),
    ("CODIGO SAP2", lambda x: x.codigo_empleado),
    ("NOMBRE COMPLETOS", lambda x: f"{x.apellido_paterno} {x.apellido_materno} {x.nombres}"),
    ("GERENCIA", lambda x: x.gerencia),
    ("SEXO", lambda x: x.sexo),
    ("SEDE3", lambda x: x.sede),
    ("WHITE COLLAR / BLUE COLLAR", lambda x: x.tipo_trabajador),
    ("Modalidad de Contrato", lambda x: x.modalidad_contrato),
    ("Fecha de Termino", lambda x: x.fecha_fin_contrato),
    ("SERVICIO", lambda x: x.servicio),
    ("REGIMEN DE TRABAJO", lambda x: x.regimen_trabajo),
    ("FECH_INGR.", lambda x: x.fecha_entrada),
    ("FECHA DE NAC.", lambda x: x.fecha_nacimiento),
    ("CC", lambda x: x.centro_costo),
    ("NOMBRE CC", lambda x: x.nombre_centro_costo),
    ("CATEGORIA CC", lambda x: x.categoria_centro_costo),
]


def escribir_base_personal(pob: Poblacion, carpeta: Path, nombre_hoja: str = "Personal") -> Path:
    corte = pob.fecha_corte_base
    wb = Workbook()
    hoja = wb.active
    hoja.title = nombre_hoja
    hoja["A1"] = "BASE DE DATOS DE PERSONAL"
    hoja["A2"] = f"Fecha de corte: {corte:%d/%m/%Y}"
    hoja["A3"] = "Empresa: EMPRESA DEMO S.A.C."
    for j, (nombre, _) in enumerate(COLUMNAS_BASE_PERSONAL, start=1):
        hoja.cell(row=FILA_CABECERA, column=j, value=nombre)

    personas = sorted(pob.en_base_personal(), key=lambda x: x.numero_documento)
    for i, persona in enumerate(personas, start=FILA_CABECERA + 1):
        for j, (_, extraer) in enumerate(COLUMNAS_BASE_PERSONAL, start=1):
            celda = hoja.cell(row=i, column=j, value=extraer(persona))
            if isinstance(celda.value, date):
                celda.number_format = FORMATO_FECHA

    return guardar_libro(wb, carpeta / f"BD. {corte:%d.%m.%Y}..xlsx")
