"""Fallos de la paridad: sin dependencias, corridas que fallan y carpetas ajenas."""

import subprocess

import pytest

from generador_demo.parametros import ParametrosGeneracion
from paridad_legacy import entorno
from paridad_legacy.entorno import paridad_disponible

P = ParametrosGeneracion(meses=1)


@pytest.fixture
def legacy_falso(tmp_path):
    raiz = tmp_path / "legacy"
    (raiz / "assets" / "validate_source").mkdir(parents=True)
    (raiz / "etl_manager.py").write_text("", encoding="utf-8")
    (raiz / "assets" / "validate_source" / "bd.yaml").write_text(
        "sheet_rules:\n  required_sheets:\n  - ACME\n", encoding="utf-8"
    )
    return raiz


def test_paridad_disponible_sin_dependencias(monkeypatch, legacy_falso):
    real = entorno.importlib.util.find_spec
    monkeypatch.setattr(
        entorno.importlib.util, "find_spec", lambda n: None if n == "yaml" else real(n)
    )
    disponible, motivo = paridad_disponible(legacy_falso)
    assert not disponible
    assert "yaml" in motivo and "uv sync --project demo --group paridad" in motivo


def test_paridad_disponible_sin_legacy(tmp_path, monkeypatch):
    monkeypatch.setattr(entorno.importlib.util, "find_spec", lambda n: object())
    disponible, motivo = paridad_disponible(tmp_path / "no-existe")
    assert not disponible and "RUTA_LEGACY" in motivo


requiere_dependencias = pytest.mark.skipif(
    not paridad_disponible(None, verificar_legacy=False)[0],
    reason="faltan dependencias del grupo paridad",
)


def _fixture_previo(carpeta, nombre, contenido):
    carpeta.mkdir(parents=True)
    (carpeta / ".generar-golden").write_text("", encoding="utf-8")
    (carpeta / nombre).write_text(contenido, encoding="utf-8")


@requiere_dependencias
def test_dominio_fallido_no_reemplaza(tmp_path, legacy_falso, monkeypatch):
    from paridad_legacy import golden
    from paridad_legacy.ejecutar import ResultadoLegacy

    _fixture_previo(tmp_path / "out" / "limpio", "planilla_gold.parquet", "viejo")
    monkeypatch.setattr(
        golden,
        "correr_legacy",
        lambda *a: [
            ResultadoLegacy("planilla", False, "fallo simulado", {}),
            ResultadoLegacy("base_personal", True, None, {}),
        ],
    )
    with pytest.raises(golden.GoldenIncompleto, match="planilla"):
        golden.generar_golden(legacy_falso, tmp_path / "out", "limpio", P)
    anterior = tmp_path / "out" / "limpio" / "planilla_gold.parquet"
    assert anterior.read_text(encoding="utf-8") == "viejo"
    assert sorted(p.name for p in (tmp_path / "out").iterdir()) == ["limpio"]


@requiere_dependencias
def test_excepcion_conserva_fixtures_anteriores(tmp_path, legacy_falso, monkeypatch):
    from paridad_legacy import golden

    _fixture_previo(tmp_path / "out" / "casos", "CASO-01.json", "{}")

    def falla(*a, **k):
        raise RuntimeError("preflight simulado roto")

    monkeypatch.setattr(golden, "validar_con_legacy", falla)
    with pytest.raises(RuntimeError):
        golden.generar_golden(legacy_falso, tmp_path / "out", "casos", P)
    assert (tmp_path / "out" / "casos" / "CASO-01.json").exists()
    assert sorted(p.name for p in (tmp_path / "out").iterdir()) == ["casos"]


@requiere_dependencias
def test_carpeta_con_resumen_sin_marcador_no_se_borra(tmp_path, legacy_falso):
    from paridad_legacy import golden

    ajena = tmp_path / "out" / "casos"
    ajena.mkdir(parents=True)
    (ajena / "resumen.json").write_text("{}", encoding="utf-8")
    (ajena / "mio.txt").write_text("x", encoding="utf-8")
    with pytest.raises(FileExistsError):
        golden.generar_golden(legacy_falso, tmp_path / "out", "casos", P)
    assert (ajena / "mio.txt").exists()


@pytest.mark.parametrize(
    "error",
    [subprocess.TimeoutExpired("legacy", 900), RuntimeError("roto")],
    ids=["timeout", "runtime"],
)
def test_cli_reporta_fallos_controlados(tmp_path, legacy_falso, monkeypatch, capsys, error):
    from paridad_legacy import cli, golden

    def falla(*a, **k):
        raise error

    monkeypatch.setattr(golden, "generar_golden", falla)
    assert cli.main(["--legacy", str(legacy_falso), "--salida", str(tmp_path / "out")]) == 1
    assert "No se generó el golden" in capsys.readouterr().out


def test_cli_pasa_meses(tmp_path, legacy_falso, monkeypatch):
    from paridad_legacy import cli, golden

    capturado = {}

    def doble(legacy, salida, modo, p):
        capturado["p"] = p
        return salida / modo

    monkeypatch.setattr(golden, "generar_golden", doble)
    args = ["--legacy", str(legacy_falso), "--salida", str(tmp_path), "--meses", "3"]
    assert cli.main(args) == 0
    assert capturado["p"].meses == 3


@pytest.mark.parametrize("meses", ["0", "25", "tres"])
def test_cli_rechaza_meses_fuera_de_rango(tmp_path, legacy_falso, meses):
    from paridad_legacy import cli

    with pytest.raises(SystemExit) as salida:
        cli.main(["--legacy", str(legacy_falso), "--salida", str(tmp_path), "--meses", meses])
    assert salida.value.code == 2
