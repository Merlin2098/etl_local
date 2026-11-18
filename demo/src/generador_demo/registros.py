"""Registros mensuales por persona: días de ausencia por motivo y horas extra."""

import random
from dataclasses import dataclass

from generador_demo.parametros import ParametrosGeneracion, fin_mes, inicio_mes
from generador_demo.poblacion import Persona, Poblacion

CAMPOS_DIAS: tuple[str, ...] = (
    "dias_descanso_medico",
    "dias_licencia_con_goce",
    "dias_licencia_sin_goce",
    "dias_subsidio_enfermedad",
    "dias_subsidio_maternidad",
    "dias_licencia_paternidad",
)
CAMPOS_HORAS: tuple[str, ...] = (
    "horas_extra_25",
    "horas_extra_35",
    "horas_extra_50",
    "horas_extra_100",
)

MOTIVOS_CON_GOCE = ["MATRIMONIO", "FALLECIMIENTO DE FAMILIAR", "CAPACITACION"]
MOTIVOS_SIN_GOCE = ["ASUNTOS PERSONALES", "ESTUDIOS"]

# (campo, probabilidad mensual, rango de días). El orden es también el de recorte.
_AUSENCIAS = (
    ("dias_descanso_medico", 0.08, (1, 5)),
    ("dias_licencia_con_goce", 0.05, (1, 3)),
    ("dias_licencia_sin_goce", 0.02, (1, 5)),
    ("dias_subsidio_enfermedad", 0.015, (5, 20)),
)
_PROB_PATERNIDAD = 0.006
_CASOS_MATERNIDAD = 3
_REPARTO_HORAS = (0.5, 0.3, 0.1, 0.1)  # 25 %, 35 %, 50 %, 100 %


@dataclass(frozen=True)
class RegistroMes:
    numero_documento: str
    periodo: str
    dias_descanso_medico: int
    dias_licencia_con_goce: int
    dias_licencia_sin_goce: int
    dias_licencia_paternidad: int
    dias_subsidio_enfermedad: int
    dias_subsidio_maternidad: int
    horas_extra_25: float
    horas_extra_35: float
    horas_extra_50: float
    horas_extra_100: float
    motivo_licencia_con_goce: str | None
    motivo_licencia_sin_goce: str | None


def dias_activos(persona: Persona, periodo: str) -> int:
    """Días calendario del periodo en que la persona estuvo contratada."""
    desde = max(persona.fecha_entrada, inicio_mes(periodo))
    hasta = fin_mes(periodo)
    if persona.fecha_salida is not None:
        hasta = min(hasta, persona.fecha_salida)
    return max(0, (hasta - desde).days + 1)


def _horas_extra(rng: random.Random, persona: Persona) -> list[float]:
    tope = 20 if persona.tipo_trabajador == "BLUE COLLAR" else 4
    total = rng.uniform(0, tope)
    return [round(total * peso * 2) / 2 for peso in _REPARTO_HORAS]


def generar_registros(pob: Poblacion, p: ParametrosGeneracion) -> list[RegistroMes]:
    rng = random.Random(p.semilla + 1)
    tardios = pob.periodos[p.indice_inicio_subsidio_maternidad :]

    # Casos de maternidad: mujeres activas en todos los periodos con columna dinámica.
    maternidad: set[str] = set()
    if tardios:
        candidatas = sorted(
            {x.numero_documento for x in pob.en_planilla(tardios[0]) if x.sexo == "F"}.intersection(
                *({x.numero_documento for x in pob.en_planilla(t)} for t in tardios)
            )
        )
        maternidad = set(rng.sample(candidatas, min(_CASOS_MATERNIDAD, len(candidatas))))

    registros: list[RegistroMes] = []
    for periodo in pob.periodos:
        for persona in sorted(pob.en_planilla(periodo), key=lambda x: x.numero_documento):
            activos = dias_activos(persona, periodo)
            dias = dict.fromkeys(CAMPOS_DIAS, 0)
            if persona.numero_documento in maternidad and periodo in tardios:
                dias["dias_subsidio_maternidad"] = activos
            else:
                for campo, prob, rango in _AUSENCIAS:
                    if rng.random() < prob:
                        dias[campo] = rng.randint(*rango)
                if persona.sexo == "M" and rng.random() < _PROB_PATERNIDAD:
                    dias["dias_licencia_paternidad"] = rng.randint(1, 10)
                # Recorte en orden de prioridad para no superar los días activos.
                disponible = activos
                for campo in CAMPOS_DIAS:
                    dias[campo] = min(dias[campo], disponible)
                    disponible -= dias[campo]
            h25, h35, h50, h100 = _horas_extra(rng, persona)
            registros.append(
                RegistroMes(
                    numero_documento=persona.numero_documento,
                    periodo=periodo,
                    **dias,
                    horas_extra_25=h25,
                    horas_extra_35=h35,
                    horas_extra_50=h50,
                    horas_extra_100=h100,
                    motivo_licencia_con_goce=(
                        rng.choice(MOTIVOS_CON_GOCE) if dias["dias_licencia_con_goce"] else None
                    ),
                    motivo_licencia_sin_goce=(
                        rng.choice(MOTIVOS_SIN_GOCE) if dias["dias_licencia_sin_goce"] else None
                    ),
                )
            )
    return registros
