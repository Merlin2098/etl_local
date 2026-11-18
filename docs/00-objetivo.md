# 00 — Objetivo del proyecto

> Estado: borrador inicial · Fecha: 2026-11-14

## 1. Contexto

`HR_Data_Integration_Pipeline` (carpeta [`../HR_Data_Integration_Pipeline/`](../HR_Data_Integration_Pipeline/)) es una aplicación de escritorio en Python
(PySide6 + Polars + DuckDB) que construí para el área de Recursos Humanos de una empresa.
Consolida archivos Excel operativos (planillas, base de datos de personal, licencias,
practicantes, exámenes de retiro, relación de ingresos) en capas **Bronze → Silver → Gold**
y deja Parquet listos para que Power BI los consuma.

Este repositorio (`etl_local`) existe para **rescatar y rehacer** ese proyecto.

## 2. Objetivo

Convertir el pipeline legacy en un **producto open source, moderno y autocontenido** que:

1. **Conserve el valor de negocio** del legacy: validación preflight de fuentes Excel,
   consolidación multi-archivo, capas Bronze/Silver/Gold, reglas de negocio en SQL,
   salidas Parquet estables (`actual/`) e históricas.
2. **Se reescriba en un stack compilado y moderno**, con binario único y liviano:
   - **Opción A:** Go + [Wails](https://wails.io/)
   - **Opción B:** Rust + [Tauri](https://tauri.app/)
   (La decisión está abierta; ver [§5](#5-decisión-de-stack-pendiente)).
3. **Reemplace Power BI por un dashboard propio** servido en `localhost`, interactivo,
   que simule la experiencia de un tablero de BI (filtros, KPIs, drill‑down, tablas),
   sin requerir licencias.
4. **Sea publicable**: sin datos reales, sin referencias al cliente, con datos sintéticos
   de demo, documentación en abierto y licencia OSS.

## 3. Alcance

### Dentro del alcance

| Área | Qué incluye |
|---|---|
| Ingesta | Lectura de Excel (`.xlsx/.xlsm/.xls`), cabeceras desplazadas, múltiples hojas, consolidación de N archivos por periodo |
| Validación | Contratos de fuente declarativos (hojas, fila/columna de cabecera, columnas requeridas, regex de nombre de archivo); *fail‑fast* |
| Transformación | Capas Bronze/Silver/Gold; reglas de negocio en SQL (DuckDB); columnas dinámicas (p. ej. subsidios) |
| Salidas | Parquet (`actual/` estable + `historico/` versionado); Excel opcional para revisión |
| Orquestación | Pipelines declarativos (YAML) por dominio; ejecución independiente por módulo |
| UI de escritorio | Selección de archivos, progreso por etapa, logs, errores legibles para usuarios no técnicos |
| Dashboard | Front web en `localhost` que lee las salidas Gold y muestra KPIs/gráficos interactivos |
| Datos demo | Generador de datos sintéticos que respete los contratos de cada dominio |

### Fuera del alcance (por ahora)

- Migración a la nube (el ADR‑0005 del legacy queda como referencia, no como meta).
- Multiusuario, autenticación, servidor central.
- Paridad visual con Power BI; se busca la *experiencia* de BI, no un clon.
- Conectores a ERPs/sistemas de nómina reales.

## 4. Principios

1. **Configuración sobre código.** Contratos, esquemas y reglas viven en archivos
   declarativos versionables; el motor es genérico.
2. **SQL como lenguaje de negocio.** Las reglas Gold se expresan en SQL de DuckDB, portable
   entre Go y Rust (ambos embeben DuckDB).
3. **Un solo formato de esquema.** El legacy tiene ~7 formas distintas de YAML de esquema;
   el rework unifica en un único formato validado.
4. **Local‑first y offline.** Todo corre en la máquina del usuario; nada sale de ella.
5. **Agnóstico del cliente.** Reglas específicas de país/empresa (p. ej. normativa laboral
   peruana) se empaquetan como *rule packs* configurables, no como código duro.
6. **Demo reproducible.** `clonar → generar datos sintéticos → correr → ver dashboard`
   en minutos.

## 5. Decisión de stack (pendiente)

> **Resuelto (2026-11-14):** Rust + Tauri. Ver [ADR-0001](adr/0001-stack-rust-tauri.md). La tabla
> siguiente se conserva como registro del análisis.

Ambas opciones comparten la arquitectura: núcleo ETL nativo + DuckDB embebido + frontend
web (que sirve tanto para la UI de escritorio como para el dashboard).

| Criterio | Go + Wails | Rust + Tauri |
|---|---|---|
| Lectura de Excel | `excelize` (lectura/escritura muy completa) | `calamine` (lectura muy rápida), `rust_xlsxwriter` |
| Motor tabular | DuckDB (`duckdb-go`, vía cgo) | DuckDB (`duckdb-rs`) **y** Polars nativo (mismo motor que el legacy) |
| Reutilizar SQL del legacy | Sí (DuckDB) | Sí (DuckDB) |
| Curva de aprendizaje | Baja | Alta |
| Velocidad de iteración | Alta | Media |
| Tamaño/rendimiento del binario | Bueno | Muy bueno |
| Cross‑compile con DuckDB | Requiere toolchain C (cgo) | Requiere toolchain C (bundled) |
| Ecosistema desktop | Wails v2 estable, v3 en evolución | Tauri v2 estable, plugins maduros |

La recomendación preliminar está en
[01-analisis-legacy.md §9](01-analisis-legacy.md#9-implicancias-para-el-rework). La decisión
final se registrará como ADR en `docs/adr/`.

## 6. Dashboard en localhost

Sustituto de Power BI. Requisitos mínimos:

- Se abre desde la app (botón *“Ver dashboard”*) o directamente en `http://localhost:<puerto>`.
- Lee las salidas **Gold** (`actual/*.parquet`) a través de DuckDB; no duplica datos.
- Filtros globales (periodo, gerencia/área, sede, tipo de trabajador, modalidad de contrato).
- Páginas sugeridas a partir de los datos que ya produce el legacy:
  - **Headcount y dotación** (BD de personal): activos, altas/bajas, white/blue collar, sexo, sede.
  - **Alertas de contrato y edad**: flags de vencimiento de contrato, 65/70 años.
  - **Ausentismo**: licencias con/sin goce, descansos médicos, subsidios (nómina + licencias).
  - **Horas extra**: por tipo (25 %, 35 %, 100 %), por área y periodo.
  - **Practicantes**: tiempo de servicio, próximos a cumplir 1/2 años.
  - **Salidas**: exámenes médicos de salida, motivo de salida, por centro de costo.
  - Nombres de reportes, columnas e indicadores: ver [logica-negocio/vocabulario.md](logica-negocio/vocabulario.md).
- Interacción tipo BI: *cross‑filtering* entre gráficos, tooltips, tablas exportables a CSV.
- Requisitos agregados tras revisar los mocks (2026-11-16, ver [diseno/](diseno/README.md)):
  - indicadores en **tasa** (rotación %, ausentismo %) con comparación contra el mes anterior y metas configurables;
  - barra **"Datos usados"** con la fecha de corte de cada fuente y un aviso de periodos mezclados;
  - botón **"¿De dónde sale?"** en cada número: fuente, fecha de corte e IDs de regla;
  - página **"Cómo leer este tablero"**: definiciones y qué filtros afectan a cada gráfico;
  - alertas en dos niveles: **riesgo agregado** para dirección y **detalle nominal** solo para RR. HH.;
  - **resumen ejecutivo en PDF** de una página.

Opciones técnicas (a decidir junto con el stack): API local del binario que ejecuta
consultas DuckDB, o DuckDB‑WASM en el navegador leyendo los Parquet directamente.

## 7. Criterios de éxito

- [ ] Los 6 dominios del legacy (+ licencias) procesan datos sintéticos con resultados
      equivalentes al legacy (pruebas de paridad sobre los mismos inputs).
- [ ] Binario único para Windows (y idealmente macOS/Linux) sin requerir Python.
- [ ] Dashboard funcional en `localhost` con al menos 4 de las páginas anteriores.
- [ ] Repo público sin datos reales ni referencias al cliente, con historial git limpio.
- [ ] README con *quick start* de demo en menos de 5 minutos.

## 8. Documentos relacionados

- [01 — Análisis del proyecto legacy](01-analisis-legacy.md)
- ADRs del legacy: [`../HR_Data_Integration_Pipeline/docs/adr/`](../HR_Data_Integration_Pipeline/docs/adr/README.md)
