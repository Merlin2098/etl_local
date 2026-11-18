"""Manifiesto: verdad de referencia de lo generado, calculada desde la población.

Sirve para probar que el motor calcula los indicadores (IND-01…08) sin depender del
legacy: lo que el motor lea de los Excel debe cuadrar con estos números.
"""

import json
from pathlib import Path

from generador_demo.dominios import (
    altas_del_mes,
    examenes_salida,
    practicantes_control,
    resumen_examenes,
    resumen_practicantes,
)
from generador_demo.parametros import ParametrosGeneracion, fin_mes, inicio_mes
from generador_demo.poblacion import Poblacion
from generador_demo.registros import CAMPOS_DIAS, CAMPOS_HORAS, RegistroMes


def construir_manifiesto(
    pob: Poblacion, registros: list[RegistroMes], p: ParametrosGeneracion, modo: str
) -> dict:
    docs_base = {x.numero_documento for x in pob.en_base_personal()}
    periodos: dict[str, dict] = {}
    for periodo in pob.periodos:
        ini, fin = inicio_mes(periodo), fin_mes(periodo)
        planilla = pob.en_planilla(periodo)
        del_mes = [r for r in registros if r.periodo == periodo]
        empleados = [x for x in pob.personas if not x.es_practicante]
        periodos[periodo] = {
            "personas_planilla": len(planilla),
            "entradas": sum(ini <= x.fecha_entrada <= fin for x in empleados),
            "salidas": sum(
                x.fecha_salida is not None and ini <= x.fecha_salida <= fin for x in empleados
            ),
            "personas_sin_base": sum(x.numero_documento not in docs_base for x in planilla),
            "dias_ausencia": {c: sum(getattr(r, c) for r in del_mes) for c in CAMPOS_DIAS},
            "horas_extra": {c: round(sum(getattr(r, c) for r in del_mes), 2) for c in CAMPOS_HORAS},
        }
    base = pob.en_base_personal()
    altas = {}
    for periodo in pob.periodos:
        empleados, practicantes = altas_del_mes(pob, periodo)
        altas[periodo] = {"empleados": len(empleados), "practicantes": len(practicantes)}
    return {
        # Plan 8: dominios restantes.
        "planilla_regimen_especial": {per: len(pob.regimen_especial) for per in pob.periodos},
        "altas_personal": altas,
        "practicantes": resumen_practicantes(practicantes_control(pob, p)),
        "examenes_salida": resumen_examenes(pob, examenes_salida(pob)),
        "semilla": p.semilla,
        "modo": modo,
        "periodos": periodos,
        "base_personal": {
            "fecha_corte": pob.fecha_corte_base.isoformat(),
            "empleados": sum(not x.es_practicante for x in base),
            "practicantes": sum(x.es_practicante for x in base),
            "casos_contrato_plazo": p.casos_contrato_plazo,
            "casos_jubilacion": p.casos_jubilacion,
        },
    }


def escribir_manifiesto(manifiesto: dict, carpeta: Path) -> Path:
    ruta = carpeta / "manifiesto.json"
    ruta.write_text(
        json.dumps(manifiesto, ensure_ascii=False, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    return ruta
