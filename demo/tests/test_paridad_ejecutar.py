import pytest

from generador_demo.cli import generar
from generador_demo.parametros import ParametrosGeneracion
from paridad_legacy.entorno import paridad_disponible, ubicar_legacy

pytestmark = pytest.mark.paridad

DISPONIBLE, MOTIVO = paridad_disponible()
if not DISPONIBLE:
    pytest.skip(MOTIVO, allow_module_level=True)
LEGACY = ubicar_legacy()

from paridad_legacy.ejecutar import SALIDAS_LEGACY, correr_legacy  # noqa: E402


def _instantanea(raiz):
    return {
        str(p.relative_to(raiz)): (p.stat().st_size, p.stat().st_mtime_ns)
        for p in raiz.rglob("*")
        if p.is_file() and ".git" not in p.relative_to(raiz).parts
    }


@pytest.fixture(scope="module")
def corrida(tmp_path_factory):
    base = tmp_path_factory.mktemp("paridad")
    datos = generar(ParametrosGeneracion(meses=3), base / "datos", "limpio")
    antes = _instantanea(LEGACY)
    trabajo = base / "trabajo"
    resultados = correr_legacy(LEGACY, datos, trabajo)
    despues = _instantanea(LEGACY)
    return resultados, trabajo, antes, despues


def test_ambos_dominios_exitosos(corrida):
    resultados, *_ = corrida
    assert [r.dominio for r in resultados] == ["planilla", "base_personal"]
    assert all(r.exito for r in resultados), [(r.dominio, r.error) for r in resultados]


def test_salidas_completas(corrida):
    resultados, *_ = corrida
    salidas = {k: v for r in resultados for k, v in r.salidas.items()}
    assert sorted(salidas) == sorted(SALIDAS_LEGACY)
    assert all(ruta.exists() for ruta in salidas.values())


def test_legacy_intacto(corrida):
    _, _, antes, despues = corrida
    assert antes == despues


def test_logs_en_trabajo(corrida):
    _, trabajo, *_ = corrida
    assert (trabajo / "planilla.log").exists()
    assert (trabajo / "base_personal.log").exists()
    assert (trabajo / "logs").is_dir()
