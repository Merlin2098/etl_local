"""Utilidades comunes de escritura de Excel."""

import io
import re
import zipfile
from datetime import datetime
from pathlib import Path

from openpyxl import Workbook

FECHA_FIJA = datetime(2025, 1, 1)
_FECHA_ZIP = (1980, 1, 1, 0, 0, 0)
_W3CDTF = "2025-01-01T00:00:00Z"
FORMATO_FECHA = "dd/mm/yyyy"


def guardar_libro(wb: Workbook, ruta: Path) -> Path:
    """Guarda el libro de forma determinista: misma entrada ⇒ mismos bytes.

    openpyxl estampa la hora actual en las propiedades del documento y en cada
    entrada del zip; ambas se fijan aquí.
    """
    wb.properties.created = FECHA_FIJA
    wb.properties.modified = FECHA_FIJA
    wb.properties.creator = "generador-demo"
    wb.properties.lastModifiedBy = "generador-demo"
    buffer = io.BytesIO()
    wb.save(buffer)
    buffer.seek(0)

    ruta.parent.mkdir(parents=True, exist_ok=True)
    with (
        zipfile.ZipFile(buffer) as origen,
        zipfile.ZipFile(ruta, "w", zipfile.ZIP_DEFLATED) as destino,
    ):
        for info in origen.infolist():
            datos = origen.read(info.filename)
            if info.filename == "docProps/core.xml":
                texto = datos.decode("utf-8")
                texto = re.sub(
                    r"(<dcterms:(?:created|modified)[^>]*>)[^<]*(</dcterms:(?:created|modified)>)",
                    rf"\g<1>{_W3CDTF}\g<2>",
                    texto,
                )
                datos = texto.encode("utf-8")
            nueva = zipfile.ZipInfo(info.filename, date_time=_FECHA_ZIP)
            nueva.compress_type = zipfile.ZIP_DEFLATED
            nueva.external_attr = info.external_attr
            destino.writestr(nueva, datos)
    return ruta
