"""Writer de la planilla mensual con el layout legacy (NOM-BRZ-04…11).

Hoja `Planilla`, título en A1:A3, cabecera en la fila 6 desde la columna A, datos desde
la fila 7 y dos filas de totales al final.
"""

from collections.abc import Callable
from datetime import date
from pathlib import Path

from openpyxl import Workbook

from generador_demo.excel_comun import FORMATO_FECHA, guardar_libro
from generador_demo.parametros import ParametrosGeneracion, fin_mes
from generador_demo.poblacion import Persona, Poblacion
from generador_demo.registros import RegistroMes

Extractor = Callable[[Persona, RegistroMes], object]

FILA_CABECERA = 6
COLUMNA_DINAMICA_MATERNIDAD = "DIAS SUB. MATERNIDAD"
FILAS_TOTALES = ("+ TOTAL INGRESOS", "- TOTAL DESCUENTOS")


def _salida(x: Persona, r: RegistroMes) -> date | None:
    """La fecha de salida solo se informa en la planilla del mes en que ocurre."""
    if x.fecha_salida is not None and x.fecha_salida <= fin_mes(r.periodo):
        return x.fecha_salida
    return None


def _motivo(x: Persona, r: RegistroMes) -> str | None:
    return x.motivo_salida if _salida(x, r) is not None else None


def _monto_dias(dias: int, x: Persona) -> float:
    return round(dias * x.remuneracion_mensual / 30, 2)


def _monto_horas(horas: float, pct: float, x: Persona) -> float:
    return round(horas * x.remuneracion_mensual / 240 * (1 + pct), 2)


COLUMNAS_PLANILLA: list[tuple[str, Extractor]] = [
    ("CÓDIGO CORPORATIVO", lambda x, r: x.codigo_empleado),
    ("APELLIDO PATERNO", lambda x, r: x.apellido_paterno),
    ("APELLIDO MATERNO", lambda x, r: x.apellido_materno),
    ("NOMBRES", lambda x, r: x.nombres),
    ("TIPO DE DOCUMENTO", lambda x, r: x.tipo_documento),
    ("DNI/CEX", lambda x, r: x.numero_documento),
    ("SEXO", lambda x, r: x.sexo),
    ("F. NACIMIENTO", lambda x, r: x.fecha_nacimiento),
    ("ÁREA", lambda x, r: x.area),
    ("CENTRO DE COSTOS", lambda x, r: x.centro_costo),
    ("FECHA DE INGRESO", lambda x, r: x.fecha_entrada),
    ("FECHA CESE", _salida),
    ("MOTIVO CESE", _motivo),
    ("DIAS SUBSIDIO ENFERMEDAD", lambda x, r: r.dias_subsidio_enfermedad),
    ("DIAS GOCE HABER(D)", lambda x, r: r.dias_licencia_con_goce),
    ("# HRS EXTRAS SIMP.25%", lambda x, r: r.horas_extra_25),
    ("# HRS EXTRAS SIMP.35%", lambda x, r: r.horas_extra_35),
    ("#HRS EXTRAS DOBLES", lambda x, r: r.horas_extra_100),
    ("#HRS SIMP.50%", lambda x, r: r.horas_extra_50),
    ("HRS SIMPLES AL 50%", lambda x, r: r.horas_extra_50),
    ("HORAS EXTRAS 25%", lambda x, r: _monto_horas(r.horas_extra_25, 0.25, x)),
    ("HORAS EXTRAS 35%", lambda x, r: _monto_horas(r.horas_extra_35, 0.35, x)),
    ("HORAS EXTRAS 100%", lambda x, r: _monto_horas(r.horas_extra_100, 1.0, x)),
    ("LICENCIA GOCE HABER", lambda x, r: _monto_dias(r.dias_licencia_con_goce, x)),
    ("LICENCIA PATERNIDAD", lambda x, r: _monto_dias(r.dias_licencia_paternidad, x)),
    ("DIAS DIST.LIC.S.GH", lambda x, r: r.dias_licencia_sin_goce),
    ("DIAS LIC. PATE (P)", lambda x, r: r.dias_licencia_paternidad),
    ("DIAS DESC MEDICO (P)", lambda x, r: r.dias_descanso_medico),
    ("DESCANSO MEDICO", lambda x, r: _monto_dias(r.dias_descanso_medico, x)),
    ("SUBS. POR ENFERMEDAD", lambda x, r: _monto_dias(r.dias_subsidio_enfermedad, x)),
    ("SUBS. POR ACCIDENTE", lambda x, r: 0.0),
    ("SUBSIDIO MATERNIDAD", lambda x, r: _monto_dias(r.dias_subsidio_maternidad, x)),
]


def escribir_planilla(
    pob: Poblacion,
    registros: list[RegistroMes],
    p: ParametrosGeneracion,
    periodo: str,
    carpeta: Path,
) -> Path:
    columnas = list(COLUMNAS_PLANILLA)
    if pob.periodos.index(periodo) >= p.indice_inicio_subsidio_maternidad:
        columnas.append((COLUMNA_DINAMICA_MATERNIDAD, lambda x, r: r.dias_subsidio_maternidad))

    por_doc = {x.numero_documento: x for x in pob.personas}
    del_mes = sorted(
        (r for r in registros if r.periodo == periodo), key=lambda r: r.numero_documento
    )

    wb = Workbook()
    hoja = wb.active
    hoja.title = "Planilla"
    hoja["A1"] = "PLANILLA DE REMUNERACIONES"
    hoja["A2"] = f"Periodo: {periodo}"
    hoja["A3"] = "Empresa: EMPRESA DEMO S.A.C."
    for j, (nombre, _) in enumerate(columnas, start=1):
        hoja.cell(row=FILA_CABECERA, column=j, value=nombre)

    fila = FILA_CABECERA + 1
    for r in del_mes:
        persona = por_doc[r.numero_documento]
        for j, (_, extraer) in enumerate(columnas, start=1):
            celda = hoja.cell(row=fila, column=j, value=extraer(persona, r))
            if isinstance(celda.value, date):
                celda.number_format = FORMATO_FECHA
        fila += 1
    for texto in FILAS_TOTALES:
        hoja.cell(row=fila, column=1, value=texto)
        fila += 1

    return guardar_libro(wb, carpeta / f"Planilla {periodo} Empleados.xlsx")
