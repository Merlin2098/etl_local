import json

import pytest
from openpyxl import load_workbook

from generador_demo.casos import CASOS
from generador_demo.cli import generar, main
from generador_demo.parametros import ParametrosGeneracion
from generador_demo.suciedad import INYECTORES

P = ParametrosGeneracion(meses=3)


@pytest.fixture(scope="module")
def generados(tmp_path_factory):
    base = tmp_path_factory.mktemp("gen")
    return generar(P, base, "limpio"), generar(P, base, "sucio"), generar(P, base, "casos")


def _planilla(carpeta):
    return load_workbook(carpeta / "planilla" / "Planilla 2025-11 Empleados.xlsx")["Planilla"]


def _manifiesto(carpeta):
    return json.loads((carpeta / "manifiesto.json").read_text(encoding="utf-8"))


def _filas(hoja, desde):
    return list(hoja.iter_rows(min_row=desde, values_only=True))


def _col(hoja, fila_cab, nombre):
    cab = [c.value for c in hoja[fila_cab]]
    return cab.index(nombre)


def test_hay_seis_inyectores():
    assert sorted(INYECTORES) == [f"SUC-0{i}" for i in range(1, 7)]


def test_suc_01_dni_numerico(generados):
    hoja = _planilla(generados[1])
    i = _col(hoja, 6, "DNI/CEX")
    # Celda numérica en vez de texto; openpyxl relee 99000123.0 como int.
    assert any(isinstance(f[i], (int, float)) for f in _filas(hoja, 7))
    limpio = _planilla(generados[0])
    assert all(isinstance(f[i], str) for f in _filas(limpio, 7) if f[i] is not None)


def test_suc_02_espacios_en_encabezados(generados):
    cab = [c.value for c in _planilla(generados[1])[6] if c.value is not None]
    assert sum(v != " ".join(v.split()) for v in cab) == 3


def test_suc_03_fechas_como_texto(generados):
    hoja = load_workbook(generados[1] / "base_personal" / "BD. 12.11.2025..xlsx")["Personal"]
    i = _col(hoja, 10, "FECH_INGR.")
    textos = [f[i] for f in _filas(hoja, 11) if isinstance(f[i], str)]
    assert textos and all(len(t) == 10 and t[2] == "/" and t[5] == "/" for t in textos)


def test_suc_04_nd_en_centro_de_costos(generados):
    hoja = _planilla(generados[1])
    i = _col(hoja, 6, "CENTRO DE COSTOS")
    assert any(f[i] == "#N/A" for f in _filas(hoja, 7))


def test_suc_05_fila_vacia_intercalada(generados):
    filas = _filas(_planilla(generados[1]), 7)
    vacias = [n for n, f in enumerate(filas) if all(v is None for v in f)]
    assert vacias and 0 < vacias[0] < len(filas) - 2


def test_suc_06_dni_con_punto_cero_en_licencias(generados):
    hoja = load_workbook(generados[1] / "planilla" / "licencias" / "CONTROL DE LICENCIAS.xlsx")[
        "LICENCIA CON GOCE"
    ]
    assert any(isinstance(f[0], str) and f[0].endswith(".0") for f in _filas(hoja, 3))


def test_sucio_mantiene_conteos(generados):
    limpio, sucio = _manifiesto(generados[0]), _manifiesto(generados[1])
    assert sucio.pop("modo") == "sucio"
    limpio.pop("modo")
    assert sucio == limpio


def test_sucio_no_cambia_filas_con_datos(generados):
    def con_dni(carpeta):
        hoja = _planilla(carpeta)
        i = _col(hoja, 6, "DNI/CEX")
        return sum(f[i] is not None for f in _filas(hoja, 7))

    assert con_dni(generados[0]) == con_dni(generados[1])


def test_casos_completos(generados):
    carpeta = generados[2]
    subcarpetas = sorted(p.name for p in carpeta.iterdir() if p.is_dir())
    assert subcarpetas == sorted(CASOS) == [f"CASO-{i:02d}" for i in range(1, 11)]
    for caso in subcarpetas:
        esperado = json.loads((carpeta / caso / "esperado.json").read_text(encoding="utf-8"))
        assert set(esperado) == {"caso", "regla", "hoja", "celda"}
        assert esperado["caso"] == caso
        archivos = [p for p in (carpeta / caso).iterdir() if p.suffix == ".xlsx"]
        assert len(archivos) == 1


def _archivo_caso(carpeta, caso):
    return next(p for p in (carpeta / caso).iterdir() if p.suffix == ".xlsx")


def test_caso_01_cabecera_en_a7(generados):
    hoja = load_workbook(_archivo_caso(generados[2], "CASO-01"))["Planilla"]
    assert hoja["A6"].value is None
    assert hoja["A7"].value == "CÓDIGO CORPORATIVO"


def test_caso_02_hoja_renombrada(generados):
    assert load_workbook(_archivo_caso(generados[2], "CASO-02")).sheetnames == ["Hoja1"]


def test_caso_03_sin_dni(generados):
    hoja = load_workbook(_archivo_caso(generados[2], "CASO-03"))["Planilla"]
    assert "DNI/CEX" not in [c.value for c in hoja[6]]


def test_caso_04_nombre_sin_periodo(generados):
    assert _archivo_caso(generados[2], "CASO-04").name == "Planilla Empleados.xlsx"


def test_caso_05_nombre_base_invalido(generados):
    assert _archivo_caso(generados[2], "CASO-05").name == "Base de Datos 12.11.2025.xlsx"


def test_caso_06_licencias_sin_hoja(generados):
    wb = load_workbook(_archivo_caso(generados[2], "CASO-06"))
    assert wb.sheetnames == ["LICENCIA CON GOCE"]


def test_main_acepta_los_tres_modos(tmp_path):
    for modo in ("limpio", "sucio", "casos"):
        assert main(["--salida", str(tmp_path), "--meses", "3", "--modo", modo]) == 0


def test_suc_04_es_error_real_de_excel(generados):
    hoja = _planilla(generados[1])
    i = _col(hoja, 6, "CENTRO DE COSTOS") + 1
    errores = [
        hoja.cell(row=f, column=i)
        for f in range(7, hoja.max_row + 1)
        if hoja.cell(row=f, column=i).data_type == "e"
    ]
    assert errores and all(c.value == "#N/A" for c in errores)


def _normalizar(valor):
    return " ".join(str(valor).split())


def _leer_normalizado(ruta):
    """Lector independiente con las reglas legacy: cabecera normalizada, sin vacías ni totales."""
    hoja = load_workbook(ruta)["Planilla"]
    cab = [_normalizar(c.value) if c.value is not None else None for c in hoja[6]]
    filas = []
    for f in hoja.iter_rows(min_row=7, values_only=True):
        if f[0] is None or str(f[0]).lstrip("+-* ").upper().startswith("TOTAL"):
            continue
        filas.append(dict(zip(cab, f, strict=False)))
    return filas


def test_sucio_cuadra_con_manifiesto_releyendo_los_excel(generados):
    from generador_demo.parametros import fin_mes, inicio_mes

    columnas_dias = {
        "dias_descanso_medico": "DIAS DESC MEDICO (P)",
        "dias_licencia_con_goce": "DIAS GOCE HABER(D)",
        "dias_licencia_sin_goce": "DIAS DIST.LIC.S.GH",
        "dias_licencia_paternidad": "DIAS LIC. PATE (P)",
        "dias_subsidio_enfermedad": "DIAS SUBSIDIO ENFERMEDAD",
        "dias_subsidio_maternidad": "DIAS SUB. MATERNIDAD",
    }
    sucio = generados[1]
    m = _manifiesto(sucio)
    for periodo, esperado in m["periodos"].items():
        filas = _leer_normalizado(sucio / "planilla" / f"Planilla {periodo} Empleados.xlsx")
        docs = {str(f["DNI/CEX"]).removesuffix(".0") for f in filas}
        ini, fin = inicio_mes(periodo), fin_mes(periodo)

        def en_mes(valor, ini=ini, fin=fin):
            return valor is not None and ini <= valor.date() <= fin

        assert len(docs) == esperado["personas_planilla"]
        assert sum(en_mes(f["FECHA DE INGRESO"]) for f in filas) == esperado["entradas"]
        assert sum(en_mes(f["FECHA CESE"]) for f in filas) == esperado["salidas"]
        for campo, columna in columnas_dias.items():
            total = sum(f.get(columna) or 0 for f in filas)
            assert total == esperado["dias_ausencia"][campo], (periodo, campo)


def test_casos_de_los_dominios_del_plan_8(generados):
    carpeta = generados[2]
    hoja = load_workbook(_archivo_caso(carpeta, "CASO-07"))["Planilla"]
    assert "DNI/CEX" not in [c.value for c in hoja[6]]
    assert load_workbook(_archivo_caso(carpeta, "CASO-08")).sheetnames == ["EMPLEADOS"]
    assert _archivo_caso(carpeta, "CASO-09").name == "Practicantes.xlsx"
    hoja = load_workbook(_archivo_caso(carpeta, "CASO-10"))["DATA"]
    assert "FECHA DE CESE" not in [c.value for c in hoja[3]]
