# Plan 8 — Dominios restantes: diseño

> Fecha: 2026-11-17 · Estado: **aprobado** (2026-11-17) · implementado (Plan 8, 2026-11-18) · Opción elegida: **A (molde común, sin paridad con el legacy)**

## 1. Intención

Sumar a la app los cuatro reportes que faltan del legacy para que la versión open source cubra todo el
proceso de RR. HH., **rescatando lo mínimo indispensable** y no la fidelidad al legacy. El usuario lo pidió
así: «no tiene que ser 100% fiel al legacy… agilizar el desarrollo y reducir la complejidad».

| Reporte (UI) | Fuente canónica | Legacy | Reglas |
|---|---|---|---|
| Planilla régimen especial | `planilla_regimen_especial` | Nómina régimen minero | [nomina-regimen-minero.md](../../logica-negocio/nomina-regimen-minero.md) |
| Altas de personal | `altas_personal` | Relación de ingresos (PDT) | [pdt-relacion-ingresos.md](../../logica-negocio/pdt-relacion-ingresos.md) |
| Control de practicantes | `practicantes` | Control de practicantes | [control-practicantes.md](../../logica-negocio/control-practicantes.md) |
| Exámenes médicos de salida | `examenes_salida` | Exámenes de retiro | [examen-retiro.md](../../logica-negocio/examen-retiro.md) |

## 2. Objetivos, no objetivos y anti-objetivos

**Objetivos**

- Cada reporte nuevo aparece en Inicio de la app, se revisa, se procesa y abre su Excel de revisión, igual
  que planilla y base de personal (`config/reportes.yaml` ya preveía esto: «la UI no cambia»).
- Un **molde común**: agregar un reporte = un contrato en `config/fuentes/`, una carpeta `config/sql/<fuente>/`
  y una entrada en `config/reportes.yaml`. Sin código Rust por dominio.
- El generador de `demo/` produce un Excel de ejemplo por dominio y su manifiesto, y los tests del motor
  comparan contra ese manifiesto.

**No objetivos**

- Paridad con el legacy (no hay `generar-golden` ni fixtures de estos dominios).
- Cambios en el tablero: ni indicadores nuevos ni pestañas nuevas. Los reportes nuevos **no** entran en
  IND-01…08 (tampoco la planilla de régimen especial; ver §5.1).
- Catálogo de centros de costo «antiguo» (`CC_OLD`): se descarta.
- Histórico propio por dominio: se usa el mismo `historico/<marca>/` de toda la salida.
- Pantallas nuevas en la app.

**Anti-objetivos** (sería un fracaso aunque funcione)

- Que un dominio nuevo obligue a tocar Rust o Svelte para existir.
- Montos de dinero en un Gold (TR-D22).
- Que un archivo de un dominio nuevo con errores impida procesar los demás reportes **sin decir por qué**
  (hoy la regla es «si un archivo tiene errores no se publica nada»; se mantiene, con su mensaje).

## 3. Restricciones

- Las de siempre (AGENTS.md): sin nombre del cliente, sin datos reales, todo en español con identificadores
  ASCII, sin commits sin pedido, directrices de compilación de la hoja de ruta (`-j 2`, nunca `cargo clean`,
  release solo al final y solo).
- Formato de contrato de ADR-0003 sin cambios. Si el molde necesita algo nuevo en el contrato, se propone
  como cambio aditivo y se pregunta antes.
- Sin dependencias nuevas.

## 4. Enfoques considerados

| | Enfoque | Costo | Elegido |
|---|---|---|---|
| A | Molde común (contrato + SQL + Excel), sin paridad con el legacy, sin tablero | Bajo por dominio; algunas reglas finas del legacy se pierden, cada una anotada | **Sí** |
| B | Igual que A, pero solo con los dominios que alguien usa hoy | Menos trabajo, menos cobertura | No |
| C | Saltar el Plan 8 y publicar con tres reportes | El más rápido; producto incompleto | No |

## 5. Diseño

### 5.1 El molde («reporte simple»)

Hoy `proceso::procesar_con` conoce tres fuentes fijas (`Entradas { planillas, licencias, base_personal }`).
Se agrega un camino genérico para **cualquier otra fuente**:

1. `Entradas` gana un mapa `otras: BTreeMap<String, Vec<PathBuf>>` (fuente → archivos). `entradas/<fuente>/`
   guarda los archivos igual que hoy; los archivos nuevos de una fuente reemplazan a los guardados de esa
   fuente (los de varios periodos se combinan por periodo, como la planilla).
2. Lectura: el contrato de la fuente (preflight y conversión existentes). Cada hoja leída se carga en
   `<fuente>_lectura`, unidas por nombre, con las columnas constantes `hoja`, `archivo_origen`,
   `fila_origen` y, si el contrato tiene `fecha_en_nombre`, `periodo` o `fecha_corte`.
3. SQL: se ejecuta `config/sql/<fuente>/*.sql` en orden; debe dejar `<fuente>_gold`.
4. Se exporta `gold/<fuente>.parquet` y `excel/<fuente>.xlsx`. Etapa `<fuente>` en `ejecucion.json`.
5. Controles: los de `config/sql/controles/<fuente>_*.sql`, con la misma forma de 6 columnas.
6. El orden es: licencias, planilla, base de personal, **otras** (así `centros_costo` ya existe para
   exámenes de salida).
7. Avance de la app: un solo paso nuevo, `otros_reportes` («Ordenando los demás reportes»), para no tocar la
   lista de pasos por dominio.

La planilla de régimen especial usa este mismo molde, como **reporte aparte** (su Gold y su Excel), aunque
su formato se parece al de la planilla estándar. Así no se toca la combinación por periodo de la planilla
ni sus controles, y el tablero sigue igual. Sumarla a los indicadores queda como trabajo futuro (sería unir
su Gold al de la planilla con una columna `regimen`).

### 5.2 Reglas por dominio (lo mínimo)

**Planilla régimen especial** (`planilla_regimen_especial`)
- Archivo: uno por mes, `periodo` en el nombre (patrón del layout demo), hoja `Planilla`, cabecera `A6`,
  clave `numero_documento`; las filas de totales se descartan como en la planilla (corrige RM-DIF-01).
- Gold: `periodo, anio, mes, nombre_mes` (en español), documento, sexo, fecha de nacimiento, puesto, sede,
  área, centro de costo, fecha de ingreso, días y horas (trabajados, descanso médico, licencias, horas
  extra en número de horas). **Sin** montos (`TOTAL INGRESOS`, `DESCUENTOS`, `APORTACIONES`, `NETO`,
  importes de horas extra) por TR-D22. Sin nombres (como el legacy, RM-GLD-03).
- Sin enriquecimiento con licencias.

**Altas de personal** (`altas_personal`)
- Archivo: uno por mes, `periodo` en el nombre; hojas `EMPLEADOS` y `PRACTICANTES`.
- Gold único con `tipo` (`empleado` / `practicante`, desde la hoja) y las columnas comunes: periodo,
  documento, nombres, sexo, fecha de nacimiento, cargo, área, sede, fecha de inicio; más
  `modalidad_contrato` (empleados) y `universidad` (practicantes). Sin `PROYECTO` ni `CODIGO SAP`.
- Sin imputaciones de `SERVICIO`/`REGIMEN` ni «POR DEFINIR» (se deja nulo).

**Control de practicantes** (`practicantes`)
- Archivo con `fecha_corte` en el nombre (como la base de personal); hoja `Practicantes`, cabecera `A4`.
- Gold: las columnas del legacy (CP-GLD-02) más el tiempo de servicio y las tres alertas, **con las
  fórmulas del legacy**: `D = fecha de renovación − fecha de ingreso`; `por_cumplir_1` (335–364 días),
  `cumplio_1` (≥ 365), `por_cumplir_2` (640–729), solo para la condición de practicante profesional
  (comparada normalizada, TR-D24). Los umbrales van en `parametros.yaml` (TR-D10).
- `tiempo_servicio` en texto, con singular correcto («1 mes»).
- Sin renovación o con `D` negativo: tiempo y alertas nulos, con un aviso por fila.

**Exámenes médicos de salida** (`examenes_salida`)
- Archivo sin fecha en el nombre; hoja `DATA`, cabecera `A3`, clave `numero_documento`.
- Gold: nombre, documento, fecha de cese, causa de salida, cargo, código de centro de costo, `anio`, `mes`,
  `nombre_mes` (español). Se excluyen los practicantes (cargo comparado normalizado contra un valor de
  `parametros.yaml`).
- Cruce con `centros_costo` de la base de personal **de la misma ejecución** (código comparado con trim y
  mayúsculas): nombre, categoría y gerencia del centro de costo, y `estado_cruce`
  (`ok` / `sin_codigo` / `codigo_no_encontrado`). Sin base de personal cargada: `estado_cruce =
  sin_catalogo` y un aviso.

### 5.3 Generador de datos demo

- Desde la misma población del Plan 1: altas = entradas de cada mes; exámenes de salida = salidas del
  periodo; practicantes = los practicantes de la base, con fechas de renovación que caen en cada ventana;
  régimen especial = un grupo pequeño propio (~50 personas) que no está en la planilla estándar.
- Solo el modo `limpio`, más **un** caso sucio por dominio que el preflight debe atrapar.
- El manifiesto suma, por dominio, las cifras que los tests verifican (filas del Gold, conteos de cada
  alerta de practicantes, conteo por `estado_cruce`).

### 5.4 App y documentos

- `config/reportes.yaml`: cuatro entradas nuevas. Los servicios de la app (`reconocer`, `revisar`,
  `procesar`, resultado de «Listo») pasan de los tres campos fijos a la fuente del reporte; para los
  reportes del molde, «Listo» cuenta las filas de `gold/<fuente>.parquet`.
- Cada documento de `docs/logica-negocio/` de estos dominios recibe una sección corta «Decisiones del
  rework (Plan 8)» que dice qué pregunta abierta se resolvió y cómo. `reglas-transversales.md` §5 registra
  TR-D9 como decidido (reporte aparte, sin indicadores).

## 6. Qué queda para quien construya

- Los nombres canónicos exactos de las columnas (siguiendo `vocabulario.md`) y los patrones de nombre de
  archivo del layout demo.
- El texto exacto de avisos nuevos y del paso `otros_reportes`.
- Cómo partir el plan en tareas (sugerencia: molde con un dominio de prueba → los otros tres → app → docs).

## 7. Decisiones que tomé yo (revisables)

1. La planilla de régimen especial es un reporte aparte y **no** entra en los indicadores del tablero.
2. Practicantes conserva la fórmula del legacy (renovación − ingreso), no la fecha de corte.
3. Exámenes de salida usa solo el catálogo de centros de costo de la base de personal.
4. Altas de personal sale como un solo Gold con columna `tipo` (el legacy tenía dos).
5. Meses en español en todos los Gold (como la planilla del Plan 4).
6. Solo modo `limpio` en el demo, más un caso sucio por dominio.
