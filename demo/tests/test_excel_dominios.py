"""Excel de los dominios del Plan 8: régimen especial, altas, practicantes y exámenes de salida."""

import json

import pytest
from openpyxl import load_workbook

from generador_demo.cli import generar
from generador_demo.parametros import ParametrosGeneracion

P = ParametrosGeneracion(meses=3)


@pytest.fixture(scope="module")
def limpio(tmp_path_factory):
    return generar(P, tmp_path_factory.mktemp("gen"), "limpio")


def _manifiesto(carpeta):
    return json.loads((carpeta / "manifiesto.json").read_text(encoding="utf-8"))


def _filas(hoja, fila_cabecera):
    cab = [c.value for c in hoja[fila_cabecera]]
    filas = [f for f in hoja.iter_rows(min_row=fila_cabecera + 1, values_only=True) if any(f)]
    return cab, filas


def test_regimen_especial_un_archivo_por_periodo_con_totales(limpio):
    m = _manifiesto(limpio)
    archivos = sorted((limpio / "planilla_regimen_especial").glob("*.xlsx"))
    assert [a.name for a in archivos] == [
        f"Planilla Regimen Especial {p}.xlsx" for p in m["periodos"]
    ]
    for a in archivos:
        periodo = a.stem.split()[-1]
        hoja = load_workbook(a)["Planilla"]
        assert hoja["A6"].value == "CÓDIGO CORPORATIVO"
        cab, filas = _filas(hoja, 6)
        assert filas[-1][0] == "TOTALES"
        dni = cab.index("DNI/CEX")
        con_dni = [f for f in filas if f[dni]]
        assert len(con_dni) == m["planilla_regimen_especial"][periodo]
        assert "NETO" in cab


def test_regimen_especial_no_esta_en_la_planilla(limpio):
    m = _manifiesto(limpio)
    periodo = next(iter(m["periodos"]))
    hoja = load_workbook(limpio / "planilla_regimen_especial" / f"Planilla Regimen Especial {periodo}.xlsx")["Planilla"]
    cab, filas = _filas(hoja, 6)
    especiales = {f[cab.index("DNI/CEX")] for f in filas if f[cab.index("DNI/CEX")]}
    hoja_p = load_workbook(limpio / "planilla" / f"Planilla {periodo} Empleados.xlsx")["Planilla"]
    cab_p, filas_p = _filas(hoja_p, 6)
    estandar = {f[cab_p.index("DNI/CEX")] for f in filas_p}
    assert especiales and not (especiales & estandar)


def test_altas_cuadran_con_entradas_del_mes(limpio):
    m = _manifiesto(limpio)
    for periodo, datos in m["periodos"].items():
        wb = load_workbook(limpio / "altas_personal" / f"Altas de personal {periodo}.xlsx")
        assert wb.sheetnames == ["EMPLEADOS", "PRACTICANTES"]
        _, empleados = _filas(wb["EMPLEADOS"], 1)
        cab_p, practicantes = _filas(wb["PRACTICANTES"], 1)
        assert cab_p[0] == "Numero Documento"
        assert len(empleados) == m["altas_personal"][periodo]["empleados"] == datos["entradas"]
        assert len(practicantes) == m["altas_personal"][periodo]["practicantes"]


def test_practicantes_ventanas(limpio):
    m = _manifiesto(limpio)
    assert m["practicantes"] == {
        "filas": 60,
        "por_cumplir_1": 6,
        "cumplio_1": 13,
        "por_cumplir_2": 5,
        "sin_renovacion": 2,
    }
    archivos = list((limpio / "practicantes").glob("*.xlsx"))
    assert [a.name for a in archivos] == ["BD Practicantes 12.11.2025.xlsx"]
    hoja = load_workbook(archivos[0])["Practicantes"]
    cab, filas = _filas(hoja, 4)
    assert cab[:4] == ["N°", "DNI", "APELLIDOS Y NOMBRES", "CONDICION"]
    assert len(filas) == 60
    ing, ren, cond = cab.index("FECHA ING"), cab.index("F. RENOVACION"), cab.index("CONDICION")
    prof = [f for f in filas if f[cond] == "PRACTICANTE PROFESIONAL"]
    assert len(prof) == 50
    dias = [(f[ren] - f[ing]).days for f in prof if f[ren] is not None]
    assert sum(335 <= d <= 364 for d in dias) == 6
    assert sum(640 <= d <= 729 for d in dias) == 5
    assert sum(d >= 365 for d in dias) == 13


def test_examenes_estados_del_cruce(limpio):
    e = _manifiesto(limpio)["examenes_salida"]
    assert e["sin_codigo"] == 2
    assert e["codigo_no_encontrado"] == 2
    assert e["practicantes_excluidos"] == 3
    assert e["ok"] + e["sin_codigo"] + e["codigo_no_encontrado"] == e["filas"]
    hoja = load_workbook(limpio / "examenes_salida" / "Examenes de salida.xlsx")["DATA"]
    cab, filas = _filas(hoja, 3)
    assert cab == ["NOMBRE", "DNI", "FECHA DE CESE", "CAUSA DE SALIDA", "CARGO", "NOMBRE DE CC"]
    assert len(filas) == e["filas"] + e["practicantes_excluidos"]
    assert sum(f[4] == "STUDENT INTERNAL" for f in filas) == 3
