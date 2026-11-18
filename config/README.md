# Configuración del motor

| Archivo | Qué es |
|---|---|
| `fuentes/<fuente>.yaml` | **Contrato de fuente** ([ADR-0003](../docs/adr/0003-formato-contrato-fuente.md)): nombre de archivo, hojas, celda de cabecera y columnas, con su nombre canónico y su tipo. Un archivo por fuente; el nombre del archivo es el de la fuente. |
| `parametros.yaml` | Umbrales de negocio versionados (TR-D10): jubilación, contratos, practicantes, tipos de licencia, subsidios y **metas** del tablero (opcional, `IND-0N: número`). |
| `sql/<etapa>/` | SQL de DuckDB de Silver, Gold y controles (Plan 4). Se ejecuta en orden de nombre. |
| `indicadores.yaml` | Catálogo del tablero (Plan 5): nombre, pregunta, unidad, sentido de "mejor", fuente, IDs de regla y etiquetas de IND-01…08. Solo metadatos. |
| `reportes.yaml` | Reportes que ofrece la app de escritorio (Plan 6): título, descripción, contrato principal, complementos, Excel que abre «Abrir en Excel» y concordancia. Cada `fuente` debe tener contrato en `fuentes/`. |
| `sql/indicadores/` | Fórmulas de IND-01…08 e IND-C1/IND-C3 ([indicadores-tablero.md §7](../docs/logica-negocio/indicadores-tablero.md)). `vacias/` define las tablas vacías que se usan cuando falta un Parquet. |
| `esquema-contrato.json` | JSON Schema de los contratos, **generado** desde los tipos de `crates/core`. Lo usan los editores para autocompletar y marcar errores. |

Los contratos incluidos describen el **layout demo** (`demo/`). Para usar archivos de otra empresa, se ajustan
`origen`, `nombre` de hoja y `cabecera`, sin cambiar los nombres canónicos.

## Agregar un reporte (molde, Plan 8)

Planilla régimen especial, Altas de personal, Control de practicantes y Exámenes médicos de salida son reportes
del **molde**: no tienen código propio en Rust ni en la app. Para agregar otro:

1. **Contrato** `fuentes/<fuente>.yaml`. Si `archivo.fecha_en_nombre` es `AAAA-MM`, los archivos se combinan por
   mes; si es `DD.MM.AAAA`, es la fecha de corte; sin fecha, el archivo nuevo reemplaza al guardado.
2. **SQL** `sql/<fuente>/*.sql` que lea `<fuente>_lectura` y cree `<fuente>_gold`. La tabla de lectura trae las
   columnas del contrato (todas las hojas unidas por nombre), más `hoja`, `archivo_origen`, `fila_origen` y
   `periodo` o `fecha_corte` según el nombre del archivo. También existe `centros_costo` (vacía si no se cargó la
   base de personal). Avisos opcionales en `sql/controles/<fuente>_*.sql`, con las 6 columnas de todo control.
3. **Entrada** en `reportes.yaml` con `id`, `fuente` y `excel` iguales a `<fuente>`.

El motor exporta `gold/<fuente>.parquet` y `excel/<fuente>.xlsx`. Estos reportes no alimentan el tablero.

## Probar un contrato

```bash
cargo run -p etl-local-cli -- validar --fuente planilla "ruta/Planilla 2025-11 Empleados.xlsx"
```

El programa no arranca si un contrato tiene una clave desconocida o una regla inválida, y dice cuál.

## Regenerar el esquema

Si cambian los tipos de `crates/core/src/contrato.rs`:

```bash
ACTUALIZAR_ESQUEMA=1 cargo test -p etl-local-core --test contratos_config esquema_contrato_al_dia
```

Un test falla mientras el esquema guardado no coincida con el generado.
