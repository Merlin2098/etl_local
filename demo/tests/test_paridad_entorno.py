from pathlib import Path

import pytest

from paridad_legacy.entorno import (
    LegacyNoDisponible,
    entorno_subproceso,
    hoja_requerida_base,
    nombres_a_ocultar,
    ocultar,
    ubicar_legacy,
)


@pytest.fixture
def legacy_falso(tmp_path):
    raiz = tmp_path / "legacy"
    (raiz / "assets" / "validate_source").mkdir(parents=True)
    (raiz / "etl_manager.py").write_text("", encoding="utf-8")
    (raiz / "assets" / "validate_source" / "bd.yaml").write_text(
        "id: bd\nsheet_rules:\n  required_sheets:\n  - ACME\n", encoding="utf-8"
    )
    return raiz


def test_ubicar_legacy_por_argumento_y_variable(legacy_falso, monkeypatch):
    assert ubicar_legacy(legacy_falso) == legacy_falso
    monkeypatch.setenv("RUTA_LEGACY", str(legacy_falso))
    assert ubicar_legacy() == legacy_falso


def test_ubicar_legacy_ausente_lanza_mensaje(tmp_path, monkeypatch):
    monkeypatch.delenv("RUTA_LEGACY", raising=False)
    with pytest.raises(LegacyNoDisponible) as error:
        ubicar_legacy(tmp_path / "no-existe")
    assert "RUTA_LEGACY" in str(error.value)
    assert isinstance(error.value, RuntimeError)


def test_hoja_requerida_base(legacy_falso):
    pytest.importorskip("yaml")
    assert hoja_requerida_base(legacy_falso) == "ACME"


def test_nombres_a_ocultar_desde_rutas(legacy_falso):
    pytest.importorskip("yaml")
    rutas = [
        Path("x/Planilla_ACME_Consolidado.parquet"),
        Path("x/Planilla_Otra_Consolidado_Silver.parquet"),
    ]
    assert nombres_a_ocultar(legacy_falso, rutas) == {"ACME", "Otra"}


def test_ocultar_ignora_mayusculas():
    assert ocultar("hoja 'Acme' falta", {"ACME"}) == "hoja '<CLIENTE>' falta"


def test_ocultar_no_toca_texto_sin_nombres():
    assert ocultar("todo bien", set()) == "todo bien"


def test_entorno_subproceso():
    env = entorno_subproceso()
    assert env["PYTHONIOENCODING"] == "utf-8"
    assert env["PYTHONUTF8"] == "1"
    assert env["PYTHONDONTWRITEBYTECODE"] == "1"
