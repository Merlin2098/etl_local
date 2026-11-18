"""Simulación determinista de la población de personal a lo largo de los periodos.

Todo usa nombres canónicos (docs/logica-negocio/vocabulario.md); los nombres legacy
solo aparecen en los writers de Excel.
"""

import random
from dataclasses import dataclass, field, replace
from datetime import date, timedelta

from faker import Faker

from generador_demo.parametros import ParametrosGeneracion, fin_mes, inicio_mes, periodos

OBRA = "CONTRATO POR OBRA O SERVICIO ESPECIFICO"
INCREMENTO = "CONTRATO POR INCREMENTO DE ACTIVIDAD"
INDETERMINADO = "PLAZO INDETERMINADO"
CONVENIO = "TERMINO DE CONVENIO"

MOTIVOS_SALIDA = ["RENUNCIA", "FIN DE CONTRATO", "DESPIDO", "JUBILACION"]

GERENCIAS = {
    "Operaciones": (48, ["Producción", "Logística", "Planificación"]),
    "Mantenimiento": (23, ["Mantenimiento mecánico", "Mantenimiento eléctrico"]),
    "Administración": (11, ["Finanzas", "Recursos humanos", "Contabilidad"]),
    "Comercial": (8, ["Ventas", "Atención al cliente"]),
    "Seguridad": (6, ["Seguridad industrial", "Salud ocupacional"]),
    "Tecnología": (4, ["Sistemas", "Soporte técnico"]),
}
GERENCIAS_OPERATIVAS = {"Operaciones", "Mantenimiento", "Seguridad"}
SEDES = (["Sede Central", "Planta Norte", "Planta Sur"], [30, 45, 25])
SERVICIOS = ["Producción", "Soporte", "Gestión"]
REGIMENES = ["5x2", "14x7", "4x3"]

ANIO = 365.25


@dataclass(frozen=True)
class Persona:
    numero_documento: str
    tipo_documento: str
    codigo_empleado: str
    apellido_paterno: str
    apellido_materno: str
    nombres: str
    sexo: str
    fecha_nacimiento: date
    gerencia: str
    area: str
    centro_costo: str
    nombre_centro_costo: str
    categoria_centro_costo: str
    sede: str
    tipo_trabajador: str
    modalidad_contrato: str
    servicio: str
    regimen_trabajo: str
    remuneracion_mensual: float
    fecha_entrada: date
    fecha_fin_contrato: date | None
    fecha_salida: date | None
    motivo_salida: str | None
    es_practicante: bool
    en_base_personal: bool


@dataclass(frozen=True)
class Poblacion:
    periodos: list[str]
    personas: list[Persona]
    fecha_corte_base: date
    # Plan 8: personal de la planilla de régimen especial; no está en `personas` ni en la planilla estándar.
    regimen_especial: list[Persona] = field(default_factory=list)

    def en_planilla(self, periodo: str) -> list[Persona]:
        ini, fin = inicio_mes(periodo), fin_mes(periodo)
        return [
            x
            for x in self.personas
            if not x.es_practicante
            and x.fecha_entrada <= fin
            and (x.fecha_salida is None or x.fecha_salida >= ini)
        ]

    def en_base_personal(self) -> list[Persona]:
        corte = self.fecha_corte_base
        return [
            x
            for x in self.personas
            if x.en_base_personal
            and x.fecha_entrada <= corte
            and (x.fecha_salida is None or x.fecha_salida >= corte)
        ]


class _Fabrica:
    """Crea personas con datos sintéticos a partir de un único generador aleatorio."""

    def __init__(self, p: ParametrosGeneracion, corte: date):
        self.rng = random.Random(p.semilla)
        self.faker = Faker("es_ES")
        self.faker.seed_instance(p.semilla)
        self.corte = corte
        self.contador = 0
        self._codigos_cc: dict[str, str] = {}

    def fecha_entre(self, desde: date, hasta: date) -> date:
        return desde + timedelta(days=self.rng.randint(0, (hasta - desde).days))

    def nacimiento(self, edad_min: int, edad_max: int) -> date:
        anio = self.rng.randint(self.corte.year - edad_max, self.corte.year - edad_min)
        return self.nacimiento_en(anio)

    def nacimiento_en(self, anio: int) -> date:
        return date(anio, 1, 1) + timedelta(days=self.rng.randint(0, 364))

    def _centro_costo(self, area: str) -> str:
        if area not in self._codigos_cc:
            self._codigos_cc[area] = f"CC-{1001 + len(self._codigos_cc)}"
        return self._codigos_cc[area]

    def persona(
        self,
        *,
        modalidad: str,
        fecha_entrada: date,
        fecha_nacimiento: date,
        fecha_fin_contrato: date | None = None,
        es_practicante: bool = False,
    ) -> Persona:
        self.contador += 1
        rng = self.rng
        nombres_g = list(GERENCIAS)
        gerencia = rng.choices(nombres_g, weights=[GERENCIAS[g][0] for g in nombres_g])[0]
        area = rng.choice(GERENCIAS[gerencia][1])
        operativa = gerencia in GERENCIAS_OPERATIVAS
        if es_practicante:
            tipo = "WHITE COLLAR"
        else:
            prob_operativo = 0.85 if operativa else 0.1
            tipo = "BLUE COLLAR" if rng.random() < prob_operativo else "WHITE COLLAR"
        sexo = "M" if rng.random() < 0.7 else "F"
        nombres = self.faker.first_name_male() if sexo == "M" else self.faker.first_name_female()
        base_sueldo = (1500, 4500) if tipo == "BLUE COLLAR" else (2500, 12000)
        if es_practicante:
            base_sueldo = (1025, 1500)
        return Persona(
            numero_documento=f"99{self.contador:06d}",
            tipo_documento="DNI",
            codigo_empleado=f"E{self.contador:05d}",
            apellido_paterno=self.faker.last_name(),
            apellido_materno=self.faker.last_name(),
            nombres=nombres,
            sexo=sexo,
            fecha_nacimiento=fecha_nacimiento,
            gerencia=gerencia,
            area=area,
            centro_costo=self._centro_costo(area),
            nombre_centro_costo=area,
            categoria_centro_costo="PRODUCTIVO" if operativa else "ADMINISTRATIVO",
            sede=rng.choices(SEDES[0], weights=SEDES[1])[0],
            tipo_trabajador=tipo,
            modalidad_contrato=modalidad,
            servicio=rng.choice(SERVICIOS),
            regimen_trabajo=rng.choice(REGIMENES),
            remuneracion_mensual=float(round(rng.uniform(*base_sueldo), -1)),
            fecha_entrada=fecha_entrada,
            fecha_fin_contrato=fecha_fin_contrato,
            fecha_salida=None,
            motivo_salida=None,
            es_practicante=es_practicante,
            en_base_personal=True,
        )

    def fin_contrato(self, modalidad: str, desde: date) -> date | None:
        if modalidad == INDETERMINADO:
            return None
        return desde + timedelta(days=self.rng.randint(30, 180))


def simular_poblacion(p: ParametrosGeneracion) -> Poblacion:
    per = periodos(p)
    fin_anio, fin_mes_num = (int(v) for v in p.periodo_fin.split("-"))
    corte = date(fin_anio, fin_mes_num, p.dia_corte_base)
    fab = _Fabrica(p, corte)
    rng = fab.rng
    primer_dia = inicio_mes(per[0])
    antes_inicio = primer_dia - timedelta(days=1)

    personas: dict[str, Persona] = {}
    protegidos: set[str] = set()

    def agregar(x: Persona, proteger: bool = False) -> None:
        personas[x.numero_documento] = x
        if proteger:
            protegidos.add(x.numero_documento)

    # Casos plantados: contratos por superar su plazo (antigüedad a la fecha de corte).
    for modalidad, n, rango in (
        (OBRA, p.casos_contrato_obra, (4.55, 4.95)),
        (INCREMENTO, p.casos_contrato_incremento, (2.55, 2.95)),
    ):
        for _ in range(n):
            entrada = corte - timedelta(days=int(rng.uniform(*rango) * ANIO))
            agregar(
                fab.persona(
                    modalidad=modalidad,
                    fecha_entrada=entrada,
                    fecha_nacimiento=fab.nacimiento(25, 60),
                    fecha_fin_contrato=fab.fin_contrato(modalidad, corte),
                ),
                proteger=True,
            )

    # Casos plantados: edad de jubilación (este año o el próximo).
    y = corte.year
    anios_jub = [y - 65] * (p.casos_jubilacion_65 - p.casos_jubilacion_65 // 3) + [y - 64] * (
        p.casos_jubilacion_65 // 3
    )
    anios_jub += [y - 70] * (p.casos_jubilacion_70 - p.casos_jubilacion_70 // 2) + [y - 69] * (
        p.casos_jubilacion_70 // 2
    )
    for anio in anios_jub:
        agregar(
            fab.persona(
                modalidad=INDETERMINADO,
                fecha_entrada=fab.fecha_entre(corte - timedelta(days=int(30 * ANIO)), antes_inicio),
                fecha_nacimiento=fab.nacimiento_en(anio),
            ),
            proteger=True,
        )

    # Población inicial restante (sin cumplir por azar ningún criterio de alerta).
    rangos_antiguedad = {
        INDETERMINADO: 20.0,
        OBRA: 4.0,  # < 4,5 años incluso sumando el fin de contrato (máx. 180 días)
        INCREMENTO: 2.0,  # < 2,5 años
    }
    for _ in range(p.personas_iniciales - len(personas)):
        modalidad = rng.choices([INDETERMINADO, OBRA, INCREMENTO], weights=[60, 25, 15])[0]
        desde = corte - timedelta(days=int(rangos_antiguedad[modalidad] * ANIO))
        agregar(
            fab.persona(
                modalidad=modalidad,
                fecha_entrada=fab.fecha_entre(desde, antes_inicio),
                fecha_nacimiento=fab.nacimiento(20, 62),
                fecha_fin_contrato=fab.fin_contrato(modalidad, corte),
            )
        )

    # Movimientos mes a mes.
    ultimo = len(per) - 1
    entradas_ultimo: list[str] = []
    for i, periodo in enumerate(per):
        ini, fin = inicio_mes(periodo), fin_mes(periodo)
        n_salidas = rng.randint(*p.salidas_por_mes)
        n_entradas = rng.randint(*p.entradas_por_mes)

        if i == 0:
            # Caso plantado: entra y sale el mismo mes.
            x = fab.persona(
                modalidad=OBRA,
                fecha_entrada=ini + timedelta(days=2),
                fecha_nacimiento=fab.nacimiento(20, 62),
                fecha_fin_contrato=ini + timedelta(days=19),
            )
            agregar(
                replace(x, fecha_salida=ini + timedelta(days=19), motivo_salida="FIN DE CONTRATO"),
                proteger=True,
            )
            n_salidas -= 1
            n_entradas -= 1

        # En el último periodo, las salidas ocurren desde el día de corte y las entradas hasta
        # él, para que la base de personal coincida con la planilla salvo los "sin base".
        dia_sal_min = p.dia_corte_base if i == ultimo else 1
        dia_ent_max = p.dia_corte_base if i == ultimo else fin.day

        elegibles = sorted(
            doc
            for doc, x in personas.items()
            if not x.es_practicante
            and doc not in protegidos
            and x.fecha_entrada < ini
            and x.fecha_salida is None
        )
        for doc in rng.sample(elegibles, n_salidas):
            salida = ini.replace(day=rng.randint(dia_sal_min, fin.day))
            personas[doc] = replace(
                personas[doc], fecha_salida=salida, motivo_salida=rng.choice(MOTIVOS_SALIDA)
            )

        for _ in range(n_entradas):
            modalidad = rng.choices([OBRA, INCREMENTO, INDETERMINADO], weights=[40, 30, 30])[0]
            entrada = ini.replace(day=rng.randint(1, dia_ent_max))
            x = fab.persona(
                modalidad=modalidad,
                fecha_entrada=entrada,
                fecha_nacimiento=fab.nacimiento(20, 62),
                fecha_fin_contrato=fab.fin_contrato(modalidad, max(entrada, corte)),
            )
            agregar(x)
            if i == ultimo:
                entradas_ultimo.append(x.numero_documento)

    # Personas de la planilla que aún no figuran en la base de personal (IND-C1).
    for doc in rng.sample(entradas_ultimo, p.personas_sin_base):
        personas[doc] = replace(personas[doc], en_base_personal=False)

    # Practicantes: solo en la base de personal, no en la planilla.
    for _ in range(p.practicantes):
        entrada = corte - timedelta(days=rng.randint(30, 700))
        agregar(
            fab.persona(
                modalidad=CONVENIO,
                fecha_entrada=entrada,
                fecha_nacimiento=fab.nacimiento(19, 25),
                fecha_fin_contrato=entrada + timedelta(days=rng.choice([365, 730])),
                es_practicante=True,
            )
        )

    # Plan 8: grupo propio del régimen especial, creado al final para no alterar a las demás personas.
    especiales = [
        fab.persona(
            modalidad=INDETERMINADO,
            fecha_entrada=fab.fecha_entre(corte - timedelta(days=int(10 * ANIO)), antes_inicio),
            fecha_nacimiento=fab.nacimiento(22, 60),
        )
        for _ in range(p.personas_regimen_especial)
    ]

    return Poblacion(
        periodos=per,
        personas=list(personas.values()),
        fecha_corte_base=corte,
        regimen_especial=especiales,
    )
