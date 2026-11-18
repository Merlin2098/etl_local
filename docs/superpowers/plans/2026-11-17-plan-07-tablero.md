# Plan 7: Tablero (pantallas 6–7) — Plan de implementación

> **Para agentes:** SUB-SKILL REQUERIDA: usar superpowers:subagent-driven-development (recomendado) o
> superpowers:executing-plans para implementar este plan tarea por tarea. Los pasos usan casillas (`- [ ]`).
> Este repo **no usa git**: donde la skill dice «commit», se registra la tarea en el ledger
> `.superpowers/sdd/2026-11-17-plan-07-tablero/progress.md`.

**Objetivo:** dirección mira el tablero en 30 segundos (tasas, comparación con el mes anterior, riesgos y tres
gráficos), RR. HH. ve quiénes están en alerta, cualquiera entiende de dónde sale cada número y se lleva un
resumen de una página en PDF. Funciona en la app de escritorio y en el navegador, sin internet.

**Arquitectura:**
- **API (`crates/api`, `crates/core/src/tablero*`).** Crece en tres cosas:
  - el desglose por gerencia de IND-01, IND-07 e IND-08, en SQL;
  - un JSON Schema de todas sus respuestas, del que salen los tipos TypeScript;
  - servir los archivos de `app/ui/dist/` (sin `app.html`) en la misma dirección que la API.
- **Frontend (`app/ui`).** Una segunda entrada, `tablero.html` (ADR-0005 §3), con su estado en una clase con
  *runes* que recibe un cliente de la API inyectado, igual que `Flujo` en el Plan 6. Solo usa `fetch` a
  `/api/v1/`: nada de `@tauri-apps`. Los filtros viven en la URL.
- **App de escritorio (`app/src-tauri`).** Levanta la API en su propio runtime al arrancar y abre el tablero en
  una ventana que apunta a `http://127.0.0.1:<puerto>/tablero.html`: el mismo camino que en el navegador.

**Stack:**
- Rust: lo que ya hay (axum 0.8, tokio, schemars 1.2 con la feature `chrono04`).
- Frontend: lo del Plan 6 más `echarts` 6.1.0 (ADR-0005) y `json-schema-to-typescript` (desarrollo; versión exacta
  fijada en la Tarea 2).

**Spec:**
- [docs/diseno/README.md](../../diseno/README.md): pantallas 6 y 7, principios y lenguaje visual. Los mocks v2
  están en un lienzo privado; los textos que usa este plan están copiados en la sección «Textos».
- [indicadores-tablero.md](../../logica-negocio/indicadores-tablero.md): IND-01…08, checks y §7.
- [ADR-0004](../../adr/0004-api-local-tablero.md): API local, `Host`, privacidad por nivel.
- [ADR-0005](../../adr/0005-frontend-svelte-echarts.md): Svelte, ECharts SVG, `tablero.html`, PDF por impresión.
- [reglas-transversales.md](../../logica-negocio/reglas-transversales.md): TR-D15…TR-D19.

**Decisiones del usuario (2026-11-17):**
1. **Pestañas Resumen y Alertas.** Alertas (solo RR. HH.) muestra las listas con nombres, porque «Ver detalle» y
   «Ver quiénes» las necesitan. Ausentismo, Horas extra, Practicantes y Salidas quedan para cuando se diseñen.
2. **Desglose por gerencia en la API.** Cambio aditivo en `/api/v1/indicadores`: campo `por_gerencia` en
   IND-01, IND-07 e IND-08.
3. **Tipos TypeScript derivados:** `schemars` genera el JSON Schema y `json-schema-to-typescript` los tipos; un
   test falla si quedan desfasados. Reemplaza también los tipos escritos a mano del Plan 6 (nota de ADR-0005).

**Decisiones de este plan** (el ejecutor las registra como `Ruling:` si las cambia):
1. **«Personas en planilla por gerencia» y «Días de ausencia» son listas de barras en HTML** (botones), como en el
   mock, y no ECharts: cada barra es un botón que filtra y se lee con lector de pantalla. ECharts dibuja
   «Entradas y salidas por mes» (barras agrupadas, renderizador SVG).
2. **IND-07 no lleva comparación con el mes anterior.** Usa la última base de personal y no responde al
   periodo (§2), así que «Igual que en octubre» del mock no tiene dato que lo respalde.
3. **Sin «Definir meta» ni «Actualizar» en el tablero.** No hay pantalla de ajustes; en lugar de «Actualizar», el
   tablero vuelve a pedir los datos cuando la ventana recupera el foco.
4. **«Datos en Excel» descarga un CSV** de los indicadores visibles (sin nombres). Los CSV usan BOM UTF-8, `;` y
   coma decimal, como los números de la UI.
5. **Sin «Ver quiénes son» en Listo:** ningún aviso actual de Listo tiene un detalle nominal asociado.
6. **Sin la nota «Datos de ejemplo generados para la demo.»:** el tablero no sabe si los datos son de demo.
7. **El tablero de la app se sirve desde la API** (mismo origen, sin CORS). En `tauri dev` la ventana apunta a
   Vite (`localhost:5173`), que reenvía `/api` a la API de la app en el puerto 8787.

## Restricciones globales

- **Confidencialidad:**
  - nunca escribir el nombre del cliente (usar `<CLIENTE>`) ni copiar datos reales;
  - `HR_Data_Integration_Pipeline/` es solo lectura (AGENTS.md §3.1);
  - las pruebas usan solo los fixtures sintéticos de `crates/core/tests/fixtures/demo/`.
- **Idioma:** español en UI, docs, mensajes y nombres; identificadores ASCII (sin ñ ni tildes).
- **Privacidad por nivel (TR-D18, ADR-0004 §4):** la pestaña Resumen, el PDF y «Datos en Excel» nunca muestran
  nombres ni números de documento. Solo la pestaña Alertas llama a `/api/v1/detalle/…`.
- **El frontend no calcula indicadores** (ADR-0005): muestra, filtra y explica. Se permiten sumas y porcentajes
  de un desglose ya calculado y días del mes.
- **UI sin jerga** (diseno §Principios 3). Los IDs de regla aparecen solo en «¿De dónde sale?» y en «Cómo leer
  este tablero» (TR-D17), con la fuente en mono.
- **Comparaciones:** la flecha indica la dirección y una palabra dice si es mejor o peor; nunca solo el color
  (indicadores-tablero §4).
- **Lenguaje visual de diseno §Lenguaje visual:** los tokens de `app/ui/src/lib/estilos/tokens.css`. «Entraron» y
  «Salieron» son `#2B6CB0` y `#C2620F`, y las barras de una sola serie, `#1E6B47`. Controles de 44 px como mínimo.
- **Funciona sin internet:** nada desde un CDN; las fuentes son las del Plan 6.
- **El navegador no ve Tauri:** `tablero.html` no importa `@tauri-apps/*` (se verifica sobre `dist/`).
- **Formato:** números con `formato.ts` (`1.239`, `1,9 %`); los meses en español.
- **Dependencias:** solo las de «Stack». Otra exige preguntar (AGENTS.md §4).
- **Directrices de compilación** ([hoja de ruta](2026-11-16-hoja-de-ruta.md)), más lo aprendido en el Plan 6:
  - nunca `cargo clean`; tras un corte, borrar los `*.rmeta`/`*.rlib` de 0 bytes en todo `target/`;
  - `-j 2`, o `-j 1` con menos de ~4 GB libres; nunca dos compilaciones a la vez, ni tests mientras compila un release;
  - release solo en la Tarea 11, solo, en segundo plano;
  - `rust-analyzer.checkOnSave` está apagado (`.vscode/settings.json`); no reactivarlo.

## Textos (del mock v2; exactos salvo los marcados ✎)

| Lugar | Texto |
|---|---|
| Encabezado | marca «Tablero RR. HH.»; «Cómo leer este tablero», «Datos en Excel», «Resumen en PDF (1 página)» |
| Pestañas | «Resumen», «Alertas (solo RR. HH.)» (`aria-label` «Páginas del tablero») |
| Datos usados | «**Datos usados:**» y, por fuente: «Planilla: {mes año}», «Base de personal: {fecha corta}», «Licencias: incluidas» ✎ (no traen fecha); con periodos mezclados, el `aviso_periodos` de la API con ícono ⚠ y la palabra «Aviso» |
| Filtros | «Periodo», «Gerencia», «Sede», «Tipo de trabajador»; opciones «Todas», «Todas», «Todos» |
| Tarjeta IND-01 | «Personas en planilla»; «{n} menos/más que en {mes}» o «Igual que en {mes}»; «{Mes}: {anterior} · {IND-02} entraron, {IND-03} salieron» |
| Tarjeta IND-04 | «Rotación del mes»; «Empeoró/Mejoró: era {x} en {mes}» o «Igual que en {mes}»; «Sin meta definida» o «Meta: {x}» ✎ |
| Tarjeta IND-05 | «Ausentismo»; comparación como IND-04; «{días} días perdidos de {programados} programados» |
| Tarjeta IND-07 | «Contratos por superar su plazo»; «Superan el plazo legal en 6 meses o menos» (sin comparación, decisión 2) |
| ¿De dónde sale? | botón `aria-label` «¿De dónde sale este número?»; título «¿De dónde sale este número?»; «Fuente», «Qué cuenta», «Regla»; en IND-01: «**{n} personas** de la planilla no están en la base de personal, así que no aparecen al filtrar por gerencia o sede.» + «Ver quiénes» |
| Riesgos | «Riesgos a atender»; «Resumen para dirección · el detalle con nombres está en la pestaña «Alertas», solo para RR. HH.»; «Ver detalle (solo RR. HH.)» |
| Riesgo IND-07 | «contratos temporales superarán el plazo legal en los próximos 6 meses» (1: «contrato temporal superará…»); «{a} por obra o servicio (máx. 5 años) · {b} por incremento de actividad (máx. 3 años). Si se pasan del plazo, pueden considerarse contratos indefinidos.»; «Más casos: {G1} ({n}), {G2} ({n}), {G3} ({n}), otras ({n})»; con 0: «Ningún contrato temporal supera su plazo en los próximos 6 meses.» ✎ |
| Riesgo IND-08 | «personas llegan a la edad de jubilación este año o el próximo» (1: «persona llega…»); «{a} cumplen 65 años · {b} cumplen 70 años. Conviene planificar reemplazos y traspaso de conocimiento.»; «Más casos: …»; con 0: «Nadie llega a la edad de jubilación este año ni el próximo.» ✎ |
| Movimientos | «Entradas y salidas de personal por mes»; «Número de personas»; «Siempre muestra los últimos 6 meses · sí responde a Gerencia, Sede y Tipo»; leyenda «Entraron», «Salieron»; tooltip «{Mes año}», «Entraron: {n}», «Salieron: {n}», «Variación: {±n} personas» |
| Por gerencia | «Personas en planilla por gerencia»; «Haz clic en una barra para filtrar todo el tablero»; «{Mes año} · {n} personas sin gerencia en la base de personal no se muestran» (sin la segunda parte si n = 0); `aria-label` de cada barra «{gerencia}: {n} personas» |
| Ausencias | «Días de ausencia en {mes}, por motivo»; «{días} días · {x} del tiempo programado»; «Fuente: planilla · cada columna de días se cuenta una sola vez»; filas «{motivo}» y «{días} · {pct}» |
| En cada gráfico | «Ver como tabla» / «Ver como gráfico» ✎; «Descargar CSV» ✎; vacío: «Sin datos para este filtro.» ✎ |
| Sin datos | «Todavía no hay datos para el tablero» / «Procesa la planilla o la base de personal en la app y vuelve aquí.» ✎ |
| Error | «No se pudo cargar el tablero» + mensaje de la API + «Volver a intentar» ✎ |
| Filtro inválido | «El filtro «{valor}» ya no existe en los datos; se muestran todos.» ✎ |
| Alertas | título «Alertas (solo RR. HH.)»; aviso «Esta pestaña muestra nombres. Es solo para RR. HH.: no la compartas.» ✎; secciones «Personas con alertas ({n})» y «Personas de la planilla sin base de personal ({n})» ✎ |
| Cómo leer | todo el texto del mock (Tarea 8) |
| App: Inicio | botón principal «Ver el tablero»; desactivado: «Procesa un reporte primero.» ✎ |
| App: Listo | botón principal «Ver en el tablero»; «Abrir en Excel» pasa a secundario |
| Impresión | título «Resumen RR. HH. · {Mes año}» ✎; «Filtros: {lista}» ✎; pie «Generado el {fecha} a las {hh:mm}» ✎ |

## Foco de revisión

1. **Todavía no hay datos o solo hay base de personal.** Si falta `actual/`, el tablero muestra «Todavía no hay
   datos…». Si no hay planilla procesada, las tarjetas de planilla muestran «—» y los gráficos de planilla, su
   vacío, sin romperse. Tests en las Tareas 4 y 5.
2. **Un solo mes de planilla.** No hay comparación: IND-04 muestra «—» con la nota de la API y el gráfico de
   movimientos tiene una sola barra por serie. Tests en las Tareas 5 y 6.
3. **Un filtro que deja a cero personas** (gerencia y sede sin cruce). Las tarjetas muestran 0 o «—», nunca
   `NaN`, y los gráficos muestran «Sin datos para este filtro.». Tests en las Tareas 5 y 6.
4. **Un enlace guardado con un filtro que ya no existe** (se reprocesó y cambió una gerencia). La API responde
   400: el tablero quita los filtros de dimensión, avisa y muestra todo. Test en la Tarea 4.
5. **Se reprocesa con el tablero abierto.** La API puede responder un error que pide reintentar mientras se
   reemplaza `actual/`: el tablero reintenta una vez y, al volver el foco, muestra los datos nuevos. Test en la
   Tarea 4.

---

### Tarea 1: Desglose por gerencia en la API

**Archivos:**
- Crear: `config/sql/indicadores/35_por_gerencia.sql`
- Modificar: `crates/core/src/tablero.rs` (carga del SQL nuevo y armado de la respuesta),
  `crates/core/src/tablero/respuesta.rs` (`Indicador.por_gerencia`)
- Tests: `crates/core/tests/tablero.rs`, `crates/api/tests/api.rs`

**Interfaces:**
- Produce: `Indicador.por_gerencia: Vec<Parte>` (`categoria` = `etiqueta` = nombre de la gerencia, `valor` =
  personas), ordenado por `valor` descendente y luego por nombre. Vacío en todo indicador que no sea IND-01,
  IND-07 o IND-08.

- [ ] **Paso 1: tests que fallan** en `crates/core/tests/tablero.rs`, con los helpers del archivo:

```rust
#[test]
fn por_gerencia_de_ind01_mas_sin_base_da_el_total() {
    let r = tablero().indicadores(&Filtros::default()).unwrap();
    let c1 = tablero().checks(&Filtros::default()).unwrap().checks
        .into_iter().find(|c| c.id == "IND-C1").unwrap().valor.unwrap();
    let ind01 = ind(&r, "IND-01");
    let suma: f64 = ind01.por_gerencia.iter().map(|p| p.valor).sum();
    assert_eq!(suma + c1, ind01.valor.unwrap());
    assert!(ind01.por_gerencia.windows(2).all(|w| w[0].valor >= w[1].valor));
}

#[test]
fn por_gerencia_de_alertas_suma_el_total() {
    let r = tablero().indicadores(&Filtros::default()).unwrap();
    for id in ["IND-07", "IND-08"] {
        let i = ind(&r, id);
        assert_eq!(i.por_gerencia.iter().map(|p| p.valor).sum::<f64>(), i.valor.unwrap(), "{id}");
    }
    assert!(ind(&r, "IND-04").por_gerencia.is_empty());
}

#[test]
fn por_gerencia_respeta_el_filtro_de_gerencia() {
    let g = tablero().filtros().unwrap().gerencias[0].clone();
    let f = Filtros { gerencia: Some(g.clone()), ..Default::default() };
    let i = ind(&tablero().indicadores(&f).unwrap(), "IND-01").clone();
    assert_eq!(i.por_gerencia.len(), 1);
    assert_eq!(i.por_gerencia[0].categoria, g);
    assert_eq!(i.por_gerencia[0].valor, i.valor.unwrap());
}
```

  En `crates/api/tests/api.rs`, `indicadores_traen_por_gerencia`: `GET /api/v1/indicadores` → cada elemento de
  `indicadores` tiene la clave `por_gerencia` y la de IND-01 no está vacía.

- [ ] **Paso 2:** `cargo test -p etl-local-core --test tablero por_gerencia -j 2` → falla por el campo inexistente.
- [ ] **Paso 3: implementar.**
  - `35_por_gerencia.sql` crea `ind_por_gerencia(id, gerencia, valor)`:
    - IND-01: personas distintas de la planilla del periodo filtrado, unidas a la base, agrupadas por `gerencia`.
      Quien no tiene registro en la base no entra (eso es IND-C1).
    - IND-07 e IND-08: conteo de `alerta_contrato_plazo` y `alerta_jubilacion` en `empleados_f`, por `gerencia`.
    - Gerencia nula o vacía: fuera.
  - Se ejecuta en el mismo orden y con el mismo mecanismo que `30_alertas.sql`, después de él.
  - `tablero.rs` llena `por_gerencia` desde esa tabla con la consulta parametrizada que ya usan los desgloses.
- [ ] **Paso 4:** `cargo test -p etl-local-core --test tablero -j 2` y `cargo test -p etl-local-api -j 2` → todo verde.
- [ ] **Paso 5:** ledger. Anotar en [indicadores-tablero.md §7](../../logica-negocio/indicadores-tablero.md) la
  fila «Desglose por gerencia | `config/sql/indicadores/35_por_gerencia.sql`».

### Tarea 2: Esquema JSON y tipos TypeScript derivados

**Archivos:**
- Modificar: `Cargo.toml` (`schemars = { version = "1.2", features = ["chrono04"] }`),
  `crates/core/src/tablero/respuesta.rs`, `crates/core/src/tablero.rs` (`Filtros`, `Detalle`),
  `crates/core/src/proceso.rs` (`Fuente`), `crates/core/src/preflight.rs` (`FechaNombre`),
  `crates/api/src/lib.rs` (el cuerpo de error pasa a `respuesta.rs` como `ErrorApi`)
- Crear: `crates/core/src/tablero/esquema.rs`, `crates/core/tests/esquema_api.rs`,
  `app/ui/src/lib/api/esquema.json` (generado), `app/ui/src/lib/api/tipos.gen.ts` (generado),
  `app/ui/scripts/tipos.mjs`, `app/ui/src/lib/api/tipos.test.ts`
- App: `app/src-tauri/src/esquema.rs` (test), `app/ui/src/app/esquema.json` y `app/ui/src/app/tipos.gen.ts` (generados)
- Borrar (los creó el Plan 6, los reemplaza este mecanismo): `app/src-tauri/src/ejemplos.rs`,
  `app/src-tauri/tests/ejemplos/`, `app/ui/src/app/tipos.test.ts` y la constante `CAMPOS` de `tipos.ts`

**Interfaces:**
- Produce:
  - `etl_local_core::tablero::esquema::esquema_api() -> serde_json::Value`: un esquema con `$defs` para
    `RespuestaIndicadores`, `Estado`, `OpcionesFiltro`, `RespuestaSeries`, `RespuestaChecks`,
    `RespuestaDetalle`, `Filtros` y `ErrorApi`, y sus dependencias;
  - `app/ui/src/lib/api/tipos.gen.ts` exporta una interfaz por cada uno de esos nombres;
  - `app/ui/src/app/tipos.gen.ts`: lo mismo para `Inicio`, `ArchivoElegido`, `RevisionArchivo`, `Avance`,
    `Resultado` y `ErrorApp` de la app. `app/ui/src/app/tipos.ts` los re-exporta y conserva sus utilidades
    (`textoFecha`, etc.).
  - Script npm `"tipos": "node scripts/tipos.mjs"`, que regenera los dos `.gen.ts`, y su función exportada
    `generar(esquema: object, nombre: string): Promise<string>`.

- [ ] **Paso 1: tests que fallan.**
  - `crates/core/tests/esquema_api.rs::esquema_api_al_dia`: con `ACTUALIZAR_ESQUEMA=1` escribe
    `app/ui/src/lib/api/esquema.json` (JSON con sangría y `\n` final); sin la variable, compara con el archivo,
    ignorando `\r`. El mensaje de fallo dice que hay que correr `ACTUALIZAR_ESQUEMA=1` y luego `npm --prefix app/ui run tipos`.
  - Mismo patrón en `app/src-tauri/src/esquema.rs` (`#[cfg(test)]`), con `app/ui/src/app/esquema.json`.
  - `tipos.test.ts`: por cada par (esquema, `.gen.ts`), `await generar(esquema, nombre)` es igual al contenido
    del archivo. Se leen con `import.meta.glob(..., { query: "?raw", eager: true })`, como en el Plan 6.
- [ ] **Paso 2:** `cargo test -p etl-local-core --test esquema_api -j 2` → no compila (`esquema_api` no existe).
- [ ] **Paso 3: implementar.**
  - `#[derive(JsonSchema)]` en cada tipo de respuesta y sus dependencias. `NaiveDate` sale como
    `string`/`date` gracias a `chrono04`.
  - `esquema_api()` arma el documento con `schemars::generate::SchemaSettings::draft2020_12()` y un
    `SchemaGenerator`, una entrada por tipo en `$defs`.
  - `scripts/tipos.mjs` usa `compile` de `json-schema-to-typescript` con
    `{ bannerComment: "// Generado desde el esquema de Rust: no editar. Regenerar con `npm run tipos`.", unreachableDefinitions: true, additionalProperties: false }`.
  - Instalar `json-schema-to-typescript` como `devDependency` con versión exacta (la última estable a la fecha
    de ejecución, `npm view json-schema-to-typescript version`). Registrar la versión en el ledger.
  - Correr `ACTUALIZAR_ESQUEMA=1` para los dos esquemas y luego `npm run tipos`.
  - El cliente del Plan 6 (`backend.ts`, pantallas) importa los tipos generados. Las diferencias de forma
    (por ejemplo, `null` frente a campo opcional) se corrigen en el código de la UI, no en el esquema.
- [ ] **Paso 4:** `cargo test -j 2`, `cargo test -p etl-local-app --lib -j 2`, `npm --prefix app/ui test` y
  `npm --prefix app/ui run check` → todo verde.
- [ ] **Paso 5:** ledger. En la nota del Plan 6 de ADR-0005, agregar: «Plan 7: los tipos se derivan con schemars
  + json-schema-to-typescript (`npm run tipos`)». En AGENTS.md §6, el comando para regenerar los tipos.

### Tarea 3: La API sirve el tablero

**Archivos:**
- Modificar: `crates/api/src/lib.rs`, `crates/cli/src/tablero.rs`, `crates/cli/tests/cli.rs`
- Test: `crates/api/tests/api.rs`

**Interfaces:**
- Produce:
  - `etl_local_api::aplicacion(tablero: Tablero, puerto: u16, ui: Option<PathBuf>) -> Router`: el router de la
    API más los archivos estáticos;
  - `etl_local_api::servir(listener: TcpListener, tablero: Tablero, ui: Option<PathBuf>, apagado: impl Future<Output = ()> + Send + 'static) -> io::Result<()>`.
    La CLI pasa `ctrl_c`; la app, `std::future::pending()`.
  - CLI: `etl-local tablero --ui <carpeta>`. Sin la opción, usa `app/ui/dist` si contiene `tablero.html`; si
    no, `<carpeta del ejecutable>/ui`; si tampoco, sirve solo la API.

- [ ] **Paso 1: tests que fallan** en `api.rs`. Una carpeta temporal `ui/` con `tablero.html`,
  `assets/t.js`, `app.html` y un `secreto.txt` **fuera** de ella:
  - `raiz_sirve_el_tablero`: `GET /` → 200, `text/html; charset=utf-8`, con el contenido de `tablero.html`.
  - `asset_con_su_tipo`: `GET /assets/t.js` → 200 y `text/javascript`.
  - `no_sirve_app_html`: `GET /app.html` → 404.
  - `no_sale_de_ui`: `GET /..%2fsecreto.txt`, `GET /assets/..%5c..%5csecreto.txt` y `GET /%2e%2e/secreto.txt`
    → 404, y el cuerpo no contiene el texto del secreto.
  - `html_con_csp`: la respuesta de `/` trae
    `Content-Security-Policy: default-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self'; connect-src 'self'; frame-ancestors 'none'`
    y `X-Content-Type-Options: nosniff`.
  - `sin_ui_la_raiz_explica`: con `ui = None`, `GET /` → 404 JSON con `codigo` `sin_interfaz` y un mensaje que
    menciona `npm --prefix app/ui run build`.
  - `host_ajeno_tambien_en_estaticos`: `GET /` con `Host: evil.example` → 403.
  - En `crates/cli/tests/cli.rs`, `tablero_ayuda_menciona_ui`: `etl-local tablero --help` contiene `--ui`.
- [ ] **Paso 2:** `cargo test -p etl-local-api -j 2` → falla (`aplicacion` no existe).
- [ ] **Paso 3: implementar** un manejador propio, sin dependencias nuevas.
  - Solo `GET`. Decodifica `%xx`, rechaza segmentos `..` o vacíos y cualquier `\` o `:`. Comprueba con
    `canonicalize` que la ruta final queda dentro de `ui`. `/` equivale a `/tablero.html`; `app.html` da 404.
  - Tipos por extensión: `html`, `js`, `css`, `woff2`, `svg`, `png`, `ico`, `json`; cualquier otra,
    `application/octet-stream`.
  - Encabezados de seguridad (los del test) en todas las respuestas estáticas; `Cache-Control: no-cache` en el HTML.
  - Las rutas `/api/…` siguen como hasta ahora (404 JSON `ruta_inexistente`).
  - La CLI imprime `Tablero en http://127.0.0.1:{puerto}/ (Ctrl+C para salir)`, o la línea de la API más
    `Falta la interfaz: npm --prefix app/ui run build` cuando no encontró la carpeta.
- [ ] **Paso 4:** `cargo test -p etl-local-api -p etl-local-cli -j 2` → verde.
- [ ] **Paso 5:** ledger. En ADR-0004 §Notas: «Plan 7: la API también sirve `app/ui/dist` (sin `app.html`), con CSP propia».

### Tarea 4: Base del tablero (entrada, cliente, estado, encabezado, Datos usados y filtros)

**Archivos:**
- Crear: `app/ui/tablero.html`, `app/ui/src/tablero/main.ts`, `app/ui/src/tablero/Tablero.svelte`,
  `app/ui/src/tablero/api.ts`, `app/ui/src/tablero/estado.svelte.ts`, `app/ui/src/tablero/textos.ts`,
  `app/ui/src/tablero/DatosUsados.svelte`, `app/ui/src/tablero/Filtros.svelte`
- Modificar: `app/ui/vite.config.ts` (entrada `tablero`; `server.proxy` `/api` →
  `http://127.0.0.1:8787` con `changeOrigin: true`), `app/ui/src/lib/componentes/Encabezado.svelte` (prop
  `titulo`, por defecto «Integrador RR. HH.»)
- Tests: `app/ui/src/tablero/estado.test.ts`, `app/ui/src/tablero/textos.test.ts`, `app/ui/src/tablero/api.test.ts`

**Interfaces:**
- Consume: los tipos de `src/lib/api/tipos.gen.ts` (Tarea 2).
- Produce:
  - `interface ClienteApi { estado(); filtros(); indicadores(f: Filtros); series(f: Filtros); checks(f: Filtros); detalle(tipo: "alertas" | "sin_base", f: Filtros) }`,
    cada método devuelve la respuesta tipada;
  - `crearCliente(base = "", pedir: typeof fetch = fetch): ClienteApi`: los filtros van como parámetros de
    consulta, solo los no vacíos;
  - `class FalloApi extends Error { codigo: string; estado: number }`, con el `mensaje` de la API como `message`;
  - `class EstadoTablero`, con *runes*:
    - campos: `filtros`, `pestana: "resumen" | "alertas" | "como-leer"`, `datos: DatosResumen | null`,
      `detalle: { alertas; sinBase } | null`, `vacio: boolean`, `error: FalloApi | null`, `aviso: string | null`,
      `cargando: boolean`;
    - métodos: `constructor(cliente: ClienteApi, url: { leer(): string; escribir(q: string): void }, esperar = (ms: number) => new Promise(r => setTimeout(r, ms)))`,
      `cargar()`, `cambiarFiltro(clave, valor | null)`, `irA(pestana)`;
  - `DatosResumen = { estado; opciones; indicadores; series; checks }`;
  - en `textos.ts`:
    - `porcentaje(f: number | null): string` (`0.0186` → «1,9 %», `null` → «—»);
    - `cifra(v: number | null, unidad: string): string`;
    - `textoComparacion(i: Indicador, mesAnterior: string | null): string | null`;
    - `textoMeta(i: Indicador): string`;
    - `textoFuente(f: Fuente, periodo: string | null): string`.

- [ ] **Paso 1: tests que fallan.**
  - `api.test.ts`: los parámetros vacíos no viajan. Un 400 con cuerpo `{codigo, mensaje}` lanza `FalloApi` con
    ese código, mensaje y estado.
  - `textos.test.ts`:
    - `porcentaje(0.0186)` → «1,9 %», `porcentaje(null)` → «—»;
    - IND-01 con 1.239 contra 1.243 → «4 menos que en octubre»; igual → «Igual que en octubre»;
    - IND-04 con `comparacion: "peor"` y `anterior` 0,009 → «Empeoró: era 0,9 % en octubre»;
    - sin anterior → `null`;
    - `textoMeta`: sin meta → «Sin meta definida»; con 0,015 → «Meta: 1,5 %»;
    - `textoFuente`: planilla → «Planilla: noviembre 2025», base → «Base de personal: 12 nov 2025»,
      licencias → «Licencias: incluidas».
  - `estado.test.ts`, con un cliente falso que registra las llamadas:
    - `cargar()` pide estado, filtros, indicadores, series y checks, y **nunca** `detalle`;
    - los filtros de la URL `?periodo=2025-10&gerencia=Operaciones` llegan al cliente; `cambiarFiltro("sede", "Lima")`
      escribe `periodo=2025-10&gerencia=Operaciones&sede=Lima` y vuelve a cargar;
    - **(foco 4)** un 400 `filtro_invalido` con `gerencia=Vieja` quita gerencia, sede y tipo, conserva el
      periodo, pone el aviso «El filtro «Vieja» ya no existe en los datos; se muestran todos.» y carga de nuevo;
    - **(foco 1)** un 404 `sin_datos` deja `vacio = true` y `error = null`;
    - **(foco 5)** un error con el código que la API usa para «reintentar» (ver `crates/api/src/lib.rs`)
      reintenta una vez tras `esperar(1000)`; si falla dos veces, queda en `error`;
    - `irA("alertas")` pide `detalle` dos veces (alertas y sin_base); `irA("resumen")` no.
- [ ] **Paso 2:** `npm --prefix app/ui test` → fallan los nuevos.
- [ ] **Paso 3: implementar.**
  - `estado.svelte.ts` según la interfaz. La pestaña viaja en el hash (`#alertas`, `#como-leer`) y los filtros en
    la consulta, con `history.replaceState`.
  - `main.ts` monta `Tablero.svelte` con `crearCliente()` y la URL real, y vuelve a cargar en `visibilitychange`
    cuando la página pasa a visible.
  - `Tablero.svelte` muestra:
    - el encabezado con título «Tablero RR. HH.» y los tres botones de «Textos»;
    - las pestañas, con `aria-current="page"` en la activa;
    - `DatosUsados` (`role="status"`) y `Filtros`: `<select>` con `<label>`, opciones de `/filtros`; el periodo
      sin «Todas», los meses con `nombreMes`;
    - los estados vacío, error y aviso con `Aviso.svelte`.
  - El cuerpo de Resumen queda como contenedor para las Tareas 5 y 6.
  - `tablero.html` usa la misma cabecera y estilos que `app.html` y `<title>Tablero RR. HH.</title>`.
- [ ] **Paso 4:** `npm --prefix app/ui test` y `npm --prefix app/ui run check` → verde. Verificar el aislamiento de
  Tauri: tras `npm --prefix app/ui run build`, `grep -l "__TAURI\|@tauri-apps" app/ui/dist/assets/tablero*.js` no
  imprime nada.
- [ ] **Paso 5:** ledger.

### Tarea 5: Tarjetas, «¿De dónde sale?» y Riesgos a atender

**Archivos:**
- Crear: `app/ui/src/tablero/Tarjetas.svelte`, `app/ui/src/tablero/Origen.svelte`,
  `app/ui/src/tablero/Riesgos.svelte`, `app/ui/src/tablero/glosario.ts`
- Modificar: `app/ui/src/tablero/textos.ts`, `app/ui/src/tablero/Tablero.svelte`
- Tests: `app/ui/src/tablero/textos.test.ts`, `app/ui/src/tablero/glosario.test.ts`

**Interfaces:**
- Produce:
  - `GLOSARIO: Record<string, { nombre: string; pregunta: string; calculo: string; fuente: string; nuevo: boolean }>`
    con IND-01…05, IND-07 e IND-08 (texto de la Tarea 8; IND-02 e IND-03 comparten «Entradas / Salidas»);
  - en `textos.ts`:
    - `lineaTarjeta(id: string, d: DatosResumen): string | null`, la segunda línea de cada tarjeta según «Textos»;
    - `textoRiesgo(i: Indicador): { cifra: string; titulo: string; detalle: string; masCasos: string | null }`;
    - `masCasos(partes: Parte[]): string | null`, los 3 primeros y «otras ({resto})» si quedan;
    - `diasDelMes(periodo: string): number`.

- [ ] **Paso 1: tests que fallan.**
  - `lineaTarjeta`:
    - IND-01 → «Octubre: 1.243 · 19 entraron, 23 salieron»;
    - IND-05 con desglose de 1.050 días, IND-01 = 1.239 y periodo `2025-11` → «1.050 días perdidos de 37.170 programados»;
    - IND-07 → «Superan el plazo legal en 6 meses o menos».
  - **(foco 1 y 3)** con IND-01 `valor: null` o `0`, ninguna línea contiene `NaN`, `null` ni `undefined`;
    `cifra(null, …)` → «—».
  - **(foco 2)** IND-04 sin anterior y con `nota` → la tarjeta usa la `nota` en vez de la comparación;
    `textoComparacion` → `null`.
  - `textoRiesgo` de IND-07 con desglose obra 11 e incremento 7 y `por_gerencia` Operaciones 9, Mantenimiento 5,
    Comercial 2, Seguridad 1 y Tecnología 1 → detalle «11 por obra o servicio (máx. 5 años) · 7 por incremento de
    actividad (máx. 3 años). Si se pasan del plazo, pueden considerarse contratos indefinidos.» y masCasos
    «Más casos: Operaciones (9), Mantenimiento (5), Comercial (2), otras (2)». Con valor 1, el título va en
    singular; con 0, el texto de «Textos».
  - `textoRiesgo` de IND-08 con 65 este año 5, 65 el próximo 4, 70 este año 1 y 70 el próximo 3 → «9 cumplen 65
    años · 4 cumplen 70 años. Conviene planificar reemplazos y traspaso de conocimiento.».
  - `masCasos` con 3 partes o menos → sin «otras»; con 0 → `null`.
  - `glosario.test.ts`: cada ID de las tarjetas y de Riesgos tiene entrada en `GLOSARIO`.
- [ ] **Paso 2:** `npm --prefix app/ui test` → fallan.
- [ ] **Paso 3: implementar.**
  - `Tarjetas` muestra 4 tarjetas en este orden: IND-01, IND-04, IND-05, IND-07. Cada una lleva nombre, cifra
    grande, comparación con flecha y palabra (si la hay) y segunda línea.
  - `Origen` es un `<button aria-expanded>` que abre un panel `role="dialog"` con «Fuente» (`origen.fuente` de
    {periodo} o con corte al {fecha}), «Qué cuenta» (`GLOSARIO[id].calculo`) y «Regla» (chips mono con
    `origen.reglas`). En IND-01 agrega la línea de IND-C1, con «Ver quiénes» → `irA("alertas")`, solo si
    IND-C1 > 0. Se cierra con Esc y al hacer clic fuera.
  - `Riesgos` muestra IND-07 e IND-08 con `textoRiesgo`, y «Ver detalle (solo RR. HH.)» → `irA("alertas")`.
- [ ] **Paso 4:** `npm --prefix app/ui test` y `run check` → verde.
- [ ] **Paso 5:** ledger.

### Tarea 6: Gráficos (ECharts y barras HTML), «Ver como tabla» y CSV

**Archivos:**
- Crear: `app/ui/src/lib/graficos/echarts.ts` (registra solo `BarChart`, `GridComponent`, `TooltipComponent`,
  `LegendComponent`, `AriaComponent` y `SVGRenderer` desde `echarts/core`), `app/ui/src/lib/graficos/tema.ts`,
  `app/ui/src/lib/graficos/Grafico.svelte`, `app/ui/src/lib/csv.ts`,
  `app/ui/src/tablero/graficos.ts`, `app/ui/src/tablero/Movimientos.svelte`,
  `app/ui/src/tablero/PorGerencia.svelte`, `app/ui/src/tablero/Ausencias.svelte`,
  `app/ui/src/tablero/TablaDatos.svelte`
- Tests: `app/ui/src/tablero/graficos.test.ts`, `app/ui/src/lib/csv.test.ts`

**Interfaces:**
- Consume: `EstadoTablero.cambiarFiltro` (Tarea 4).
- Produce:
  - `aCsv(encabezados: string[], filas: (string | number | null)[][]): string`: BOM, `;`, CRLF, comillas
    dobles escapadas y números con coma decimal sin separador de miles;
  - `descargar(nombre: string, contenido: string): void`, con un `<a download>` y un `Blob`;
  - `opcionMovimientos(series: RespuestaSeries): EChartsOption | null`: los últimos 6 periodos con datos de
    IND-02 e IND-03; `null` si no hay puntos;
  - `filasGerencia(i: Indicador): { gerencia: string; valor: number; pct: number }[]`;
  - `filasAusencias(i: Indicador): { motivo: string; dias: number; pct: number }[]`: sin las filas en 0, de mayor
    a menor;
  - `<Grafico opcion etiqueta>`: crea la instancia en `onMount`, `setOption` al cambiar, `resize` con
    `ResizeObserver` y en `beforeprint`, `dispose` al desmontar.

- [ ] **Paso 1: tests que fallan.**
  - `csv.test.ts`: `aCsv(["Gerencia","Personas"], [["Op; \"A\"", 1.5]])` →
    `"﻿Gerencia;Personas\r\n\"Op; \"\"A\"\"\";1,5\r\n"`.
  - `graficos.test.ts`:
    - con 8 periodos, `opcionMovimientos` usa los 6 últimos;
    - las series se llaman «Entraron» y «Salieron», con colores `#2B6CB0` y `#C2620F`;
    - el renderizador lo fija `Grafico` (SVG) y `aria.enabled` es `true`;
    - **(foco 2)** con 1 periodo, una categoría por serie;
    - **(foco 3)** sin puntos con valor → `null`;
    - `filasGerencia` ordena y calcula `pct` sobre la suma; con `por_gerencia` vacío → `[]`;
    - `filasAusencias` omite los ceros y sus `pct` suman 100 ± 0,1.
- [ ] **Paso 2:** `npm --prefix app/ui test` → fallan.
- [ ] **Paso 3: implementar.**
  - Instalar `echarts` 6.1.0 exacto (si `npm view echarts@6.1.0 version` falla, la última 6.x, registrada como
    `Ruling:`).
  - Cada tarjeta de gráfico lleva título, subtítulo y leyenda según «Textos».
  - «Ver como tabla» alterna con `TablaDatos`, una `<table>` con `<caption>` y su «Descargar CSV».
  - `PorGerencia` dibuja un `<button>` por barra: el ancho es su `pct` y el clic llama
    `cambiarFiltro("gerencia", g)`.
  - `Ausencias` usa barras `#1E6B47`.
  - Sin datos, cada gráfico muestra «Sin datos para este filtro.».
  - `tema.ts` toma colores y tipografías de `tokens.css` (los mismos valores), sin modo oscuro (pendiente en diseno).
- [ ] **Paso 4:** `npm --prefix app/ui test`, `run check` y `run build` → verde.
- [ ] **Paso 5:** ledger. Anotar en `docs/diseno/README.md` (Implementado) que por gerencia y ausencias son
  barras HTML (decisión 1).

### Tarea 7: Pestaña «Alertas (solo RR. HH.)»

**Archivos:**
- Crear: `app/ui/src/tablero/Alertas.svelte`, `app/ui/src/tablero/alertas.ts`
- Test: `app/ui/src/tablero/alertas.test.ts`

**Interfaces:**
- Consume: `EstadoTablero.detalle` (Tarea 4), `aCsv` y `descargar` (Tarea 6).
- Produce: `textoAlertas(fila: Record<string, unknown>): string[]` y `COLUMNAS_ALERTAS` / `COLUMNAS_SIN_BASE`
  (`{ clave, titulo }[]`).

- [ ] **Paso 1: test que falla.**
  - `textoAlertas` de una fila con `alerta_contrato_obra: true` y `cumple_65_proximo_anio: true` →
    `["Contrato por obra o servicio por superar su plazo", "Cumple 65 el próximo año"]`.
  - Las columnas de alertas son, en orden: Documento, Nombre, Gerencia, Sede, Tipo de trabajador, Alertas, Fin
    de contrato y Nacimiento (fechas `dd/mm/aaaa`).
  - Las de sin base: Documento, Apellidos y nombres, Área y Centro de costo.
- [ ] **Paso 2:** `npm --prefix app/ui test` → falla.
- [ ] **Paso 3: implementar** `Alertas.svelte`:
  - el aviso de privacidad de «Textos»;
  - las dos secciones, cada una con su tabla (encabezado fijo y desplazamiento propio) y «Descargar CSV»;
  - los filtros de dimensión aplican; en «sin base», el periodo.
  - En impresión, la pestaña entera queda oculta (`@media print { display: none }`).
- [ ] **Paso 4:** `npm --prefix app/ui test` y `run check` → verde.
- [ ] **Paso 5:** ledger.

### Tarea 8: «Cómo leer este tablero» (pantalla 7)

**Archivos:**
- Crear: `app/ui/src/tablero/ComoLeer.svelte`
- Modificar: `app/ui/src/tablero/glosario.ts` (textos), `app/ui/src/tablero/textos.ts`
- Test: `app/ui/src/tablero/glosario.test.ts`

**Interfaces:**
- Produce:
  - `ejemplo(id: "IND-04" | "IND-05", d: DatosResumen | null): string | null`: «Este mes: 23 ÷ 1.241 = 1,9 %»
    con los datos vigentes; `null` sin datos;
  - `ALCANCE_FILTROS: { grafico: string; periodo: string; gerencia: boolean; sede: boolean; tipo: boolean }[]`.

- [ ] **Paso 1: tests que fallan.**
  - `ejemplo("IND-04", d)` usa IND-03, el promedio de IND-01 actual y anterior, y el valor de IND-04.
  - `ejemplo("IND-05", d)` usa los días de ausencia, los días programados y el valor.
  - Con IND-04 nulo → `null`.
  - `ALCANCE_FILTROS` tiene una fila por cada sección de Resumen, con los títulos exactos de «Textos».
- [ ] **Paso 2:** `npm --prefix app/ui test` → fallan.
- [ ] **Paso 3: implementar** con el texto del mock:
  - Título «Cómo leer este tablero» y bajada «Cada número responde una pregunta. Aquí está cuál, de dónde sale y
    cómo se calcula. Los marcados como Nuevo no existían en el reporte anterior.». Botón «Volver al tablero».
  - **Indicadores** (`<dl>`), con «Cálculo» y «Fuente» por entrada:
    - Personas en planilla (Nuevo): «Personas distintas en la planilla del mes, contadas por número de documento.» · Planilla · IND-01.
    - Entradas / Salidas (Nuevo): «Entradas: personas cuya fecha de entrada cae dentro del mes. Salidas: personas cuya fecha de salida cae dentro del mes.» · Planilla · IND-02, IND-03.
    - Rotación del mes (Nuevo): «Salidas del mes ÷ promedio de personas en planilla de este mes y el anterior.» + `ejemplo` ✎ · Planilla · IND-04.
    - Ausentismo (Nuevo): «Días de ausencia (descansos médicos, licencias y subsidios) ÷ días programados (personas × días del mes).» + `ejemplo` ✎ · Planilla · IND-05.
    - Contratos por superar su plazo: «Por obra o servicio: 4 años y medio o más de servicio (máximo 5). Por incremento de actividad: 2 años y medio o más (máximo 3).» · Base de personal · BD-FLG-08, BD-FLG-09.
    - Edad de jubilación: «Según el año de nacimiento.» Agregar «Por ahora no usa el día exacto del cumpleaños; está en revisión.» solo si TR-D15 lo sigue diciendo; si no, `Ruling:`. · Base de personal · BD-FLG-05 a BD-FLG-07.
  - **«¿Por qué algunas personas no aparecen al filtrar por gerencia?»**, con los dos párrafos del mock.
  - **«Qué filtros afectan a cada gráfico»**: tabla con columnas Gráfico, Periodo, Gerencia, Sede y Tipo de
    trabajador, y valores «Sí» / «No (…)» en texto, no solo color. Filas:
    - Indicadores de arriba: Sí;
    - Riesgos a atender: «No (última base de personal)» ✎;
    - Entradas y salidas de personal por mes: «No (últimos 6 meses)»;
    - Personas en planilla por gerencia ✎: Sí;
    - Días de ausencia por motivo: Sí.
- [ ] **Paso 4:** `npm --prefix app/ui test` y `run check` → verde.
- [ ] **Paso 5:** ledger.

### Tarea 9: Resumen en PDF y «Datos en Excel»

**Archivos:**
- Crear: `app/ui/src/tablero/impresion.css`
- Modificar: `app/ui/src/tablero/Tablero.svelte`, `app/ui/src/tablero/textos.ts`
- Test: `app/ui/src/tablero/textos.test.ts`

**Interfaces:**
- Produce:
  - `csvIndicadores(d: DatosResumen): string`, con columnas Indicador, Valor, Mes anterior y Unidad; una fila
    por IND-01…08; tasas en %;
  - `lineaFiltros(f: Filtros, opciones): string`, por ejemplo «Periodo: noviembre 2025 · Gerencia: Todas · Sede:
    Todas · Tipo de trabajador: Todos».

- [ ] **Paso 1: tests que fallan.**
  - `csvIndicadores` empieza con BOM, tiene 9 líneas y no contiene ninguna clave del detalle nominal
    (`numero_documento`, `nombre_completo`, `nombres`).
  - `lineaFiltros` con filtros vacíos → el texto de ejemplo de la interfaz.
- [ ] **Paso 2:** `npm --prefix app/ui test` → fallan.
- [ ] **Paso 3: implementar.**
  - «Resumen en PDF (1 página)»: si hace falta, `irA("resumen")`, espera un `tick()` y llama a `window.print()`.
  - «Datos en Excel» → `descargar("tablero-{periodo}.csv", csvIndicadores(d))`.
  - `impresion.css` (`@media print`, `@page { size: A4 portrait; margin: 12mm }`):
    - oculta encabezado, pestañas, selects, botones, «¿De dónde sale?», tablas y Alertas;
    - muestra un título solo para impresión, `lineaFiltros` y el pie de «Textos»;
    - tarjetas en 4 columnas y gráficos en una grilla 2×2 con `break-inside: avoid`;
    - todo entra en una página A4.
- [ ] **Paso 4: verificar en una página.**
  - `npm --prefix app/ui test` → verde.
  - Con `npm --prefix app/ui run build` y `etl-local tablero --salida <salida demo> --ui app/ui/dist --puerto 0`
    (la salida demo se arma como en la Tarea 6 del Plan 5), generar con Playwright `page.pdf({ format: "A4" })`.
  - Contar las páginas con `/Type\s*\/Page[^s]/g` → **1**. Guardar una captura del PDF en el workspace del plan.
- [ ] **Paso 5:** ledger.

### Tarea 10: El tablero en la app de escritorio

**Archivos:**
- Modificar:
  - `app/src-tauri/Cargo.toml`: `etl-local-api` (ruta del workspace) y `tokio` (workspace, con `net`);
  - `app/src-tauri/src/main.rs`, `app/src-tauri/src/comandos.rs` (`abrir_tablero`),
    `app/src-tauri/src/servicio.rs` (`url_tablero`, `puerto_api`);
  - `app/src-tauri/tauri.conf.json` (`bundle.resources` suma `"../ui/dist/": "ui/"`);
  - `app/ui/src/app/backend.ts`, `app/ui/src/app/simulado.ts`, `app/ui/src/app/pantallas/Inicio.svelte`,
    `app/ui/src/app/pantallas/Listo.svelte`, `app/ui/src/lib/componentes/Icono.svelte` (ícono `tablero`)
- Tests: `app/src-tauri/src/servicio.rs` (tests), `app/ui/src/app/flujo.test.ts`

**Interfaces:**
- Produce:
  - `servicio::puerto_api(debug: bool, entorno: Option<String>) -> u16`: en debug, `ETL_LOCAL_PUERTO` o 8787; en
    release, 0;
  - `servicio::url_tablero(base: &str, pestana: Option<&str>) -> String`: `{base}/tablero.html`, más
    `#{pestana}` si viene;
  - comando `abrir_tablero(pestana: Option<String>)`: abre o enfoca la ventana `tablero`;
  - `Backend.abrirTablero(pestana?: "alertas"): Promise<void>`.

- [ ] **Paso 1: tests que fallan.**
  - Rust:
    - `puerto_api(true, None)` → 8787, `puerto_api(true, Some("9000"))` → 9000, `puerto_api(false, Some("9000"))` → 0;
    - `url_tablero("http://127.0.0.1:5000", None)` → `http://127.0.0.1:5000/tablero.html`;
    - con `Some("alertas")` → `…/tablero.html#alertas`.
  - `flujo.test.ts`:
    - en Listo, el botón principal es «Ver en el tablero» y llama a `abrirTablero()`;
    - en Inicio, con todas las tarjetas en `nunca`, «Ver el tablero» está desactivado con el motivo «Procesa un
      reporte primero.» (por `puedeVerTablero`, un getter nuevo de `Flujo`).
- [ ] **Paso 2:** `cargo test -p etl-local-app --lib -j 2` y `npm --prefix app/ui test` → fallan.
- [ ] **Paso 3: implementar.**
  - En `setup`, abrir un `std::net::TcpListener` en `127.0.0.1:{puerto_api}`. Si el 8787 de debug está ocupado,
    usar 0 y avisar por consola.
  - Lanzar `etl_local_api::servir` con `tauri::async_runtime::spawn`, con
    `ui = Some(resource_dir/ui)` en release y `None` en debug.
  - Guardar en `Estado` la base del tablero: `http://localhost:5173` en debug, `http://127.0.0.1:{puerto}` en release.
  - Si la ventana `tablero` existe, `abrir_tablero` navega a la URL y la enfoca. Si no, la crea con
    `WebviewWindowBuilder` (`WebviewUrl::External`), título «Tablero RR. HH.», 1440×900, mínimo 960×700.
    Esa ventana no tiene capacidades de Tauri.
  - En Inicio, el botón principal con ícono va junto al título, como en el mock. En Listo, «Ver en el tablero»
    pasa a ser el principal y «Abrir en Excel» y «Abrir carpeta», los secundarios.
- [ ] **Paso 4:** `cargo test -p etl-local-app --lib -j 2`, `cargo clippy --workspace --all-targets -j 2 -- -D warnings`,
  `npm --prefix app/ui test` y `run check` → verde.
- [ ] **Paso 5:** ledger. En `docs/diseno/README.md`, quitar «Pendiente hasta el Plan 7» y anotar la decisión 5.

### Tarea 11: Documentación, prueba manual e instalador

**Archivos:**
- Modificar: `AGENTS.md` §6 (`etl-local tablero --ui`, `npm run tipos`, dev del tablero: `etl-local tablero` +
  `npm --prefix app/ui run dev` → `http://localhost:5173/tablero.html`), `docs/diseno/README.md` («Implementado
  (Plan 7)»), ADR-0005 §Notas, `docs/superpowers/plans/2026-11-16-hoja-de-ruta.md` (Plan 7 → Completado)

- [ ] **Paso 1: suites completas.** Una a la vez:
  - `cargo test -j 2`;
  - `cargo test -p etl-local-app --lib -j 2`;
  - `cargo clippy --workspace --all-targets -j 2 -- -D warnings`;
  - `cargo fmt --all --check`;
  - `npm --prefix app/ui test`, `run check` y `run build`.
  - Revisión de confidencialidad con el comando de AGENTS.md sobre los archivos nuevos y `app/ui/dist/` →
    sin coincidencias.
- [ ] **Paso 2: prueba manual en el navegador.** `etl-local tablero` sobre la salida demo; con Playwright,
  capturas de:
  - Resumen;
  - un filtro por gerencia aplicado con clic en la barra;
  - «¿De dónde sale?» de IND-01;
  - Alertas;
  - Cómo leer;
  - la vista de impresión;
  - un enlace con `?gerencia=NoExiste` (foco 4);
  - una salida vacía (foco 1).
  Registrar los tiempos de carga en debug.
- [ ] **Paso 3: prueba manual en la app real** (`tauri dev` con WebView2 `--remote-debugging-port=9222`, como en el
  Plan 6): «Ver el tablero» desde Inicio y desde Listo; reprocesar con el tablero abierto y volver a la
  ventana (foco 5). Cada hallazgo se corrige con un test que falle primero.
- [ ] **Paso 4:** revisión final del plan completo (revisor nuevo, solo lectura, sin compilar) y una pasada de
  correcciones.
- [ ] **Paso 5: instalador**, solo, en segundo plano, con `-j 1`: `npm --prefix app/ui run tauri -- build`.
  Comprobar que el instalado abre el tablero sin Vite (la carpeta `ui/` está entre los recursos).
- [ ] **Paso 6:** hoja de ruta → Plan 7 **Completado**. Mensaje final con todas las líneas `Ruling:` y los menores
  diferidos.
