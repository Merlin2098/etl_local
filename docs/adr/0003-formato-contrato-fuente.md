# ADR-0003: Formato unificado de contrato de fuente

- **Estado:** Aceptado
- **Fecha:** 2026-11-16
- **Relacionado:** [ADR-0001](0001-stack-rust-tauri.md), [ADR-0002](0002-demo-python-uv.md),
  [validacion-preflight.md](../logica-negocio/validacion-preflight.md),
  [vocabulario.md](../logica-negocio/vocabulario.md),
  [reglas-transversales.md §5](../logica-negocio/reglas-transversales.md#5-decisiones-pendientes-para-el-rework)

## Contexto

En el legacy, lo que se sabe de un archivo fuente está repartido en tres lugares que no coinciden:

1. **El contrato de preflight** (`validate_source/<id>.yaml`): nombre de archivo, hojas, celda de cabecera y
   columnas mínimas. Las columnas se listan a mano y anulan en silencio al esquema que dicen usar
   ([validacion-preflight.md §7.1](../logica-negocio/validacion-preflight.md#7-limitaciones-y-propuestas-para-el-rework)).
2. **El esquema** (`esquemas/esquema_<dominio>.yaml`): tipos, nulos y valores permitidos. Hay unas 7 formas
   distintas (`schema:` como mapa, `columns:` como lista, `hojas.<h>.schema`, `column_mapping`…).
3. **El código de cada step**: cómo encontrar el fin de los datos, qué columnas renombrar, cómo convertir fechas.
   El preflight y los steps normalizan las cabeceras de forma distinta, lo que produce falsos "pasa" (§7.3–7.4).

Los Planes 1 y 2 aportaron casos concretos que el formato nuevo debe cubrir:

- **CASO-04:** el legacy no exige que la planilla traiga el periodo en el nombre (NOM-BRZ-05).
- **SUC-03:** fechas escritas como texto se vuelven nulas sin aviso y una alerta de contrato se pierde (BD-DIF-03).
- **CASO-01:** la cabecera desplazada una fila solo produce "found empty/shifted" (TR-D13 pide decir dónde está).
- **TR-57:** la fecha de referencia debe salir de la fuente, no del reloj (TR-D3).

ADR-0001 ya fijó YAML leído con `serde`. Falta decidir **qué** se escribe en ese YAML.

## Decisión

### 1. Un archivo por fuente, una sola fuente de verdad

Cada fuente (`planilla`, `base_personal`, `licencias`…, nombres de [vocabulario.md §1](../logica-negocio/vocabulario.md#1-reportes-fuentes))
tiene **un único contrato** en `config/fuentes/<fuente>.yaml`. Ese archivo reemplaza al contrato de
preflight, al esquema y al mapeo de columnas del legacy. De él salen:

- la validación previa (preflight);
- la lectura del Excel y el renombrado a nombres canónicos;
- los tipos de la capa Silver;
- la verificación del generador demo: los tests de `crates/core` validan sus Excel contra estos contratos
  (el generador no los lee; ver la nota de 2026-11-16 en ADR-0002).

Las reglas que cruzan fuentes o calculan indicadores **no** van aquí: viven en SQL (`config/sql/`) y en los
*rule packs* (TR-D10).

### 2. Estructura

```yaml
version: 1                       # versión del formato; el motor rechaza versiones desconocidas
fuente: planilla                 # nombre canónico
titulo: Planilla mensual         # lo que ve el usuario

archivo:
  extensiones: [xlsx, xlsm, xls]
  patron_nombre: '(?i)Planilla (?P<periodo>\d{4}-\d{2}).*\.(xlsx|xlsm|xls)'   # coincidencia COMPLETA
  fecha_en_nombre: { grupo: periodo, formato: AAAA-MM }   # opcional; AAAA-MM | DD.MM.AAAA (TR-D3)

hojas:
  - nombre: Planilla             # comparación exacta; admite lista de alternativas
    cabecera: A6
    fin_datos: fin_hoja          # fin_hoja (por defecto) | fila_vacia | clave_vacia (TR-D2)
    columnas_extra: ignorar      # ignorar | conservar
    columnas:
      - nombre: numero_documento
        origen: ['DNI/CEX']      # una o más cabeceras posibles en la fuente
        tipo: documento
        requerida: true          # la cabecera debe existir
        nula: false              # cada fila debe tener valor
        clave: true              # fila sin clave = no es dato: se descarta (con aviso si tenía otros datos)
      - nombre: fecha_entrada
        origen: ['FECHA DE INGRESO']
        tipo: fecha
        formatos: [excel, DD/MM/AAAA]
      - nombre: dias_licencia_con_goce
        origen: ['DIAS GOCE HABER(D)']
        tipo: entero
        minimo: 0
      - nombre: sexo
        origen: ['SEXO']
        tipo: texto
        valores: [M, F]
    columnas_dinamicas:
      - prefijo: dias_subsidio_
        patron_origen: '(?i)DIAS SUB\.\s*(?P<motivo>.+)'   # columnas que aparecen solo algunos meses
        tipo: entero
```

Reglas del formato:

- **Tipos cerrados:** `texto`, `entero`, `decimal`, `fecha`, `periodo`, `documento`, `booleano`. `documento`
  aplica la limpieza de TR-55 y LIC-SLV-02 (número guardado como número, `.0` al final, espacios).
- **Nombres canónicos** en `snake_case` ASCII (AGENTS.md §5). El motor los rechaza si no cumplen.
- **Claves desconocidas son error** (`deny_unknown_fields`): una errata no se ignora en silencio.
- **Valores por defecto** documentados en el tipo Rust: `requerida: false`, `nula: true`, `columnas_extra: ignorar`,
  `fin_datos: fin_hoja`.
- **Fin de datos:** con `fin_hoja` se leen todas las filas y se descartan las que tienen vacía la columna `clave`
  (como NOM-BRZ-09 y LIC-BRZ-04; una fila vacía a mitad de los datos no corta la lectura). Si la fila descartada
  tenía otros datos, o la clave trae `#N/A`/`nan`, queda un aviso PRE-24 (TR-D20, TR-D21).
- **Fecha del nombre:** el valor de `fecha_en_nombre` se informa como `periodo` (AAAA-MM) o `fecha_corte`
  (DD.MM.AAAA) de esa fuente; una fecha imposible (`31.02.2025`) es error (BD-BRZ-03).

### 3. Normalización de cabeceras: una sola, fija

Preflight y lectura usan **la misma función** (TR-D1): pasar a mayúsculas, quitar tildes, colapsar espacios y
quitar los de los extremos. Así, `Condición`, `CONDICION` y `CONDICIÓN ` son la misma cabecera. La
puntuación se conserva (`N°` ≠ `N`). No es configurable por contrato: una sola regla es más fácil de explicar.

- Si dos entradas de `origen` de una misma hoja quedan iguales tras normalizar, **el contrato es inválido**.
- Si la fuente trae dos cabeceras que quedan iguales, es error con las dos celdas.

> Diferencia con el legacy: el legacy distingue tildes (PRE-12). Quitarlas solo acepta más archivos válidos;
> no cambia ningún resultado de la paridad.

### 4. Errores con código, celda y texto en español

Cada problema se informa con:

| Campo | Ejemplo |
|---|---|
| `codigo` | `PRE-09` (ID de regla de `docs/logica-negocio/`, TR-D17) |
| `severidad` | `error` o `aviso` (TR-D5) |
| `archivo`, `hoja`, `celda` | `Planilla 2025-11 Empleados.xlsx`, `Planilla`, `A6` |
| `mensaje` | "La cabecera debería empezar en A6, pero está en A7. ¿Se insertó una fila?" |

- Todos los problemas se acumulan; solo se detiene la lectura de un archivo si no se puede abrir.
- **Cabecera desplazada (TR-D13):** si la celda esperada no coincide, se buscan las columnas requeridas en
  ±3 filas y ±3 columnas para decir dónde están. El archivo igual se rechaza.
- **Conversión fallida = aviso con celda, nunca nulo silencioso.** Una fecha `31/02/2025` o un texto en una
  columna `entero` produce un aviso con la celda (corrige BD-DIF-03 / SUC-03). Si la columna es `nula: false`,
  es error.

### 5. Validación del propio contrato

- Los contratos se cargan y validan **al arrancar** (CLI y app): versión, tipos, regex que compilan, nombres
  canónicos válidos y `origen` sin colisiones. Un contrato inválido detiene el programa con un mensaje claro;
  no se descubre a mitad de un proceso.
- Un test de `crates/core` carga todos los contratos de `config/fuentes/`.
- Los tipos Rust generan un **JSON Schema** (`config/esquema-contrato.json`) para autocompletado en editores.
  Se versiona y un test verifica que esté al día.

### 6. Qué queda fuera de este ADR

- **Perfiles de planilla** (estándar / régimen especial, TR-D9): se decide en el Plan 8, con el dominio a la vista.
  Si hace falta, se agrega `hereda: planilla` en una `version: 2`.
- **Contratos del usuario:** el repositorio trae contratos para el layout demo. Cómo una empresa adapta los suyos
  sin tocar el repositorio (carpeta de configuración local) se decide con la app (Plan 6).
- **Fuentes no Excel** (Parquet/CSV, §7.7): `archivo.formato` se agregará cuando un dominio lo necesite.

## Alternativas consideradas

| Alternativa | Motivo del descarte |
|---|---|
| Mantener contrato y esquema separados | Es el origen de la doble verdad del legacy (§7.1) |
| Conservar los YAML legacy y adaptarlos | 7 formatos distintos, nombres del cliente en hojas y columnas, claves muertas |
| JSON Schema como formato principal | Muy verboso para que una persona de RR. HH. lo lea; se genera desde Rust en su lugar |
| TOML | Las listas de columnas con varias propiedades quedan ilegibles |
| Normalización configurable por contrato | Más casos que probar y explicar sin un caso real que lo pida |
| Regex por búsqueda (como el legacy) | Acepta coincidencias parciales sin querer (§7.9) |

## Consecuencias

- **Positivas:**
  - Un solo lugar para entender un archivo fuente.
  - Preflight y lectura no pueden discrepar.
  - Los mensajes dicen hoja y celda.
  - Las conversiones fallidas ya no se pierden en silencio.
  - El generador demo y el motor comparten la misma descripción del layout.
- **Negativas:**
  - Hay que reescribir los 7 contratos del legacy (se hace por dominio, en los Planes 3 y 8).
  - Quitar tildes y avisar en conversiones fallidas cambia el comportamiento respecto al legacy. Esto es
    deliberado y se registra como corrección (TR-D11), no como falla de paridad.
- **Plan 3 (completado 2026-11-16):** contratos de `planilla`, `base_personal` y `licencias` en este formato,
  validados contra los casos de `demo/` (CASO-01…06 y SUC-01…06). La comparación con los fixtures del Plan 2 queda
  para el Plan 4, con las capas Silver y Gold.
- **Dependencias Rust previstas** (se confirman al escribir el Plan 3):
  - `serde` con un lector YAML mantenido, porque `serde_yaml` está archivado;
  - `regex`;
  - `schemars` para el JSON Schema.

## Referencias

- [validacion-preflight.md](../logica-negocio/validacion-preflight.md) §2, §4, §7
- [reglas-transversales.md](../logica-negocio/reglas-transversales.md): TR-D1, TR-D2, TR-D3, TR-D5, TR-D9, TR-D11, TR-D13, TR-D17
- [vocabulario.md](../logica-negocio/vocabulario.md)
- Hallazgos de paridad: [demo/README.md](../../demo/README.md#paridad-con-el-legacy-generar-golden)

## Notas

- **2026-11-16 (Plan 4):** se agrega a la hoja el campo opcional `filas_resumen` (lista de textos), un cambio
  compatible con la versión 1. Una fila cuya primera celda, sin `+`, `-`, `*` ni espacios al inicio, contiene alguno
  de esos textos (comparación normalizada) es una fila de totales y se descarta sin aviso (NOM-BRZ-10). Una primera
  celda que no es texto nunca es fila de totales. Un texto vacío hace inválido el contrato.
