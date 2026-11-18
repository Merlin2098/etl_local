import json

import pytest
from openpyxl import load_workbook

from generador_demo.cli import generar, main
from generador_demo.parametros import ParametrosGeneracion

TOTALES = {"+ TOTAL INGRESOS", "- TOTAL DESCUENTOS"}


@pytest.fixture(scope="module")
def salida(tmp_path_factory):
    return generar(ParametrosGeneracion(), tmp_path_factory.mktemp("gen"), "limpio")


def _leer_planilla(ruta):
    hoja = load_workbook(ruta)["Planilla"]
    cab = [c.value for c in hoja[6]]
    filas = [f for f in hoja.iter_rows(min_row=7, values_only=True) if f[0] and f[0] not in TOTALES]
    return cab, filas


def test_estructura_de_salida(salida):
    assert salida.name == "limpio"
    planillas = sorted(p.name for p in (salida / "planilla").glob("*.xlsx"))
    assert len(planillas) == 6
    assert planillas[0] == "Planilla 2025-06 Empleados.xlsx"
    assert (salida / "planilla" / "licencias" / "CONTROL DE LICENCIAS.xlsx").exists()
    assert (salida / "base_personal" / "BD. 12.11.2025..xlsx").exists()
    assert (salida / "manifiesto.json").exists()


def test_manifiesto_cuadra_con_excel(salida):
    m = json.loads((salida / "manifiesto.json").read_text(encoding="utf-8"))
    assert m["modo"] == "limpio" and m["semilla"] == 42
    for periodo, datos in m["periodos"].items():
        cab, filas = _leer_planilla(salida / "planilla" / f"Planilla {periodo} Empleados.xlsx")
        assert len(filas) == datos["personas_planilla"]
        i = cab.index("DIAS GOCE HABER(D)")
        assert sum(f[i] or 0 for f in filas) == datos["dias_ausencia"]["dias_licencia_con_goce"]
        i = cab.index("DIAS DESC MEDICO (P)")
        assert sum(f[i] or 0 for f in filas) == datos["dias_ausencia"]["dias_descanso_medico"]


def test_manifiesto_base_personal(salida):
    m = json.loads((salida / "manifiesto.json").read_text(encoding="utf-8"))
    assert m["base_personal"]["fecha_corte"] == "2025-11-12"
    assert m["base_personal"]["practicantes"] == 60
    assert m["base_personal"]["casos_contrato_plazo"] == 18
    assert m["base_personal"]["casos_jubilacion"] == 13
    assert m["periodos"]["2025-11"]["personas_sin_base"] == 8


def test_manifiesto_entradas_salidas(salida):
    m = json.loads((salida / "manifiesto.json").read_text(encoding="utf-8"))
    p = ParametrosGeneracion()
    for datos in m["periodos"].values():
        assert p.entradas_por_mes[0] <= datos["entradas"] <= p.entradas_por_mes[1]
        assert p.salidas_por_mes[0] <= datos["salidas"] <= p.salidas_por_mes[1]


def test_segunda_corrida_limpia(tmp_path):
    generar(ParametrosGeneracion(meses=6), tmp_path, "limpio")
    destino = generar(ParametrosGeneracion(meses=3), tmp_path, "limpio")
    assert len(list((destino / "planilla").glob("*.xlsx"))) == 3


def test_no_borra_carpetas_ajenas(tmp_path):
    ajena = tmp_path / "limpio"
    ajena.mkdir()
    (ajena / "mis_notas.txt").write_text("importante", encoding="utf-8")
    with pytest.raises(FileExistsError):
        generar(ParametrosGeneracion(), tmp_path, "limpio")
    assert (ajena / "mis_notas.txt").exists()


def test_periodo_invalido(capsys):
    assert main(["--periodo-fin", "2025-13"]) == 2
    assert "Periodo inválido: 2025-13. Usa el formato AAAA-MM." in capsys.readouterr().out


def test_main_ok(tmp_path):
    assert main(["--salida", str(tmp_path), "--meses", "2"]) == 0
    assert (tmp_path / "limpio" / "manifiesto.json").exists()


def test_meses_fuera_de_rango(capsys):
    assert main(["--meses", "25"]) == 2
    assert "Cantidad de meses inválida: 25. Debe estar entre 1 y 24." in capsys.readouterr().out


def test_corrida_fallida_no_bloquea(tmp_path):
    # Una corrida que se cortó dejó el marcador pero no el manifiesto.
    destino = tmp_path / "limpio"
    destino.mkdir()
    (destino / ".generar-demo").write_text("", encoding="utf-8")
    (destino / "Planilla a medio escribir.xlsx").write_bytes(b"")
    generar(ParametrosGeneracion(meses=1), tmp_path, "limpio")
    assert (destino / "manifiesto.json").exists()
    assert not (destino / "Planilla a medio escribir.xlsx").exists()


def test_marcador_se_escribe_al_crear(tmp_path):
    destino = generar(ParametrosGeneracion(meses=1), tmp_path, "limpio")
    assert (destino / ".generar-demo").exists()


def test_main_reporta_error_de_sistema(tmp_path, monkeypatch, capsys):
    def falla(*args, **kwargs):
        raise PermissionError("archivo abierto en Excel")

    monkeypatch.setattr("generador_demo.cli.generar", falla)
    assert main(["--salida", str(tmp_path)]) == 1
    salida = capsys.readouterr().out
    assert "No se pudo escribir en la carpeta de salida" in salida
    assert "archivo abierto en Excel" in salida
