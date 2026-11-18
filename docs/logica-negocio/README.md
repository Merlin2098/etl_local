# Lógica de negocio del legacy

Especificación **agnóstica de lenguaje** de toda la lógica de negocio de
`HR_Data_Integration_Pipeline`, extraída del código Python para reimplementarla en Go o Rust
sin tener que leer el original.

> Fecha de extracción: 2026-11-14 · Fuente: [`../../HR_Data_Integration_Pipeline/`](../../HR_Data_Integration_Pipeline/)
> El nombre del cliente se sustituye por `<CLIENTE>`. "(inferido)" = deducido del comportamiento
> de librerías, no explícito en el código.

## Documentos

| Documento | Dominio | Reglas | Contenido principal |
|---|---|---:|---|
| [validacion-preflight.md](validacion-preflight.md) | Transversal | 18 | Motor de contratos de fuente: formato, algoritmo, normalización de cabeceras, contratos por dominio |
| [bd.md](bd.md) | Base de datos de personal | 85 | Silver, catálogo de centros de costo, split empleados/practicantes, flags de edad y contrato |
| [nomina.md](nomina.md) | Planilla estándar | 61 | Consolidación mensual, filas de totales, columnas dinámicas de subsidios, constraints, **pipeline nómina + licencias** |
| [nomina-regimen-minero.md](nomina-regimen-minero.md) | Planilla régimen minero | 36 | Diferencias con la planilla estándar, pipeline minero + licencias |
| [licencias.md](licencias.md) | Licencias con/sin goce | 26 | Consolidación de 2 hojas, mapeo de columnas, agregación sobre la nómina |
| [pdt-relacion-ingresos.md](pdt-relacion-ingresos.md) | Relación de ingresos | 82 | Hojas EMPLEADOS/PRACTICANTES, limpieza, imputaciones, Gold por hoja |
| [control-practicantes.md](control-practicantes.md) | Control de practicantes | 30 | Cabeceras combinadas, tiempo de servicio, flags por cumplir 1/2 años |
| [examen-retiro.md](examen-retiro.md) | Exámenes de retiro | 27 | Limpieza, columnas derivadas, join con centros de costo actual/antiguo |
| [reglas-transversales.md](reglas-transversales.md) | Transversal | — | Dependencias entre dominios, patrones comunes, inconsistencias globales y decisiones para el rework |
| [vocabulario.md](vocabulario.md) | Transversal | — | Nombres genéricos (UI y canónicos) para reportes, columnas y alertas, con su origen legacy |
| [indicadores-tablero.md](indicadores-tablero.md) | Tablero (nuevo) | 8 + 3 checks | Set genérico aceptado: dotación, ingresos, ceses, rotación %, ausentismo %, horas extra, riesgos; fuente principal: planilla. No existen en el legacy |

Cada documento de dominio sigue la misma estructura:

1. Propósito
2. Fuentes (Bronze)
3. Bronze → Silver
4. Silver → Gold
5. Reglas de negocio
6. Salidas
7. Validaciones y errores
8. Discrepancias
9. Preguntas abiertas
10. Referencias

## Cómo usar estos documentos

- **Para reimplementar:** las secciones 2–6 de cada dominio son el comportamiento a preservar.
- **Para decidir:** las secciones 8–9 listan bugs y ambigüedades. **No se deben replicar a ciegas.**
  Cada una requiere una decisión explícita: preservar por paridad, corregir o descartar.
  Esas decisiones se registran en [reglas-transversales.md §5](reglas-transversales.md#5-decisiones-pendientes-para-el-rework).
- **Para tests de paridad:** los IDs de regla (`BD-FLG-03`, `NOM-GLD-07`…) sirven como nombres de
  casos de prueba.

## Verificación

Los hallazgos más fuertes de los agentes de extracción se contrastaron contra el código:

- Joins de flags en `bd` que multiplican filas cuando un documento está repetido.
- Fallos de Step 2 en `examen_retiro` que se reportan como éxito.
- `required_columns` explícitas que anulan `schema_path` en preflight.
- Parámetros del pipeline asignados por posición.
- Ausencia de columnas del esquema en el Bronze de `pdt`. Este hallazgo se matizó: la lista de columnas esperadas es solo informativa.

El resto de las reglas cita `archivo:línea` para revisarlas.
