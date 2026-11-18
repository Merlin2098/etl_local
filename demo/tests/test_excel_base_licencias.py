import re

import pytest
from openpyxl import load_workbook

from generador_demo.excel_base_personal import COLUMNAS_BASE_PERSONAL, escribir_base_personal
from generador_demo.excel_licencias import escribir_licencias
from generador_demo.parametros import ParametrosGeneracion
from generador_demo.poblacion import simular_poblacion
from generador_demo.registros import generar_registros

P = ParametrosGeneracion()
POB = simular_poblacion(P)
REG = generar_registros(POB, P)
ENCABEZADOS_BASE = [nombre for nombre, _ in COLUMNAS_BASE_PERSONAL]


@pytest.fixture(scope="module")
def base(tmp_path_factory):
    ruta = escribir_base_personal(POB, tmp_path_factory.mktemp("base"))
    return ruta, load_workbook(ruta)["Personal"]


@pytest.fixture(scope="module")
def licencias(tmp_path_factory):
    return load_workbook(escribir_licencias(POB, REG, tmp_path_factory.mktemp("lic")))


def _filas_base(hoja):
    return [f for f in hoja.iter_rows(min_row=11, values_only=True) if f[0] is not None]


def test_nombre_base_personal(base):
    ruta, _ = base
    assert re.match(r"^BD\.\s*\d{2}\.\d{2}\.\d{4}\.\.xlsx$", ruta.name)
    assert ruta.name == "BD. 12.11.2025..xlsx"


def test_cabecera_base_en_a10(base):
    _, hoja = base
    assert [c.value for c in hoja[10] if c.value is not None] == ENCABEZADOS_BASE
    assert ENCABEZADOS_BASE[:13] == [
        "NUMERO DE DOC",
        "CODIGO SAP2",
        "NOMBRE COMPLETOS",
        "GERENCIA",
        "SEXO",
        "SEDE3",
        "WHITE COLLAR / BLUE COLLAR",
        "Modalidad de Contrato",
        "Fecha de Termino",
        "SERVICIO",
        "REGIMEN DE TRABAJO",
        "FECH_INGR.",
        "FECHA DE NAC.",
    ]
    assert ENCABEZADOS_BASE[13:] == ["CC", "NOMBRE CC", "CATEGORIA CC"]


def test_filas_base(base):
    _, hoja = base
    filas = _filas_base(hoja)
    assert len(filas) == len(POB.en_base_personal())
    i_mod = ENCABEZADOS_BASE.index("Modalidad de Contrato")
    assert sum(f[i_mod] == "TERMINO DE CONVENIO" for f in filas) == 60


def test_sin_base_ausentes(base):
    _, hoja = base
    docs = {f[0] for f in _filas_base(hoja)}
    sin_base = [x.numero_documento for x in POB.personas if not x.en_base_personal]
    assert len(sin_base) == 8
    assert docs.isdisjoint(sin_base)


def test_fin_contrato_vacio_en_indeterminado(base):
    _, hoja = base
    i_mod = ENCABEZADOS_BASE.index("Modalidad de Contrato")
    i_fin = ENCABEZADOS_BASE.index("Fecha de Termino")
    for f in _filas_base(hoja):
        if f[i_mod] == "PLAZO INDETERMINADO":
            assert f[i_fin] is None
        else:
            assert f[i_fin] is not None


def test_licencias_hojas_y_cabecera(licencias):
    assert licencias.sheetnames == ["LICENCIA CON GOCE", "LICENCIA SIN GOCE"]
    cab_con = [c.value for c in licencias["LICENCIA CON GOCE"][2] if c.value is not None]
    cab_sin = [c.value for c in licencias["LICENCIA SIN GOCE"][2] if c.value is not None]
    base = ["DNI/CEX", "PERIODO", "APELLIDO PATERNO", "APELLIDO MATERNO", "NOMBRES"]
    assert cab_con == base + ["MOTIVO DE LIC.CON GOCE"]
    assert cab_sin == base + ["MOTIVO DE LIC.S.G.H"]


def test_licencias_cuadran_con_registros(licencias):
    con = [f for f in licencias["LICENCIA CON GOCE"].iter_rows(min_row=3, values_only=True)]
    sin = [f for f in licencias["LICENCIA SIN GOCE"].iter_rows(min_row=3, values_only=True)]
    assert len(con) == sum(r.dias_licencia_con_goce > 0 for r in REG)
    assert len(sin) == sum(r.dias_licencia_sin_goce > 0 for r in REG)
    assert all(isinstance(f[1], str) and re.fullmatch(r"\d{4}-\d{2}", f[1]) for f in con + sin)
