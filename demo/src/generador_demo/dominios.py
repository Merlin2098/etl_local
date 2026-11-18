"""Datos derivados de la población para los dominios del Plan 8 (practicantes y exámenes de salida).

Los usan los writers de Excel y el manifiesto, así ambos cuentan exactamente lo mismo. Cada función usa su
propio generador aleatorio con la semilla, para no alterar la simulación de la población.
"""

import random
from dataclasses import dataclass
from datetime import date, timedelta

from generador_demo.parametros import ParametrosGeneracion, fin_mes, inicio_mes
from generador_demo.poblacion import Persona, Poblacion

CARGOS = ["OPERARIO", "TECNICO", "ANALISTA", "SUPERVISOR", "ASISTENTE"]
UNIVERSIDADES = ["Universidad Nacional del Centro", "Universidad del Pacífico Sur", "Instituto Tecnológico Andino"]
PROFESIONAL = "PRACTICANTE PROFESIONAL"
PRE_PROFESIONAL = "PRACTICANTE PRE PROFESIONAL"
CARGO_PRACTICANTE = "STUDENT INTERNAL"
CODIGO_INEXISTENTE = "CC-999"

# Ventanas de días de servicio (renovación − ingreso) de los practicantes profesionales, en orden.
VENTANAS = (("por_cumplir_1", 6, (335, 364)), ("cumplio", 8, (365, 639)), ("por_cumplir_2", 5, (640, 729)))
SIN_RENOVACION = 2
PROFESIONALES = 50


def cargo_de(x: Persona) -> str:
    return CARGOS[int(x.numero_documento) % len(CARGOS)]


def universidad_de(x: Persona) -> str:
    return UNIVERSIDADES[int(x.numero_documento) % len(UNIVERSIDADES)]


def nombre_completo(x: Persona) -> str:
    return f"{x.apellido_paterno} {x.apellido_materno}, {x.nombres}"


@dataclass(frozen=True)
class FilaPracticante:
    persona: Persona
    condicion: str
    renovacion: date | None


def practicantes_control(pob: Poblacion, p: ParametrosGeneracion) -> list[FilaPracticante]:
    """Los practicantes de la base con su condición y su fecha de renovación (CP-FLG-01…06)."""
    rng = random.Random(p.semilla + 8)
    lista = sorted((x for x in pob.en_base_personal() if x.es_practicante), key=lambda x: x.numero_documento)
    filas: list[FilaPracticante] = []
    planes: list[tuple[int, int] | None] = []
    for _, n, rango in VENTANAS:
        planes += [rango] * n
    planes += [None] * SIN_RENOVACION
    planes += [(30, 334)] * (PROFESIONALES - len(planes))
    for i, x in enumerate(lista):
        if i < PROFESIONALES:
            rango = planes[i]
            renovacion = None if rango is None else x.fecha_entrada + timedelta(days=rng.randint(*rango))
            filas.append(FilaPracticante(x, PROFESIONAL, renovacion))
        else:
            filas.append(FilaPracticante(x, PRE_PROFESIONAL, x.fecha_entrada + timedelta(days=rng.randint(30, 800))))
    return filas


def resumen_practicantes(filas: list[FilaPracticante]) -> dict[str, int]:
    dias = [
        (f.renovacion - f.persona.fecha_entrada).days
        for f in filas
        if f.condicion == PROFESIONAL and f.renovacion is not None
    ]
    return {
        "filas": len(filas),
        "por_cumplir_1": sum(335 <= d <= 364 for d in dias),
        "cumplio_1": sum(d >= 365 for d in dias),
        "por_cumplir_2": sum(640 <= d <= 729 for d in dias),
        "sin_renovacion": sum(f.condicion == PROFESIONAL and f.renovacion is None for f in filas),
    }


@dataclass(frozen=True)
class FilaExamen:
    persona: Persona
    cargo: str
    codigo_cc: str | None


def examenes_salida(pob: Poblacion) -> list[FilaExamen]:
    """Ceses dentro de los periodos generados, más 3 practicantes; 2 sin código y 2 con un código inexistente."""
    ini, fin = inicio_mes(pob.periodos[0]), fin_mes(pob.periodos[-1])
    cesados = sorted(
        (x for x in pob.personas if not x.es_practicante and x.fecha_salida and ini <= x.fecha_salida <= fin),
        key=lambda x: (x.fecha_salida, x.numero_documento),
    )
    filas: list[FilaExamen] = []
    for i, x in enumerate(cesados):
        codigo: str | None = x.centro_costo
        if i in (0, 1):
            codigo = None
        elif i in (2, 3):
            codigo = CODIGO_INEXISTENTE
        filas.append(FilaExamen(x, cargo_de(x), codigo))
    practicantes = sorted((x for x in pob.personas if x.es_practicante), key=lambda x: x.numero_documento)[:3]
    for x in practicantes:
        filas.append(FilaExamen(x, CARGO_PRACTICANTE, x.centro_costo))
    return filas


def fecha_cese(f: FilaExamen, pob: Poblacion) -> date:
    """Los practicantes del examen no tienen salida en la población: cesan el último día del último periodo."""
    return f.persona.fecha_salida or fin_mes(pob.periodos[-1])


def resumen_examenes(pob: Poblacion, filas: list[FilaExamen]) -> dict[str, int]:
    catalogo = {x.centro_costo for x in pob.en_base_personal()}
    validas = [f for f in filas if f.cargo != CARGO_PRACTICANTE]
    sin_codigo = sum(f.codigo_cc is None for f in validas)
    ok = sum(f.codigo_cc is not None and f.codigo_cc in catalogo for f in validas)
    return {
        "filas": len(validas),
        "practicantes_excluidos": len(filas) - len(validas),
        "ok": ok,
        "sin_codigo": sin_codigo,
        "codigo_no_encontrado": len(validas) - ok - sin_codigo,
    }


def altas_del_mes(pob: Poblacion, periodo: str) -> tuple[list[Persona], list[Persona]]:
    """Empleados y practicantes con fecha de entrada dentro del mes."""
    ini, fin = inicio_mes(periodo), fin_mes(periodo)
    del_mes = sorted(
        (x for x in pob.personas if ini <= x.fecha_entrada <= fin), key=lambda x: (x.fecha_entrada, x.numero_documento)
    )
    return [x for x in del_mes if not x.es_practicante], [x for x in del_mes if x.es_practicante]
