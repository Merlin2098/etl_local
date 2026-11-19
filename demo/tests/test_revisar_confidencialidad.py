"""Tests del script de confidencialidad (Plan 9). Usan un nombre ficticio: nunca el real."""

import importlib.util
import sys
from pathlib import Path

import openpyxl

RUTA_SCRIPT = Path(__file__).resolve().parents[2] / "scripts" / "revisar_confidencialidad.py"
NOMBRE = "Empresa Ficticia"


def _modulo():
    spec = importlib.util.spec_from_file_location("revisar_confidencialidad", RUTA_SCRIPT)
    modulo = importlib.util.module_from_spec(spec)
    sys.modules["revisar_confidencialidad"] = modulo
    spec.loader.exec_module(modulo)
    return modulo


def test_encuentra_nombre_sin_mostrarlo(tmp_path):
    rc = _modulo()
    (tmp_path / "nota.md").write_text("linea uno\nreporte de empresa ficticia\n", encoding="utf-8")
    hallazgos = rc.revisar(tmp_path, ["nota.md"], [NOMBRE])
    assert [(h.archivo, h.motivo, h.linea) for h in hallazgos] == [("nota.md", "nombre", 2)]
    assert NOMBRE.lower() not in str(hallazgos[0]).lower()
    assert NOMBRE.lower() not in rc.describir(hallazgos[0]).lower()


def test_revisa_dentro_de_los_xlsx(tmp_path):
    rc = _modulo()
    libro = openpyxl.Workbook()
    libro.active["B3"] = f"Planilla {NOMBRE.upper()} 2025"
    libro.save(tmp_path / "datos.xlsx")
    hallazgos = rc.revisar(tmp_path, ["datos.xlsx"], [NOMBRE])
    assert [(h.archivo, h.motivo) for h in hallazgos] == [("datos.xlsx", "nombre")]


def test_encuentra_correos(tmp_path):
    rc = _modulo()
    (tmp_path / "a.txt").write_text(
        # El correo se arma por partes para que este archivo no sea, él mismo, un hallazgo.
        "contacto: alguien" + "@" + "empresa.pe\nbot: 123+x@users.noreply.github.com\n"
        "demo: ana@example.com\nCo-Authored-By: Claude <noreply@anthropic.com>\n",
        encoding="utf-8",
    )
    hallazgos = rc.revisar(tmp_path, ["a.txt"], [NOMBRE])
    assert [(h.archivo, h.motivo, h.linea) for h in hallazgos] == [("a.txt", "correo", 1)]


def test_limpio_sin_hallazgos(tmp_path):
    rc = _modulo()
    (tmp_path / "ok.rs").write_text("fn main() { println!(\"hola\"); }\n", encoding="utf-8")
    (tmp_path / "logo.png").write_bytes(bytes(range(256)))
    assert rc.revisar(tmp_path, ["ok.rs", "logo.png"], [NOMBRE]) == []


def test_versiones_y_recursos_no_son_correos(tmp_path):
    rc = _modulo()
    (tmp_path / "b.json").write_text(
        '"icons/128x128@2x.png",\nnpm view echarts@6.1.0 version\n', encoding="utf-8"
    )
    assert rc.revisar(tmp_path, ["b.json"], [NOMBRE]) == []


def test_bytes_comprimidos_con_acentos_no_son_correos(tmp_path):
    # Dentro de un video, bytes al azar leídos como latin-1 pueden armar algo como «h@sí.ab». Un correo
    # real es ASCII; un binario con un correo ASCII sigue siendo hallazgo.
    rc = _modulo()
    ruido = bytes([0, 1, 2, 3]) * 64
    (tmp_path / "demo.mp4").write_bytes(ruido + "h@sí.ab".encode("latin-1") + ruido)
    assert rc.revisar(tmp_path, ["demo.mp4"], [NOMBRE]) == []
    (tmp_path / "otro.mp4").write_bytes(ruido + b"alguien" + b"@" + b"empresa.pe" + ruido)
    assert [h.motivo for h in rc.revisar(tmp_path, ["otro.mp4"], [NOMBRE])] == ["correo"]
