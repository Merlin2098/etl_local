import json

import pytest

from generador_demo.parametros import ParametrosGeneracion
from paridad_legacy.entorno import paridad_disponible, ubicar_legacy

pytestmark = pytest.mark.paridad


def test_legacy_ausente(tmp_path, capsys):
    from paridad_legacy.cli import main

    assert main(["--legacy", str(tmp_path), "--salida", str(tmp_path / "out")]) == 2
    assert "RUTA_LEGACY" in capsys.readouterr().out


DISPONIBLE, MOTIVO = paridad_disponible()
LEGACY = ubicar_legacy() if DISPONIBLE else None

requiere_legacy = pytest.mark.skipif(not DISPONIBLE, reason=MOTIVO or "paridad no disponible")
P = ParametrosGeneracion(meses=3)


@pytest.fixture(scope="module")
def golden(tmp_path_factory):
    from paridad_legacy.golden import generar_golden

    destino = tmp_path_factory.mktemp("golden")
    return {modo: generar_golden(LEGACY, destino, modo, P) for modo in ("limpio", "casos")}


def _resumen(carpeta):
    return json.loads((carpeta / "resumen.json").read_text(encoding="utf-8"))


@requiere_legacy
def test_archivos_canonicos(golden):
    from paridad_legacy.ejecutar import SALIDAS_LEGACY

    nombres = sorted(p.stem for p in golden["limpio"].glob("*.parquet"))
    assert nombres == sorted(SALIDAS_LEGACY)


@requiere_legacy
def test_limpio_coincide_con_manifiesto(golden):
    comparacion = _resumen(golden["limpio"])["comparacion_manifiesto"]
    for clave in (
        "filas_planilla_por_periodo",
        "dias_licencia_con_goce",
        "empleados",
        "practicantes",
        "alertas_contrato_obra",
        "alertas_contrato_incremento",
    ):
        assert comparacion[clave]["coincide"], (clave, comparacion[clave])


@requiere_legacy
def test_alertas_de_edad_registran_fecha(golden):
    resumen = _resumen(golden["limpio"])
    assert resumen["fecha_ejecucion"]
    assert "alertas_jubilacion" in resumen["comparacion_manifiesto"]
    assert set(resumen["versiones"]) == {"python", "polars", "duckdb"}
    assert all(d["exito"] for d in resumen["dominios"])


@requiere_legacy
def test_sin_nombre_del_cliente(golden):
    import polars as pl

    from paridad_legacy.entorno import nombres_a_ocultar

    nombres = {n.lower() for n in nombres_a_ocultar(LEGACY)}
    for carpeta in golden.values():
        for ruta in carpeta.rglob("*"):
            if not ruta.is_file():
                continue
            assert not any(n in ruta.name.lower() for n in nombres), ruta.name
            if ruta.suffix == ".json":
                texto = ruta.read_text(encoding="utf-8").lower()
                assert not any(n in texto for n in nombres), ruta
            if ruta.suffix == ".parquet":
                df = pl.read_parquet(ruta)
                for col in df.columns:
                    assert not any(n in col.lower() for n in nombres)
                    if df[col].dtype == pl.String:
                        valores = " ".join(v for v in df[col].drop_nulls().unique()).lower()
                        assert not any(n in valores for n in nombres), (ruta.name, col)


@requiere_legacy
def test_casos_registran_errores(golden):
    carpeta = golden["casos"]
    for caso in ("CASO-01", "CASO-02", "CASO-03", "CASO-05", "CASO-06"):
        datos = json.loads((carpeta / f"{caso}.json").read_text(encoding="utf-8"))
        assert datos["errores"], caso
    caso_04 = json.loads((carpeta / "CASO-04.json").read_text(encoding="utf-8"))
    assert caso_04["etl"] == "nomina"
    assert caso_04["errores"] == []  # brecha del legacy: no valida el nombre (NOM-BRZ-05)


@requiere_legacy
def test_segunda_corrida_reemplaza(tmp_path):
    from paridad_legacy.golden import generar_golden

    destino = generar_golden(LEGACY, tmp_path, "casos", P)
    (destino / "resto_viejo.json").write_text("{}", encoding="utf-8")
    destino = generar_golden(LEGACY, tmp_path, "casos", P)
    assert not (destino / "resto_viejo.json").exists()


@requiere_legacy
def test_no_borra_carpetas_ajenas(tmp_path):
    from paridad_legacy.golden import generar_golden

    ajena = tmp_path / "casos"
    ajena.mkdir()
    (ajena / "mio.txt").write_text("x", encoding="utf-8")
    with pytest.raises(FileExistsError):
        generar_golden(LEGACY, tmp_path, "casos", P)
    assert (ajena / "mio.txt").exists()


@requiere_legacy
def test_caso_05_solo_error_de_nombre(golden):
    datos = json.loads((golden["casos"] / "CASO-05.json").read_text(encoding="utf-8"))
    assert len(datos["errores"]) == 1
    assert "Filename does not match regex" in datos["errores"][0]


@requiere_legacy
def test_resumen_declara_dependencias_de_fecha(golden):
    dependencias = _resumen(golden["limpio"])["dependencias_de_fecha"]
    assert set(dependencias["base_personal_empleados_flags"]) == {
        "tiempo_servicio_texto",
        "cumple_65_*",
        "cumple_70_*",
    }
