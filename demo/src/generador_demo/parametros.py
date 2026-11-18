"""Parámetros de generación y utilidades de periodos (YYYY-MM)."""

import calendar
from dataclasses import dataclass
from datetime import date


@dataclass(frozen=True)
class ParametrosGeneracion:
    semilla: int = 42
    periodo_fin: str = "2025-11"
    meses: int = 6
    personas_iniciales: int = 1200
    entradas_por_mes: tuple[int, int] = (10, 30)
    salidas_por_mes: tuple[int, int] = (8, 25)
    practicantes: int = 60
    personas_regimen_especial: int = 40
    personas_sin_base: int = 8
    dia_corte_base: int = 12
    casos_contrato_obra: int = 11
    casos_contrato_incremento: int = 7
    casos_jubilacion_65: int = 9
    casos_jubilacion_70: int = 4
    indice_inicio_subsidio_maternidad: int = 2

    @property
    def casos_contrato_plazo(self) -> int:
        return self.casos_contrato_obra + self.casos_contrato_incremento

    @property
    def casos_jubilacion(self) -> int:
        return self.casos_jubilacion_65 + self.casos_jubilacion_70


def _anio_mes(periodo: str) -> tuple[int, int]:
    anio, mes = periodo.split("-")
    return int(anio), int(mes)


def inicio_mes(periodo: str) -> date:
    anio, mes = _anio_mes(periodo)
    return date(anio, mes, 1)


def fin_mes(periodo: str) -> date:
    anio, mes = _anio_mes(periodo)
    return date(anio, mes, calendar.monthrange(anio, mes)[1])


def dias_del_mes(periodo: str) -> int:
    return fin_mes(periodo).day


def periodos(p: ParametrosGeneracion) -> list[str]:
    """`p.meses` periodos consecutivos que terminan en `p.periodo_fin`."""
    anio, mes = _anio_mes(p.periodo_fin)
    resultado = []
    for _ in range(p.meses):
        resultado.append(f"{anio:04d}-{mes:02d}")
        mes -= 1
        if mes == 0:
            anio, mes = anio - 1, 12
    return list(reversed(resultado))
