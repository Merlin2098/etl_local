"""Fixtures golden: salidas del legacy con nombres canónicos y un resumen.

El resumen compara las salidas del legacy con el manifiesto del generador demo.
"""

import json
import platform
import shutil
import tempfile
from datetime import datetime
from pathlib import Path

from generador_demo.cli import generar
from generador_demo.parametros import ParametrosGeneracion
from paridad_legacy.ejecutar import aplicar_hoja_requerida, correr_legacy, validar_con_legacy
from paridad_legacy.entorno import nombres_a_ocultar, ocultar

MARCADOR = ".generar-golden"
MODOS = ("limpio", "sucio", "casos")
ETL_POR_CASO = {
    "CASO-01": "nomina",
    "CASO-02": "nomina",
    "CASO-03": "nomina",
    "CASO-04": "nomina",
    "CASO-05": "bd",
    "CASO-06": "licencias",
}
NOTA_EDAD = (
    "El legacy calcula la edad con CURRENT_DATE (TR-57), no con la fecha de corte; "
    "el resultado depende de fecha_ejecucion."
)


DEPENDENCIAS_DE_FECHA = {
    "base_personal_empleados_flags": ["cumple_65_*", "cumple_70_*", "tiempo_servicio_texto"],
}
NOTA_FECHA = (
    "Estas columnas usan CURRENT_DATE en el legacy (TR-57). tiempo_servicio_texto cambia "
    "cada mes para contratos sin fecha de término; las alertas de edad cambian cada año. "
    "Para comparar, usar fecha_ejecucion como CURRENT_DATE."
)


class GoldenIncompleto(RuntimeError):
    """El legacy falló en algún dominio: no se reemplazan los fixtures anteriores."""


def _verificar_destino(destino: Path) -> None:
    """Rechaza una carpeta existente con contenido que no tenga el marcador de generar-golden."""
    if destino.exists() and not (destino / MARCADOR).exists() and any(destino.iterdir()):
        raise FileExistsError(
            f"La carpeta {destino} ya existe y no fue creada por generar-golden; no se toca."
        )


def _publicar(preparada: Path, destino: Path) -> None:
    """Reemplaza `destino` por `preparada` solo al final de una corrida exitosa."""
    nueva = destino.with_name(f".{destino.name}.nuevo")
    if nueva.exists():
        shutil.rmtree(nueva)
    try:
        shutil.copytree(preparada, nueva)
        if destino.exists():
            shutil.rmtree(destino)
        nueva.rename(destino)
    finally:
        if nueva.exists():
            shutil.rmtree(nueva)


def _escribir_json(ruta: Path, datos: dict, nombres: set[str]) -> None:
    texto = json.dumps(datos, ensure_ascii=False, indent=2, sort_keys=True, default=str)
    ruta.write_text(ocultar(texto, nombres) + "\n", encoding="utf-8")


def _comparar(esperado, legacy) -> dict:
    return {"esperado": esperado, "legacy": legacy, "coincide": esperado == legacy}


def _comparacion_manifiesto(
    manifiesto: dict, salidas: dict[str, Path], p: ParametrosGeneracion
) -> dict:
    import polars as pl

    def leer(nombre):
        return pl.read_parquet(salidas[nombre]) if nombre in salidas else None

    gold, empleados, practicantes, flags = (
        leer("planilla_gold"),
        leer("base_personal_empleados_gold"),
        leer("base_personal_practicantes_gold"),
        leer("base_personal_empleados_flags"),
    )
    periodos = manifiesto["periodos"]

    def suma(df, prefijo):
        if df is None:
            return None
        return int(sum(df[c].sum() for c in df.columns if c.startswith(prefijo)))

    filas_legacy = (
        {k: int(v) for k, v in gold.group_by("PERIODO").len().iter_rows()}
        if gold is not None
        else None
    )
    return {
        "filas_planilla_por_periodo": _comparar(
            {k: v["personas_planilla"] for k, v in periodos.items()}, filas_legacy
        ),
        "dias_licencia_con_goce": _comparar(
            sum(v["dias_ausencia"]["dias_licencia_con_goce"] for v in periodos.values()),
            int(gold["DIAS GOCE HABER(D)"].cast(pl.Float64).sum()) if gold is not None else None,
        ),
        "empleados": _comparar(
            manifiesto["base_personal"]["empleados"],
            empleados.height if empleados is not None else None,
        ),
        "practicantes": _comparar(
            manifiesto["base_personal"]["practicantes"],
            practicantes.height if practicantes is not None else None,
        ),
        "alertas_contrato_obra": _comparar(
            p.casos_contrato_obra, suma(flags, "alerta_contrato_obra")
        ),
        "alertas_contrato_incremento": _comparar(
            p.casos_contrato_incremento, suma(flags, "alerta_contrato_incremento")
        ),
        "alertas_jubilacion": {
            **_comparar(
                {"65": p.casos_jubilacion_65, "70": p.casos_jubilacion_70},
                {"65": suma(flags, "cumple_65"), "70": suma(flags, "cumple_70")}
                if flags is not None
                else None,
            ),
            "nota": NOTA_EDAD,
        },
    }


def _versiones() -> dict:
    import duckdb
    import polars

    return {
        "python": platform.python_version(),
        "polars": polars.__version__,
        "duckdb": duckdb.__version__,
    }


def generar_golden(legacy: Path, destino: Path, modo: str, p: ParametrosGeneracion) -> Path:
    if modo not in MODOS:
        raise ValueError(f"Modo desconocido: {modo}")
    salida = Path(destino) / modo
    _verificar_destino(salida)
    nombres = nombres_a_ocultar(legacy)
    base = {
        "modo": modo,
        "semilla": p.semilla,
        "fecha_ejecucion": datetime.now().date().isoformat(),
        "versiones": _versiones(),
    }

    with tempfile.TemporaryDirectory(prefix="golden-") as tmp:
        tmp = Path(tmp)
        preparada = tmp / "salida"
        preparada.mkdir()
        (preparada / MARCADOR).write_text("Carpeta creada por generar-golden.\n", encoding="utf-8")
        datos = generar(p, tmp / "datos", modo)

        if modo == "casos":
            for caso, etl in ETL_POR_CASO.items():
                archivo = next((datos / caso).glob("*.xlsx"))
                if etl == "bd":
                    # Solo se quiere el defecto plantado (nombre de archivo), no el de la hoja.
                    aplicar_hoja_requerida(legacy, archivo)
                errores = validar_con_legacy(legacy, archivo, etl, tmp / "trabajo" / caso)
                _escribir_json(
                    preparada / f"{caso}.json",
                    {"caso": caso, "etl": etl, "errores": errores},
                    nombres,
                )
            _escribir_json(preparada / "resumen.json", base, nombres)
        else:
            resultados = correr_legacy(legacy, datos, tmp / "trabajo")
            fallidos = [r for r in resultados if not r.exito]
            if fallidos:
                detalle = "; ".join(f"{r.dominio} ({r.error})" for r in fallidos)
                raise GoldenIncompleto(
                    ocultar(f"El legacy falló en: {detalle}. Se conservan los fixtures.", nombres)
                )
            salidas = {k: v for r in resultados for k, v in r.salidas.items()}
            nombres |= nombres_a_ocultar(legacy, salidas.values())
            import polars as pl

            archivos = {}
            for nombre, ruta in sorted(salidas.items()):
                shutil.copy2(ruta, preparada / f"{nombre}.parquet")
                df = pl.read_parquet(ruta)
                archivos[nombre] = {"filas": df.height, "columnas": df.columns}

            manifiesto = json.loads((datos / "manifiesto.json").read_text(encoding="utf-8"))
            resumen = {
                **base,
                "dominios": [
                    {"dominio": r.dominio, "exito": r.exito, "error": r.error} for r in resultados
                ],
                "archivos": archivos,
                "comparacion_manifiesto": _comparacion_manifiesto(manifiesto, salidas, p),
                "dependencias_de_fecha": {**DEPENDENCIAS_DE_FECHA, "nota": NOTA_FECHA},
            }
            _escribir_json(preparada / "resumen.json", resumen, nombres)

        _publicar(preparada, salida)
    return salida
