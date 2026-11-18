# Plan 3 — Motor: lectura y validación — plan de implementación

> **Para agentes:** SUB-SKILL REQUERIDA: usar superpowers:subagent-driven-development (recomendado) o
> superpowers:executing-plans para implementar este plan tarea por tarea. Los pasos usan casillas (`- [ ]`).

**Objetivo:** un workspace Rust con `crates/core` que carga contratos de fuente en el formato de ADR-0003, valida
archivos Excel con mensajes por hoja y celda, y lee los datos convertidos a nombres y tipos canónicos. También
un CLI `etl-local` con los comandos `validar` y `leer`.

**Arquitectura:**
- `crates/core` (librería `etl_local_core`, sin Tauri ni UI):
  - `normalizar` y `celda`: funciones puras;
  - `contrato`: tipos serde, carga y validación del propio contrato;
  - `informe`: problemas con código, severidad y ubicación;
  - `preflight`: valida archivo, hojas y cabecera;
  - `lectura`: convierte filas a valores tipados.
- `crates/cli` (binario `etl-local`): solo interpreta argumentos y formatea la salida.
- Los contratos de `planilla`, `base_personal` y `licencias` viven en `config/fuentes/`.
- Los tests de integración usan Excel generados por `demo/`, versionados como *fixtures* pequeños.

**Stack:**
- Rust 1.99 (edition 2024), workspace `resolver = "3"`.
- Dependencias de `core`:
  - `calamine` 0.38 (feature `dates`) y `chrono` 0.4;
  - `serde` 1 (derive) y `serde-saphyr` 2.1 como lector YAML (`serde_yaml` está archivado);
  - `regex` 1, `unicode-normalization` 0.1, `thiserror` 2, `schemars` 1.2 y `serde_json` 1.
- Dependencias de `cli`: `clap` 4.6 (derive), `anyhow` 1 y `serde_json` 1.
- Dependencias de desarrollo: `tempfile` 3 y `rust_xlsxwriter` 0.102, para crear Excel pequeños en los tests.

**Spec:**
- [ADR-0003](../../adr/0003-formato-contrato-fuente.md) (formato de contrato);
- [validacion-preflight.md](../../logica-negocio/validacion-preflight.md) (PRE-01…18, §7);
- [reglas-transversales.md §5](../../logica-negocio/reglas-transversales.md) (TR-D1, TR-D2, TR-D3, TR-D12, TR-D13);
- [vocabulario.md](../../logica-negocio/vocabulario.md);
- [nomina.md](../../logica-negocio/nomina.md) (NOM-BRZ-04…13), [bd.md](../../logica-negocio/bd.md) (BD-BRZ-02/03),
  [licencias.md](../../logica-negocio/licencias.md) (LIC-BRZ-04/06, LIC-SLV-02);
- [demo/README.md](../../../demo/README.md) (casos SUC y CASO).

## Decisiones de este plan

- **El generador demo no se migra a leer los contratos** (ADR-0002 lo pedía para "el Plan 3"). Se reemplaza por
  una verificación más barata y más fuerte: los tests de integración de Rust validan los Excel del generador
  contra los contratos (`limpio` y `sucio` sin errores; cada CASO con su error exacto). Si el generador y los
  contratos se desalinean, falla un test. La Tarea 7 agrega esta nota a ADR-0002.
- **Si varias cabeceras de `origen` están presentes**, gana la primera según el orden de `origen`. Es el caso de
  `#HRS SIMP.50%` y `HRS SIMPLES AL 50%`: la planilla demo trae ambas. No es error.
- **Los patrones** (`patron_nombre`, `patron_origen`) deben coincidir con el texto completo; el motor los envuelve
  en `^(?:…)$`. `patron_nombre` se aplica al nombre de archivo tal cual. `patron_origen` se aplica a la cabecera
  **ya normalizada** (§3 de ADR-0003), por eso no necesita `(?i)`.
- **Códigos de problema:** se reutilizan los IDs PRE de `validacion-preflight.md`. Se agregan PRE-19…PRE-23
  (Tarea 5 y Tarea 6), documentados en una sección nueva de ese archivo.
- **Filas de totales** (NOM-BRZ-10): no se tratan aquí. Las que no tienen documento se descartan por la regla
  `clave`; el resto queda para la capa Silver (Plan 4).

## Restricciones globales

- **Sin git** (decisión del usuario): no hay pasos de commit. Cada tarea termina con `cargo test --workspace`,
  `cargo clippy --workspace --all-targets -- -D warnings` y `cargo fmt --all --check` en verde.
- **Idioma** (AGENTS.md §5):
  - identificadores, mensajes y documentación en español;
  - identificadores en ASCII sin tildes ni ñ;
  - errores de librería con `thiserror`; en el binario, `anyhow`.
- **Confidencialidad** (AGENTS.md §3.1): ningún contrato, test ni *fixture* contiene el nombre del cliente ni datos
  reales. Los *fixtures* salen solo de `generar-demo`.
- **Legacy de solo lectura:** este plan no lo ejecuta ni lo lee en los tests.
- **Nombres canónicos** según `vocabulario.md`; formato `^[a-z][a-z0-9_]*$`.
- **Mensajes al usuario** en español claro, sin jerga técnica (`docs/diseno/`): "La hoja «Planilla» no existe en
  el archivo", no "Missing sheet".

## Revisión: casos que ningún test de tarea cubre por sí solo

1. **Archivo abierto en Excel** (en Windows, bloqueado o con un `~$archivo.xlsx` al lado): un error PRE-08 legible
   ("No se pudo abrir el archivo. ¿Está abierto en Excel?"), nunca un pánico. → test en Tarea 5.
2. **Hoja vacía o con solo el título** (sin filas de datos): la lectura devuelve 0 filas sin errores; el preflight
   reporta las columnas faltantes. → test en Tarea 6.
3. **Cabecera con saltos de línea o espacio duro** (`FECHA DE\nCESE`, `NBSP`): se reconoce igual. → test en Tarea 1.
4. **Contrato con una errata en una clave** (`requerido:` en lugar de `requerida:`): el programa no arranca y nombra
   el archivo y la clave. → test en Tarea 2.
5. **Número de documento escrito como número con decimales** (`12345678.0`, SUC-01) y en texto con `.0`
   (SUC-06): ambos quedan `"12345678"`. → test en Tarea 6.

---

### Tarea 1: workspace, normalización de cabeceras y referencias de celda

**Archivos:**
- Crear: `Cargo.toml` (workspace: `members = ["crates/core", "crates/cli"]`, `resolver = "3"`, dependencias
  compartidas en `[workspace.dependencies]`)
- Crear: `crates/core/Cargo.toml` (paquete `etl-local-core`, lib `etl_local_core`), `crates/core/src/lib.rs`
- Crear: `crates/core/src/normalizar.rs`, `crates/core/src/celda.rs`
- Crear: `crates/cli/Cargo.toml` (paquete `etl-local-cli`, `[[bin]] name = "etl-local"`), `crates/cli/src/main.rs`
  (vacío por ahora: `fn main() {}`)

**Interfaces:**
- Produce:
  - `normalizar::normalizar_cabecera(texto: &str) -> String`;
  - `normalizar::slug(texto: &str) -> String`;
  - `celda::Celda { pub fila: u32, pub columna: u32 }`, ambos 1-based, con `FromStr` ("A6"), `Display` ("A6"),
    `Celda::nueva(fila, columna)` y deserialización serde desde texto.

- [ ] **Paso 1: tests que fallan** (en cada módulo, `#[cfg(test)]`)

```rust
#[test]
fn normaliza_mayusculas_tildes_y_espacios() {
    assert_eq!(normalizar_cabecera("  Condición "), "CONDICION");
    assert_eq!(normalizar_cabecera("FECHA  DE\nCESE"), "FECHA DE CESE");
    assert_eq!(normalizar_cabecera("FECHA\u{a0}ING"), "FECHA ING");
    assert_eq!(normalizar_cabecera("año"), "ANO");
    assert_eq!(normalizar_cabecera("N°"), "N°");          // la puntuación se conserva
    assert_ne!(normalizar_cabecera("F. RENOVACION"), normalizar_cabecera("F RENOVACION"));
}

#[test]
fn slug_para_columnas_dinamicas() {
    assert_eq!(slug("MATERNIDAD"), "maternidad");
    assert_eq!(slug("POR ACCIDENTE (P)"), "por_accidente_p");
}

#[test]
fn celda_ida_y_vuelta() {
    assert_eq!("A6".parse::<Celda>().unwrap(), Celda::nueva(6, 1));
    assert_eq!("AA10".parse::<Celda>().unwrap(), Celda::nueva(10, 27));
    assert_eq!(Celda::nueva(2, 2).to_string(), "B2");
    for malo in ["", "6A", "A0", "a6x", "A"] {
        assert!(malo.parse::<Celda>().is_err(), "{malo}");
    }
}
```

- [ ] **Paso 2:** `cargo test -p etl-local-core`. Esperado: no compila (funciones inexistentes).
- [ ] **Paso 3: implementar.**
  - `normalizar_cabecera`: NFD con `unicode-normalization`; quitar marcas combinantes (`ñ`→`n`, `ó`→`o`; `°` queda);
    mayúsculas; colapsar todo espacio Unicode (`char::is_whitespace`) en uno solo y recortar.
  - `slug`: `normalizar_cabecera`, minúsculas, cada carácter no alfanumérico ASCII → `_`, colapsar `_` repetidos y
    recortar los de los extremos.
  - `Celda`: letras en mayúsculas o minúsculas; el resultado se muestra en mayúsculas.
- [ ] **Paso 4:** `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings` y
  `cargo fmt --all --check`. Esperado: todo en verde.

### Tarea 2: tipos del contrato, carga y validación del contrato

**Archivos:**
- Crear: `crates/core/src/contrato.rs` (tipos) y `crates/core/src/contrato/validar.rs` (reglas del contrato)
- Crear: `crates/core/src/error.rs` (`ErrorContrato`)

**Interfaces:**
- Consume: `Celda`, `normalizar_cabecera` (Tarea 1).
- Produce (todo `pub`, `Deserialize` + `JsonSchema` + `Debug` + `Clone`, con `#[serde(deny_unknown_fields)]`):

```rust
pub struct Contrato { pub version: u32, pub fuente: String, pub titulo: String,
                      pub archivo: ReglasArchivo, pub hojas: Vec<Hoja> }
pub struct ReglasArchivo { pub extensiones: Vec<String>,            // sin punto, minúsculas
                           #[serde(default)] pub patron_nombre: Option<String>,
                           #[serde(default)] pub fecha_en_nombre: Option<FechaEnNombre> }
pub struct FechaEnNombre { pub grupo: String, pub formato: FormatoFechaNombre }
pub enum FormatoFechaNombre { #[serde(rename = "AAAA-MM")] AnioMes, #[serde(rename = "DD.MM.AAAA")] DiaMesAnio }
pub struct Hoja { pub nombre: NombresHoja, pub cabecera: Celda,
                  #[serde(default)] pub fin_datos: FinDatos,          // FinHoja por defecto
                  #[serde(default)] pub columnas_extra: ColumnasExtra, // Ignorar por defecto
                  pub columnas: Vec<Columna>,
                  #[serde(default)] pub columnas_dinamicas: Vec<ColumnaDinamica> }
pub enum NombresHoja { Uno(String), Varios(Vec<String>) }            // #[serde(untagged)]; fn lista(&self) -> Vec<&str>
pub enum FinDatos { FinHoja, FilaVacia, ClaveVacia }                // snake_case
pub enum ColumnasExtra { Ignorar, Conservar }
pub struct Columna { pub nombre: String, pub origen: Vec<String>, pub tipo: TipoDato,
                     #[serde(default)] pub requerida: bool,
                     #[serde(default = "verdadero")] pub nula: bool,
                     #[serde(default)] pub clave: bool,
                     #[serde(default)] pub formatos: Vec<FormatoFecha>,   // vacío ⇒ [Excel] si tipo fecha
                     #[serde(default)] pub minimo: Option<f64>, #[serde(default)] pub maximo: Option<f64>,
                     #[serde(default)] pub valores: Vec<String> }
pub enum TipoDato { Texto, Entero, Decimal, Fecha, Periodo, Documento, Booleano }   // snake_case
pub enum FormatoFecha { #[serde(rename = "excel")] Excel, #[serde(rename = "DD/MM/AAAA")] DiaMesAnio,
                        #[serde(rename = "AAAA-MM-DD")] Iso }
pub struct ColumnaDinamica { pub prefijo: String, pub patron_origen: String, pub tipo: TipoDato }

impl Contrato {
    pub fn desde_yaml(texto: &str, origen: &str) -> Result<Contrato, ErrorContrato>; // parsea y valida
}
impl Hoja { pub fn clave(&self) -> Option<&Columna>; }  // la columna con clave: true
pub fn cargar_contrato(ruta: &Path) -> Result<Contrato, ErrorContrato>;
pub fn cargar_contratos(carpeta: &Path) -> Result<BTreeMap<String, Contrato>, ErrorContrato>; // *.yaml

#[derive(thiserror::Error, Debug)]
pub enum ErrorContrato {
    #[error("No se pudo leer {ruta}: {fuente}")] Lectura { ruta: PathBuf, fuente: std::io::Error },
    #[error("{origen}: el YAML no es válido: {mensaje}")] Yaml { origen: String, mensaje: String },
    #[error("{origen}: el contrato no es válido:\n- {}", detalles.join("\n- "))]
    Invalido { origen: String, detalles: Vec<String> },
}
```

Reglas de validación del contrato. Se acumulan todas en `Invalido.detalles`, con un texto por regla que nombra la
hoja y la columna:
- V1: `version == 1`;
- V2: `fuente`, cada `Columna.nombre` y cada `prefijo` cumplen `^[a-z][a-z0-9_]*$`;
- V3: `extensiones` no está vacía y cada una ∈ {`xlsx`, `xlsm`, `xls`};
- V4: `patron_nombre` y `patron_origen` compilan con `regex`;
- V5: si hay `fecha_en_nombre`, existe `patron_nombre` y contiene el grupo nombrado `grupo`;
- V6: `patron_origen` tiene el grupo `motivo`;
- V7: `hojas` no está vacía y cada hoja tiene al menos una columna;
- V8: en una hoja, los nombres de columna son únicos y ninguna normalización de `origen` se repite entre columnas;
- V9: `origen` no está vacío;
- V10: como máximo una columna `clave`;
- V11: `valores` solo con tipo `texto`; `minimo`/`maximo` solo con `entero` o `decimal`; `formatos` solo con `fecha`;
- V12: en `cargar_contratos`, `fuente` coincide con el nombre del archivo sin extensión, y no hay dos contratos
  con la misma fuente.

- [ ] **Paso 1: tests que fallan** en `crates/core/tests/contrato.rs`. Usan una constante `MINIMO` con un contrato
  YAML válido de una hoja y dos columnas, y una función `con(reemplazo)` que modifica una línea.
  - `contrato_minimo_carga`: `desde_yaml(MINIMO, "x.yaml")` es `Ok` y aplica los valores por defecto
    (`nula == true`, `requerida == false`, `FinDatos::FinHoja`, `ColumnasExtra::Ignorar`).
  - `errata_en_clave_nombra_la_clave`: `requerido: true` da `Err(ErrorContrato::Yaml { .. })` y el mensaje contiene
    `requerido`.
  - `version_desconocida`: `version: 2` da `Invalido` con un detalle que contiene `version`.
  - `nombre_no_canonico`: `nombre: Numero Documento` da `Invalido` con `Numero Documento` en el detalle.
  - `origen_repetido_tras_normalizar`: dos columnas con origen `['Condición']` y `['CONDICION ']` dan `Invalido`.
  - `fecha_en_nombre_sin_grupo`: `patron_nombre: 'BD.*'` con `grupo: fecha` da `Invalido`.
  - `regex_invalida`: `patron_nombre: '(['` da `Invalido`.
  - `acumula_varios_detalles`: un contrato con V1 y V2 rotas da `detalles.len() >= 2`.
  - `carpeta_con_fuente_que_no_coincide`: en un `tempdir`, `otra.yaml` con `fuente: planilla` hace que
    `cargar_contratos` dé `Invalido`.
- [ ] **Paso 2:** `cargo test -p etl-local-core --test contrato`. Esperado: no compila.
- [ ] **Paso 3:** implementar los tipos, `desde_yaml` (`serde_saphyr::from_str` → `ErrorContrato::Yaml` con el
  texto del error, que incluye línea y columna) y `validar(&Contrato) -> Vec<String>` en `contrato/validar.rs`.
- [ ] **Paso 4:** test, clippy y fmt en verde.

### Tarea 3: contratos de `planilla`, `base_personal` y `licencias`, y JSON Schema

**Archivos:**
- Crear: `config/fuentes/planilla.yaml`, `config/fuentes/base_personal.yaml`, `config/fuentes/licencias.yaml`
- Crear: `config/esquema-contrato.json` (generado) y `config/README.md` (qué es cada archivo y cómo regenerar el esquema)
- Crear: `crates/core/tests/contratos_config.rs`

**Interfaces:**
- Consume: `cargar_contratos` y `Contrato` (Tarea 2).
- Produce: los tres contratos, que usan las Tareas 5–7. `CARPETA_CONTRATOS` es un `const` de tests:
  `concat!(env!("CARGO_MANIFEST_DIR"), "/../../config/fuentes")`.

Contenido. Origen = cabecera del generador demo; nombre = `vocabulario.md`:

| Fuente | Archivo | Hoja / cabecera | Columnas (canónico ← origen) |
|---|---|---|---|
| `planilla` | `patron_nombre: '.*Planilla (?P<periodo>\d{4}-\d{2}).*\.(xlsx\|xlsm\|xls)'` (con `(?i)`), `fecha_en_nombre: {grupo: periodo, formato: AAAA-MM}` | `Planilla` / `A6` | `codigo_empleado`←`CÓDIGO CORPORATIVO`; `apellido_paterno` (requerida, nula false); `apellido_materno`; `nombres` (requerida, nula false); `tipo_documento`←`TIPO DE DOCUMENTO` (requerida); `numero_documento`←`DNI/CEX` (documento, requerida, nula false, **clave**); `sexo` (valores M, F); `fecha_nacimiento`←`F. NACIMIENTO`; `area`←`ÁREA`; `centro_costo`←`CENTRO DE COSTOS`; `fecha_entrada`←`FECHA DE INGRESO`; `fecha_salida`←`FECHA CESE`; `motivo_salida`←`MOTIVO CESE`; los 6 `dias_*` de vocabulario §4 (entero, mínimo 0); `horas_extra_25/35/100`; `horas_extra_50`←[`#HRS SIMP.50%`, `HRS SIMPLES AL 50%`] (decimal, mínimo 0). `columnas_dinamicas`: `{prefijo: dias_subsidio_, patron_origen: 'DIAS SUB\.\s*(?P<motivo>.+)', tipo: entero}` |
| `base_personal` | `patron_nombre: 'BD\.\s*(?P<fecha>\d{2}\.\d{2}\.\d{4})\.\.(xlsx\|xlsm\|xls)'` (con `(?i)`), `fecha_en_nombre: {grupo: fecha, formato: DD.MM.AAAA}` | `Personal` / `A10` | las 16 columnas de `excel_base_personal.py`: `numero_documento`←`NUMERO DE DOC` (clave, requerida); `codigo_empleado`←`CODIGO SAP2`; `nombre_completo`←`NOMBRE COMPLETOS`; `gerencia`; `sexo`; `sede`←`SEDE3`; `tipo_trabajador`←`WHITE COLLAR / BLUE COLLAR`; `modalidad_contrato`; `fecha_fin_contrato`←`Fecha de Termino`; `servicio`; `regimen_trabajo`; `fecha_entrada`←`FECH_INGR.` (fecha, `formatos: [excel, DD/MM/AAAA]`); `fecha_nacimiento`←`FECHA DE NAC.`; `centro_costo`←`CC`; `nombre_centro_costo`←`NOMBRE CC`; `categoria_centro_costo`←`CATEGORIA CC`. Requeridas: las 13 de la tabla §6 de `validacion-preflight.md` |
| `licencias` | sin patrón | `LICENCIA CON GOCE` y `LICENCIA SIN GOCE` / `A2` | en ambas: `numero_documento`←`DNI/CEX` (clave); `periodo`←`PERIODO` (periodo); `apellido_paterno`; `apellido_materno`; `nombres`; `motivo_licencia`←`MOTIVO DE LIC.CON GOCE` o `MOTIVO DE LIC.S.G.H` según la hoja. Todas requeridas (LIC-BRZ-06) |

- Las fechas de la planilla usan `formatos: [excel, DD/MM/AAAA]`.
- El nombre de la hoja de la base de personal es el del demo. Un comentario en el YAML indica que cada empresa
  pone aquí el suyo; nunca se escribe el del cliente.

- [ ] **Paso 1: tests que fallan** en `crates/core/tests/contratos_config.rs`:
  - `cargan_las_tres_fuentes`: `cargar_contratos(CARPETA_CONTRATOS)` tiene como claves exactamente
    `["base_personal", "licencias", "planilla"]`.
  - `cada_hoja_tiene_una_clave`: toda hoja de los tres contratos tiene `hoja.clave().is_some()`.
  - `esquema_contrato_al_dia`: `serde_json::to_string_pretty(&schemars::schema_for!(Contrato))` + `"\n"` es igual
    al contenido de `config/esquema-contrato.json`. Con la variable `ACTUALIZAR_ESQUEMA=1`, el test escribe el
    archivo y pasa.
- [ ] **Paso 2:** `cargo test -p etl-local-core --test contratos_config`. Esperado: FAIL (no existen los archivos).
- [ ] **Paso 3:** escribir los tres YAML y `config/README.md`. Generar el esquema con
  `ACTUALIZAR_ESQUEMA=1 cargo test -p etl-local-core --test contratos_config esquema_contrato_al_dia`.
  - `Celda` usa `#[schemars(with = "String")]`.
  - Cada YAML empieza con `# yaml-language-server: $schema=../esquema-contrato.json`.
- [ ] **Paso 4:** test (sin la variable), clippy y fmt en verde.

### Tarea 4: *fixtures* de demo para los tests de Rust

**Archivos:**
- Crear: `crates/core/tests/fixtures/demo/{limpio,sucio,casos}/…` (generados) y `crates/core/tests/fixtures/README.md`
- Crear: `crates/core/tests/comun/mod.rs`

**Interfaces:**
- Produce: `comun::fixture(modo: &str) -> PathBuf`, la carpeta del modo, que falla con un mensaje que incluye el
  comando de regeneración si no existe. También `comun::manifiesto(modo) -> serde_json::Value`.

- [ ] **Paso 1:** generar desde la raíz del repo. Se usan 3 meses para que aparezca la columna dinámica de maternidad
  (desde el tercer mes):

```bash
uv run --project demo generar-demo --modo limpio --meses 3 --salida crates/core/tests/fixtures/demo
uv run --project demo generar-demo --modo sucio  --meses 3 --salida crates/core/tests/fixtures/demo
uv run --project demo generar-demo --modo casos  --meses 1 --salida crates/core/tests/fixtures/demo
```

  Esperado: tres carpetas con `.generar-demo`, unos 2 MB en total. `limpio` y `sucio` tienen las planillas
  2025-09, 2025-10 y 2025-11, con la misma población (misma semilla y parámetros).
- [ ] **Paso 2:** escribir `fixtures/README.md` con los tres comandos y la regla "solo se regeneran con
  `generar-demo`; nunca se editan a mano". Escribir `comun/mod.rs`.
- [ ] **Paso 3:** comprobar que cada carpeta tiene `.generar-demo` (prueba de que salió del generador, con datos
  ficticios por construcción) y que `du -sh crates/core/tests/fixtures` da 2,5 MB o menos.

### Tarea 5: preflight

**Archivos:**
- Crear: `crates/core/src/informe.rs`, `crates/core/src/preflight.rs`, `crates/core/src/libro.rs`. Este último
  abre el libro y lee celdas como texto: es la única dependencia de `calamine` fuera de `lectura`.
- Crear: `crates/core/tests/preflight.rs`
- Modificar: `docs/logica-negocio/validacion-preflight.md` (sección nueva "§9 Reglas del motor nuevo", con
  PRE-19 y la diferencia de normalización)

**Interfaces:**
- Consume: `Contrato`, `Hoja`, `Columna`, `NombresHoja`, `FechaEnNombre` (Tarea 2); `Celda`, `normalizar_cabecera`
  (Tarea 1).
- Produce:

```rust
#[derive(Serialize, Debug, Clone, PartialEq)] #[serde(rename_all = "snake_case")]
pub enum Severidad { Error, Aviso }
#[derive(Serialize, Debug, Clone)]
pub struct Problema { pub codigo: String, pub severidad: Severidad, pub archivo: String,  // nombre base
                      pub hoja: Option<String>, pub celda: Option<String>, pub mensaje: String }
#[derive(Serialize, Debug, Clone, Default)]
pub struct Informe { pub problemas: Vec<Problema> }
impl Informe { pub fn tiene_errores(&self) -> bool; pub fn errores(&self) -> impl Iterator<Item=&Problema>;
               pub fn avisos(&self) -> impl Iterator<Item=&Problema>; }

#[derive(Serialize, Debug, Clone, PartialEq)]
pub enum FechaNombre { Periodo(String), FechaCorte(chrono::NaiveDate) }

pub fn validar_archivo(contrato: &Contrato, ruta: &Path) -> Informe;

// interno al crate (pub(crate)), lo usa la Tarea 6:
pub(crate) struct Inspeccion { pub informe: Informe, pub fecha_nombre: Option<FechaNombre>,
                               pub libro: Option<libro::Libro>, pub hojas: Vec<HojaUbicada> }
pub(crate) struct HojaUbicada { pub indice_contrato: usize, pub nombre_real: String,
                                pub fila_cabecera: u32,
                                pub columnas: Vec<(u32 /*columna 1-based*/, usize /*índice en Hoja.columnas*/)>,
                                pub dinamicas: Vec<(u32, String /*nombre canónico*/, TipoDato)> }
pub(crate) fn inspeccionar(contrato: &Contrato, ruta: &Path) -> Inspeccion;
```

Algoritmo. Se mantiene el orden y la acumulación del legacy (§3 de `validacion-preflight.md`); los mensajes van
en español:

| Código | Condición | Detiene | Mensaje (plantilla) | hoja / celda |
|---|---|---|---|---|
| PRE-05 | no existe | sí | «El archivo no existe.» | — |
| PRE-06 | extensión ∉ contrato (sin distinguir mayúsculas) | sí | «El archivo es .{ext}; se esperaba {lista}.» | — |
| PRE-07 | `patron_nombre` no coincide completo | no | «El nombre del archivo no tiene el formato esperado para {titulo}.» | — |
| PRE-07 | fecha del nombre imposible (`31.02.2025`, `2025-13`) | no | «La fecha del nombre ({texto}) no existe en el calendario.» | — |
| PRE-08 | `calamine::open_workbook_auto` falla | sí | «No se pudo abrir el archivo. ¿Está abierto en Excel o dañado?» | — |
| PRE-09 | ningún nombre de `NombresHoja` existe (comparación exacta) | no (esa hoja) | «Falta la hoja «{nombre}».» | primer nombre / — |
| PRE-10 | columnas requeridas ausentes en la cabecera esperada, pero presentes al desplazar ±3 filas y ±3 columnas (TR-D13) | no | «La cabecera debería empezar en {esperada}, pero está en {encontrada}. ¿Se insertó o borró una fila o columna?» | hoja / esperada |
| PRE-10 | celda de cabecera vacía y sin desplazamiento encontrado | no | «La celda {celda} debería tener la primera cabecera y está vacía.» | hoja / celda |
| PRE-11 | faltan columnas `requerida` (ninguno de sus `origen` está) | no | «Faltan columnas obligatorias: {origen[0] de cada una, separados por coma}.» | hoja / `fila {n}` |
| PRE-19 | dos cabeceras de la fila normalizan igual y alguna está en un `origen` | no | «La columna «{texto}» aparece dos veces ({celda1} y {celda2}).» | hoja / celda1 |

- **Escaneo de cabecera:** desde la celda de cabecera, como máximo 250 columnas. Termina tras 3 vacías seguidas
  después de la primera no vacía (paridad con PRE-11).
- **Desplazamiento:** si se encuentra, se reporta solo PRE-10 (no PRE-11) para esa hoja.
- **Columnas mapeadas:** a cada columna del contrato se le asigna la primera cabecera presente según el orden de
  `origen`. Las cabeceras no mapeadas que cumplen un `patron_origen` se vuelven dinámicas con nombre
  `prefijo + slug(motivo)`.
- **Fecha del nombre:** `AAAA-MM` → `FechaNombre::Periodo` (mes 1–12); `DD.MM.AAAA` → `FechaNombre::FechaCorte`.

- [ ] **Paso 1: tests que fallan** en `crates/core/tests/preflight.rs` (`mod comun;`):
  - `limpio_sin_errores` y `sucio_sin_errores`: cada Excel de `planilla/`, `base_personal/` y `planilla/licencias/`,
    validado con su contrato, da `!informe.tiene_errores()`. SUC-02 (espacios de más en cabeceras) no produce errores.
  - `casos_reportan_su_error`: una tabla fija `(caso, fuente, codigo, hoja, celda)`:
    - `CASO-01, planilla, PRE-10, Some("Planilla"), Some("A6")`, y el mensaje contiene `A7`;
    - `CASO-02, planilla, PRE-09, Some("Planilla"), None`;
    - `CASO-03, planilla, PRE-11, Some("Planilla"), Some("fila 6")`, y el mensaje contiene `DNI/CEX`;
    - `CASO-04, planilla, PRE-07, None, None`;
    - `CASO-05, base_personal, PRE-07, None, None`;
    - `CASO-06, licencias, PRE-09, Some("LICENCIA SIN GOCE"), None`.

    Para cada caso, el único `.xlsx` de su carpeta da **exactamente un** error, con esos campos. Además, `hoja` y
    `celda` coinciden con los de `esperado.json` (donde `""` equivale a `None`).
  - `periodo_y_fecha_de_corte_del_nombre`: `inspeccionar` sobre `Planilla 2025-11 Empleados.xlsx` da
    `FechaNombre::Periodo("2025-11")`; sobre `BD. 12.11.2025..xlsx`, `FechaCorte(2025-11-12)`.
  - Con `rust_xlsxwriter` en un `tempdir`:
    - `fecha_imposible_en_nombre`: `BD. 31.02.2025..xlsx` con hoja y cabecera correctas da un solo error PRE-07
      cuyo mensaje contiene `31.02.2025`;
    - `extension_no_permitida`: un `.csv` da un error PRE-06 y nada más;
    - `archivo_inexistente`: PRE-05;
    - `archivo_no_excel`: un `.xlsx` con texto plano da PRE-08 sin pánico (caso de revisión 1);
    - `cabecera_duplicada`: dos celdas `DNI/CEX` en la fila 6 dan PRE-19 con la primera celda;
    - `alias_en_orden`: una planilla con `HRS SIMPLES AL 50%` en C6 y `#HRS SIMP.50%` en E6 mapea `horas_extra_50`
      a la columna 5 (vía `inspeccionar`, comprobando `HojaUbicada.columnas`).
- [ ] **Paso 2:** `cargo test -p etl-local-core --test preflight`. Esperado: no compila.
- [ ] **Paso 3:** implementar `informe`, `libro` (abrir con `open_workbook_auto` y exponer
  `texto_celda(hoja, Celda) -> Option<String>`, con posiciones **absolutas**: `Range::get_value` es relativo a
  `range.start()`, hay que restarlo) y `preflight`. Documentar PRE-19 en `validacion-preflight.md` §9.
- [ ] **Paso 4:** test, clippy y fmt en verde.

### Tarea 6: lectura de datos tipados

**Archivos:**
- Crear: `crates/core/src/lectura.rs`, `crates/core/src/convertir.rs` (celda → `Valor`, una función por tipo)
- Crear: `crates/core/tests/lectura.rs`
- Modificar: `docs/logica-negocio/validacion-preflight.md` §9 (PRE-20…PRE-23)

**Interfaces:**
- Consume: `inspeccionar`, `Inspeccion`, `HojaUbicada`, `Informe`, `Problema`, `FechaNombre` (Tarea 5); `Contrato`,
  `TipoDato`, `FormatoFecha`, `FinDatos` (Tarea 2).
- Produce:

```rust
#[derive(Serialize, Debug, Clone, PartialEq)] #[serde(untagged)]
pub enum Valor { Nulo, Texto(String), Entero(i64), Decimal(f64), Fecha(chrono::NaiveDate), Booleano(bool) }
#[derive(Serialize, Debug, Clone)]
pub struct ColumnaLeida { pub nombre: String, pub tipo: TipoDato, pub valores: Vec<Valor> }
#[derive(Serialize, Debug, Clone)]
pub struct TablaHoja { pub hoja: String, pub filas_origen: Vec<u32>, pub columnas: Vec<ColumnaLeida> }
#[derive(Serialize, Debug, Clone)]
pub struct Lectura { pub fuente: String, pub archivo: String, pub fecha_nombre: Option<FechaNombre>,
                     pub hojas: Vec<TablaHoja>, pub informe: Informe }   // informe: solo avisos
impl TablaHoja { pub fn columna(&self, nombre: &str) -> Option<&ColumnaLeida>; pub fn filas(&self) -> usize; }

pub fn leer_archivo(contrato: &Contrato, ruta: &Path) -> Result<Lectura, Informe>; // Err si hay algún error
```

Reglas de conversión (`convertir.rs`):
- **Nulo:** celda vacía, texto vacío tras recortar, o `None`, `nan` o `NaT` (sin distinguir mayúsculas; TR-12).
- **Valores de error:** una celda de error de Excel (`#N/A`, `#DIV/0!`…) o el texto `#N/D`/`#N/A` da `Nulo` y un
  aviso **PRE-22** «La celda tiene un error de Excel ({valor}); se tomó como vacía.» (TR-12, SUC-04).
- **Por tipo:**

| Tipo | Acepta | Resultado |
|---|---|---|
| `texto` | texto (recortado); número | número entero sin `.0` |
| `texto` | fecha | `AAAA-MM-DD` |
| `documento` | número entero o decimal sin parte fraccionaria | texto sin decimales |
| `documento` | texto | recortado y sin `.0` final (LIC-SLV-02, SUC-01, SUC-06); sin relleno de ceros |
| `entero` | entero; decimal sin parte fraccionaria; texto numérico | entero |
| `decimal` | número; texto con punto decimal | decimal |
| `fecha` | según `formatos` (vacío = `[excel]`): `excel` = celda de fecha de Excel; `DD/MM/AAAA` y `AAAA-MM-DD` = texto | fecha |
| `periodo` | texto `AAAA-MM` con mes 1–12; celda de fecha | `Texto("AAAA-MM")` |
| `booleano` | booleano; `SI`/`NO`/`TRUE`/`FALSE`/`1`/`0` sin distinguir mayúsculas | booleano |

- **Conversión imposible:** `Nulo` y un aviso **PRE-20** «No se pudo leer «{valor}» como {tipo}.», con la celda
  (corrige BD-DIF-03 / SUC-03).
- **Nulo prohibido:** `Nulo` en una columna `nula: false` (en una fila que se conserva) da un error **PRE-21**
  «Falta {origen usado} en la fila {n}.»
- **Fuera de rango:** un valor fuera de `valores` (comparado con `normalizar_cabecera`) o de `minimo`/`maximo` da
  un aviso **PRE-23** «El valor «{v}» no es uno de los esperados ({lista}).» o «… está fuera del rango».
- **Filas:**
  - desde `fila_cabecera + 1` hasta el final de la hoja;
  - `FilaVacia` corta en la primera fila sin ninguna celda con valor y `ClaveVacia` en la primera con la clave
    vacía; `FinHoja` no corta;
  - con cualquier `fin_datos`, se **descarta** sin aviso toda fila cuya clave esté vacía (o la fila entera, si la
    hoja no tiene clave);
  - `filas_origen` guarda el número de fila de Excel (1-based) de cada fila conservada.
- **Columnas:**
  - el orden es el del contrato y luego las dinámicas, en orden de aparición;
  - las columnas del contrato que no estén en el archivo aparecen con todos sus valores `Nulo`;
  - con `columnas_extra: conservar`, las cabeceras no mapeadas se agregan como `texto` con nombre `slug(cabecera)`.

- [ ] **Paso 1: tests que fallan** en `crates/core/tests/lectura.rs`:
  - `limpio_cuadra_con_manifiesto`: para cada periodo del manifiesto de `limpio`, la planilla de ese periodo
    cumple:
    - `filas()` == `personas_planilla`;
    - la suma de `dias_licencia_con_goce`, `dias_licencia_sin_goce` y `dias_descanso_medico` es igual a la de
      `dias_ausencia`;
    - la suma de `dias_subsidio_maternidad` (dinámica) es igual a la del manifiesto, o la columna no existe si el
      valor es 0;
    - la suma de `horas_extra_25` es igual a la del manifiesto, con tolerancia 1e-6;
    - el informe no tiene avisos.
  - `base_personal_cuadra`:
    - `filas()` == `empleados + practicantes` del manifiesto;
    - `fecha_nombre == Some(FechaCorte(…))` y coincide con `base_personal.fecha_corte`.
  - `licencias_dos_hojas`: `hojas.len() == 2`; la columna `periodo` tiene solo valores `AAAA-MM`.
  - `sucio_mismos_conteos`: con `sucio`, cada planilla y la base de personal dan los mismos `filas()` que con
    `limpio`. La fila vacía de SUC-05 no corta la lectura.
  - `sucio_documentos_limpios` (caso de revisión 5): en la planilla y en licencias de `sucio`, todo
    `numero_documento` es `Texto` que cumple `^\d+$`.
  - `sucio_fechas_texto_se_leen`: en la base de personal de `sucio`, ningún aviso PRE-20 en `fecha_entrada`
    (SUC-03 usa `DD/MM/AAAA`). Los valores de `fecha_entrada` son iguales, fila por fila, a los de `limpio`.
  - `sucio_error_excel_avisa`: hay al menos un aviso PRE-22 con `celda` no vacía en la planilla de `sucio` (SUC-04).
  - Con `rust_xlsxwriter`:
    - `hoja_sin_datos` (caso de revisión 2): solo cabecera, `Ok` con `filas() == 0`;
    - `fecha_invalida_avisa`: `31/02/2025` en una columna `fecha` da `Nulo` y PRE-20 en su celda;
    - `nulo_prohibido_es_error`: `nombres` vacío en una fila con documento da `Err`, con PRE-21 en esa fila;
    - `fila_vacia_corta`: con `fin_datos: fila_vacia`, una fila en blanco a mitad corta la lectura y las filas
      posteriores no se leen;
    - `documento_como_numero`: `12345678.0` (número) y `"12345678.0"` (texto) dan `Texto("12345678")`.
- [ ] **Paso 2:** `cargo test -p etl-local-core --test lectura`. Esperado: no compila.
- [ ] **Paso 3:** implementar `convertir.rs` (funciones puras por tipo, con tests unitarios propios para cada fila
  de la tabla de conversión) y `lectura.rs` sobre `inspeccionar`. Documentar PRE-20…23 en §9.
- [ ] **Paso 4:** test, clippy y fmt en verde.

### Tarea 7: CLI `etl-local` y documentación

**Archivos:**
- Modificar: `crates/cli/src/main.rs`; crear `crates/cli/src/salida.rs` (formato humano y JSON)
- Crear: `crates/cli/tests/cli.rs`
- Modificar:
  - `AGENTS.md` §2 (quitar "(planificada)" de `crates/` y `config/`) y §6 (comando `etl-local`);
  - `docs/adr/0002-demo-python-uv.md` (nota de la decisión sobre el generador);
  - la hoja de ruta (Plan 3 → Completado).

**Interfaces:**
- Consume: `cargar_contratos`, `validar_archivo`, `leer_archivo`, `Informe`, `Lectura` (Tareas 2, 5 y 6).
- Produce:

```
etl-local validar --fuente <FUENTE> [--config <CARPETA>] [--json] <ARCHIVOS>...
etl-local leer    --fuente <FUENTE> [--config <CARPETA>] [--json] <ARCHIVOS>...
```

- `--config` vale por defecto `config/fuentes`.
- **Códigos de salida:**
  - 0: sin errores (puede haber avisos);
  - 1: algún archivo con errores;
  - 2: problema de uso, de contrato o fuente desconocida. Este caso imprime las fuentes disponibles.
- **Salida humana**, una línea por problema:
  `{archivo} · {hoja} · {celda}: {mensaje} [{codigo}]`, omitiendo las partes vacías y prefijando `Aviso: ` a los
  avisos. Al final: «{n} archivos revisados: {e} con errores, {a} avisos.»
- **`leer`** imprime además, por archivo y hoja, «{hoja}: {filas} filas, {columnas} columnas» y la fecha del nombre.
- **`--json`** imprime un arreglo con un objeto por archivo:
  - en `validar`: `{"archivo", "informe"}`;
  - en `leer`: `{"archivo", "fecha_nombre", "hojas": [{"hoja", "filas", "columnas": [nombres]}], "informe"}`.

  No incluye los valores: eso es para el Plan 4.

- [ ] **Paso 1: tests que fallan** en `crates/cli/tests/cli.rs`. Ejecutan `env!("CARGO_BIN_EXE_etl-local")` con
  `--config` apuntando a `config/fuentes` y los *fixtures* de la Tarea 4:
  - `validar_limpio_sale_0`: las 3 planillas de `limpio` dan código 0 y la última línea contiene
    `3 archivos revisados: 0 con errores`.
  - `validar_caso_sale_1`: CASO-03 da código 1 y stdout contiene `DNI/CEX` y `[PRE-11]`.
  - `fuente_desconocida_sale_2`: `--fuente nomina` da código 2, y stderr contiene `planilla` y `base_personal`.
  - `contrato_roto_sale_2`: una carpeta temporal con un YAML inválido da código 2, y stderr nombra el archivo.
  - `leer_json`: `leer --json` sobre la base de personal de `limpio` da JSON válido; `hojas[0].filas` es igual a
    `empleados + practicantes` y `fecha_nombre` contiene `2025-11-12`.
- [ ] **Paso 2:** `cargo test -p etl-local-cli`. Esperado: FAIL.
- [ ] **Paso 3:** implementar con `clap` derive. `main` devuelve `std::process::ExitCode`; los errores de contrato
  se muestran con `ErrorContrato`.
- [ ] **Paso 4:** actualizar los documentos:
  - **AGENTS.md §6:** fila "Validar archivos" con
    `cargo run -p etl-local-cli -- validar --fuente planilla <archivos>`.
  - **ADR-0002:** nota con fecha: «El generador no lee los contratos; en su lugar, los tests de `crates/core`
    validan su salida contra `config/fuentes/` (Plan 3).»
  - **Hoja de ruta:** fila del Plan 3 en **Completado**.
- [ ] **Paso 5:** verificación final desde la raíz:
  - `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings` y `cargo fmt --all --check`;
  - `uv run --project demo pytest demo/tests -q`, que sigue en verde;
  - el nombre del cliente no aparece en `config/`, `crates/` ni `docs/`: se lee en tiempo de ejecución con
    `uv run --project demo python -c "from paridad_legacy.entorno import nombres_a_ocultar, ubicar_legacy; print('
'.join(nombres_a_ocultar(ubicar_legacy())))"`
    y se pasa a `grep -ril -f -`, sin escribirlo nunca en un archivo. Esperado: sin coincidencias;
  - `cargo run -p etl-local-cli -- validar --fuente planilla "demo/generated/limpio/planilla/Planilla 2025-11 Empleados.xlsx"`,
    que da código 0.
