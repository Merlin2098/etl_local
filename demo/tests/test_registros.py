from statistics import mean

from generador_demo.parametros import ParametrosGeneracion
from generador_demo.poblacion import simular_poblacion
from generador_demo.registros import CAMPOS_DIAS, CAMPOS_HORAS, dias_activos, generar_registros

P = ParametrosGeneracion()
POB = simular_poblacion(P)
REG = generar_registros(POB, P)
POR_DOC = {x.numero_documento: x for x in POB.personas}


def test_un_registro_por_persona_en_planilla():
    for periodo in POB.periodos:
        esperados = {x.numero_documento for x in POB.en_planilla(periodo)}
        obtenidos = [r.numero_documento for r in REG if r.periodo == periodo]
        assert len(obtenidos) == len(set(obtenidos))
        assert set(obtenidos) == esperados


def test_dias_no_superan_mes_ni_dias_activos():
    p = ParametrosGeneracion(periodo_fin="2025-03", meses=3)  # incluye febrero
    pob = simular_poblacion(p)
    por_doc = {x.numero_documento: x for x in pob.personas}
    for r in generar_registros(pob, p):
        total = sum(getattr(r, c) for c in CAMPOS_DIAS)
        assert all(getattr(r, c) >= 0 for c in CAMPOS_DIAS)
        assert total <= dias_activos(por_doc[r.numero_documento], r.periodo)


def test_dias_activos_febrero():
    p = ParametrosGeneracion(periodo_fin="2025-03", meses=3)
    pob = simular_poblacion(p)
    antiguo = next(x for x in pob.en_planilla("2025-02") if x.fecha_salida is None)
    assert dias_activos(antiguo, "2025-02") == 28


def test_licencias_con_motivo():
    for r in REG:
        assert (r.dias_licencia_con_goce > 0) == (r.motivo_licencia_con_goce is not None)
        assert (r.dias_licencia_sin_goce > 0) == (r.motivo_licencia_sin_goce is not None)
    assert any(r.dias_licencia_con_goce > 0 for r in REG)
    assert any(r.dias_licencia_sin_goce > 0 for r in REG)


def test_maternidad_y_paternidad_por_sexo():
    for r in REG:
        sexo = POR_DOC[r.numero_documento].sexo
        if r.dias_subsidio_maternidad > 0:
            assert sexo == "F"
        if r.dias_licencia_paternidad > 0:
            assert sexo == "M"
    assert any(r.dias_licencia_paternidad > 0 for r in REG)


def test_subsidio_maternidad_desde_indice():
    tardios = POB.periodos[P.indice_inicio_subsidio_maternidad :]
    for r in REG:
        if r.periodo not in tardios:
            assert r.dias_subsidio_maternidad == 0
    assert any(r.dias_subsidio_maternidad > 0 for r in REG if r.periodo == POB.periodos[-1])


def test_todos_los_motivos_aparecen():
    for campo in CAMPOS_DIAS:
        assert any(getattr(r, campo) > 0 for r in REG), campo


def test_horas_extra_operativos():
    def promedio(tipo):
        return mean(
            sum(getattr(r, c) for c in CAMPOS_HORAS)
            for r in REG
            if POR_DOC[r.numero_documento].tipo_trabajador == tipo
        )

    assert promedio("BLUE COLLAR") > promedio("WHITE COLLAR")


def test_determinista():
    assert generar_registros(POB, P) == REG
