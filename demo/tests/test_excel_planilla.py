import hashlib
import re
from datetime import date, datetime

import pytest
from openpyxl import load_workbook

from generador_demo.excel_planilla import COLUMNAS_PLANILLA, escribir_planilla
from generador_demo.parametros import ParametrosGeneracion
from generador_demo.poblacion import simular_poblacion
from generador_demo.registros import generar_registros

P = ParametrosGeneracion()
POB = simular_poblacion(P)
REG = generar_registros(POB, P)
ENCABEZADOS = [nombre for nombre, _ in COLUMNAS_PLANILLA]
TOTALES = ["+ TOTAL INGRESOS", "- TOTAL DESCUENTOS"]


@pytest.fixture(scope="module")
def hoja_noviembre(tmp_path_factory):
    ruta = escribir_planilla(POB, REG, P, "2025-11", tmp_path_factory.mktemp("nov"))
    return ruta, load_workbook(ruta)["Planilla"]


def _encabezados(hoja):
    return [c.value for c in hoja[6] if c.value is not None]


def _filas_datos(hoja):
    return [
        fila
        for fila in hoja.iter_rows(min_row=7, values_only=True)
        if fila[0] is not None and fila[0] not in TOTALES
    ]


def test_nombre_y_hoja(hoja_noviembre):
    ruta, _ = hoja_noviembre
    assert ruta.name == "Planilla 2025-11 Empleados.xlsx"
    assert re.search(r"(\d{4}-\d{2})", ruta.name).group(1) == "2025-11"
    assert load_workbook(ruta).sheetnames == ["Planilla"]


def test_cabecera_en_fila_6_con_columna_dinamica(hoja_noviembre):
    _, hoja = hoja_noviembre
    assert hoja["A6"].value == "CÓDIGO CORPORATIVO"
    assert _encabezados(hoja) == ENCABEZADOS + ["DIAS SUB. MATERNIDAD"]


def test_sin_columna_dinamica_antes_del_indice(tmp_path):
    hoja = load_workbook(escribir_planilla(POB, REG, P, "2025-06", tmp_path))["Planilla"]
    assert _encabezados(hoja) == ENCABEZADOS


def test_filas_de_datos(hoja_noviembre):
    _, hoja = hoja_noviembre
    assert len(_filas_datos(hoja)) == len(POB.en_planilla("2025-11"))


def test_filas_de_totales(hoja_noviembre):
    _, hoja = hoja_noviembre
    ultimas = [fila[0] for fila in hoja.iter_rows(min_row=7, values_only=True)][-2:]
    assert ultimas == TOTALES


def test_titulo_sin_cliente(hoja_noviembre):
    _, hoja = hoja_noviembre
    assert hoja["A1"].value == "PLANILLA DE REMUNERACIONES"
    assert hoja["A2"].value == "Periodo: 2025-11"
    assert hoja["A3"].value == "Empresa: EMPRESA DEMO S.A.C."
    assert hoja["A4"].value is None and hoja["A5"].value is None


def test_fechas_son_fechas(hoja_noviembre):
    _, hoja = hoja_noviembre
    i_nac = ENCABEZADOS.index("F. NACIMIENTO")
    i_ing = ENCABEZADOS.index("FECHA DE INGRESO")
    for fila in _filas_datos(hoja):
        assert isinstance(fila[i_nac], (date, datetime))
        assert isinstance(fila[i_ing], (date, datetime))


def test_dias_cuadran_con_registros(hoja_noviembre):
    _, hoja = hoja_noviembre
    i = ENCABEZADOS.index("DIAS GOCE HABER(D)")
    esperado = sum(r.dias_licencia_con_goce for r in REG if r.periodo == "2025-11")
    assert sum(fila[i] or 0 for fila in _filas_datos(hoja)) == esperado


def test_byte_a_byte(tmp_path):
    a = escribir_planilla(POB, REG, P, "2025-11", tmp_path / "a")
    b = escribir_planilla(POB, REG, P, "2025-11", tmp_path / "b")
    assert hashlib.sha256(a.read_bytes()).digest() == hashlib.sha256(b.read_bytes()).digest()
