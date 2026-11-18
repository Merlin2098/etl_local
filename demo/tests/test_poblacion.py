from generador_demo.parametros import ParametrosGeneracion, fin_mes, inicio_mes, periodos
from generador_demo.poblacion import simular_poblacion

POB = simular_poblacion(ParametrosGeneracion())


def _en_mes(fecha, periodo):
    return fecha is not None and inicio_mes(periodo) <= fecha <= fin_mes(periodo)


def test_periodos_por_defecto():
    assert periodos(ParametrosGeneracion()) == [
        "2025-06",
        "2025-07",
        "2025-08",
        "2025-09",
        "2025-10",
        "2025-11",
    ]


def test_periodos_cruzan_anio():
    assert periodos(ParametrosGeneracion(periodo_fin="2025-02", meses=3)) == [
        "2024-12",
        "2025-01",
        "2025-02",
    ]


def test_misma_semilla_misma_poblacion():
    assert simular_poblacion(ParametrosGeneracion()) == POB


def test_semilla_distinta_poblacion_distinta():
    assert simular_poblacion(ParametrosGeneracion(semilla=7)) != POB


def test_documentos_unicos_y_ficticios():
    docs = [x.numero_documento for x in POB.personas]
    assert len(docs) == len(set(docs))
    assert all(len(d) == 8 and d.startswith("99") and d.isdigit() for d in docs)


def test_movimientos_coherentes():
    per = POB.periodos
    for i, periodo in enumerate(per):
        actuales = {x.numero_documento for x in POB.en_planilla(periodo)}
        siguientes = (
            {x.numero_documento for x in POB.en_planilla(per[i + 1])} if i + 1 < len(per) else set()
        )
        anteriores = {x.numero_documento for x in POB.en_planilla(per[i - 1])} if i > 0 else set()
        for x in POB.personas:
            if x.es_practicante:
                continue
            if _en_mes(x.fecha_salida, periodo):
                assert x.numero_documento in actuales
                assert x.numero_documento not in siguientes
            if _en_mes(x.fecha_entrada, periodo):
                assert x.numero_documento in actuales
                assert x.numero_documento not in anteriores


def test_hay_entradas_y_salidas_en_cada_periodo():
    p = ParametrosGeneracion()
    for periodo in POB.periodos:
        entradas = [
            x for x in POB.personas if not x.es_practicante and _en_mes(x.fecha_entrada, periodo)
        ]
        salidas = [
            x for x in POB.personas if not x.es_practicante and _en_mes(x.fecha_salida, periodo)
        ]
        assert p.entradas_por_mes[0] <= len(entradas) <= p.entradas_por_mes[1]
        assert p.salidas_por_mes[0] <= len(salidas) <= p.salidas_por_mes[1]


def test_entra_y_sale_mismo_mes():
    casos = [
        x
        for x in POB.personas
        if x.fecha_salida is not None
        and x.fecha_entrada.strftime("%Y-%m") == x.fecha_salida.strftime("%Y-%m")
    ]
    assert casos
    for x in casos:
        periodo = x.fecha_entrada.strftime("%Y-%m")
        assert x in POB.en_planilla(periodo)


def test_casos_plantados_en_base_personal():
    base = POB.en_base_personal()
    corte = POB.fecha_corte_base
    obra = [
        x
        for x in base
        if x.modalidad_contrato == "CONTRATO POR OBRA O SERVICIO ESPECIFICO"
        and (corte - x.fecha_entrada).days / 365.25 >= 4.5
    ]
    incr = [
        x
        for x in base
        if x.modalidad_contrato == "CONTRATO POR INCREMENTO DE ACTIVIDAD"
        and (corte - x.fecha_entrada).days / 365.25 >= 2.5
    ]
    assert (len(obra), len(incr)) == (11, 7)
    jub65 = [x for x in base if x.fecha_nacimiento.year in (corte.year - 65, corte.year + 1 - 65)]
    jub70 = [x for x in base if x.fecha_nacimiento.year in (corte.year - 70, corte.year + 1 - 70)]
    assert (len(jub65), len(jub70)) == (9, 4)


def test_practicantes_y_sin_base():
    assert sum(x.es_practicante for x in POB.personas) == 60
    assert all(
        x.modalidad_contrato == "TERMINO DE CONVENIO" for x in POB.personas if x.es_practicante
    )
    ultimo = POB.en_planilla("2025-11")
    assert all(not x.es_practicante for x in ultimo)
    assert sum(not x.en_base_personal for x in ultimo) == 8


def test_fecha_corte_base():
    assert POB.fecha_corte_base.isoformat() == "2025-11-12"
