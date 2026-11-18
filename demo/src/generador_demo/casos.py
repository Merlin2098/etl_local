"""Modo `casos`: un archivo roto por subcarpeta, con el error que el motor debe reportar.

Cada caso trae `esperado.json` con la regla incumplida, la hoja y la celda o fila donde
el usuario debe mirar (TR-D13: mensajes en términos de Excel).
"""

import json
from pathlib import Path

from openpyxl import load_workbook

from generador_demo.excel_altas import escribir_altas
from generador_demo.excel_base_personal import escribir_base_personal
from generador_demo.excel_comun import guardar_libro
from generador_demo.excel_examenes import escribir_examenes
from generador_demo.excel_licencias import escribir_licencias
from generador_demo.excel_planilla import escribir_planilla
from generador_demo.excel_practicantes import escribir_practicantes
from generador_demo.excel_regimen_especial import escribir_regimen_especial
from generador_demo.parametros import ParametrosGeneracion
from generador_demo.poblacion import Poblacion, simular_poblacion
from generador_demo.registros import RegistroMes, generar_registros

CASOS: dict[str, dict[str, str]] = {
    "CASO-01": {
        "regla": "NOM-BRZ-13 / TR-D13: la cabecera debe empezar en A6",
        "hoja": "Planilla",
        "celda": "A6",
    },
    "CASO-02": {
        "regla": "NOM-BRZ-06: debe existir la hoja Planilla",
        "hoja": "Planilla",
        "celda": "",
    },
    "CASO-03": {
        "regla": "NOM-BRZ-13: falta la columna obligatoria DNI/CEX",
        "hoja": "Planilla",
        "celda": "fila 6",
    },
    "CASO-04": {
        "regla": "NOM-BRZ-04: el nombre del archivo debe incluir el periodo AAAA-MM",
        "hoja": "",
        "celda": "",
    },
    "CASO-05": {
        "regla": "BD-BRZ-02: el nombre debe ser 'BD. DD.MM.AAAA..xlsx'",
        "hoja": "",
        "celda": "",
    },
    "CASO-06": {
        "regla": "LIC-BRZ-06: debe existir la hoja LICENCIA SIN GOCE",
        "hoja": "LICENCIA SIN GOCE",
        "celda": "",
    },
    "CASO-07": {
        "regla": "RM-BRZ-08: falta la columna obligatoria DNI/CEX en la planilla de régimen especial",
        "hoja": "Planilla",
        "celda": "fila 6",
    },
    "CASO-08": {
        "regla": "PDT §2.2: debe existir la hoja PRACTICANTES en las altas de personal",
        "hoja": "PRACTICANTES",
        "celda": "",
    },
    "CASO-09": {
        "regla": "CP §2: el nombre debe ser 'BD Practicantes DD.MM.AAAA.xlsx'",
        "hoja": "",
        "celda": "",
    },
    "CASO-10": {
        "regla": "ER-BRZ-01: falta la columna obligatoria FECHA DE CESE en los exámenes de salida",
        "hoja": "DATA",
        "celda": "fila 3",
    },
}


def _quitar_columna(ruta: Path, hoja: str, fila: int, nombre: str) -> None:
    wb = load_workbook(ruta)
    h = wb[hoja]
    h.delete_cols(next(c.column for c in h[fila] if c.value == nombre))
    guardar_libro(wb, ruta)


def _planilla(pob: Poblacion, reg: list[RegistroMes], p: ParametrosGeneracion, d: Path) -> Path:
    return escribir_planilla(pob, reg, p, pob.periodos[-1], d)


def generar_casos(p: ParametrosGeneracion, carpeta: Path) -> list[Path]:
    pob = simular_poblacion(p)
    reg = generar_registros(pob, p)
    rutas: list[Path] = []
    for caso, esperado in CASOS.items():
        d = carpeta / caso
        d.mkdir(parents=True)
        if caso in ("CASO-01", "CASO-02", "CASO-03", "CASO-04"):
            ruta = _planilla(pob, reg, p, d)
            wb = load_workbook(ruta)
            hoja = wb["Planilla"]
            if caso == "CASO-01":
                hoja.insert_rows(6)
            elif caso == "CASO-02":
                hoja.title = "Hoja1"
            elif caso == "CASO-03":
                col = next(c.column for c in hoja[6] if c.value == "DNI/CEX")
                hoja.delete_cols(col)
            guardar_libro(wb, ruta)
            if caso == "CASO-04":
                ruta = ruta.rename(d / "Planilla Empleados.xlsx")
        elif caso == "CASO-05":
            ruta = escribir_base_personal(pob, d)
            ruta = ruta.rename(d / f"Base de Datos {pob.fecha_corte_base:%d.%m.%Y}.xlsx")
        elif caso == "CASO-06":
            ruta = escribir_licencias(pob, reg, d)
            wb = load_workbook(ruta)
            wb.remove(wb["LICENCIA SIN GOCE"])
            guardar_libro(wb, ruta)
        elif caso == "CASO-07":
            ruta = escribir_regimen_especial(pob, p, pob.periodos[-1], d)
            _quitar_columna(ruta, "Planilla", 6, "DNI/CEX")
        elif caso == "CASO-08":
            ruta = escribir_altas(pob, pob.periodos[-1], d)
            wb = load_workbook(ruta)
            wb.remove(wb["PRACTICANTES"])
            guardar_libro(wb, ruta)
        elif caso == "CASO-09":
            ruta = escribir_practicantes(pob, p, d)
            ruta = ruta.rename(d / "Practicantes.xlsx")
        else:
            ruta = escribir_examenes(pob, d)
            _quitar_columna(ruta, "DATA", 3, "FECHA DE CESE")
        (d / "esperado.json").write_text(
            json.dumps({"caso": caso, **esperado}, ensure_ascii=False, indent=2) + "\n",
            encoding="utf-8",
        )
        rutas.append(ruta)
    (carpeta / "casos.json").write_text(
        json.dumps(sorted(CASOS), ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    return rutas
