//! Base DuckDB en memoria: carga las tablas leídas, ejecuta el SQL de `config/sql/` y exporta Parquet.

use std::path::Path;

use chrono::{Datelike, NaiveDate};
use duckdb::types::Value;
use duckdb::{Connection, appender_params_from_iter};

use crate::contrato::TipoDato;
use crate::error::ErrorProceso;
use crate::lectura::{TablaHoja, Valor};
use crate::normalizar::normalizar_cabecera;
use crate::parametros::Parametros;

/// Misma regla que `normalizar_cabecera` (TR-D1), disponible en SQL (TR-D24).
const MACRO_NORMALIZAR: &str = r"CREATE MACRO normalizar(x) AS upper(trim(regexp_replace(strip_accents(x), '\s+', ' ', 'g')));";

pub struct Almacen {
    conexion: Connection,
}

fn error(donde: impl Into<String>) -> impl FnOnce(duckdb::Error) -> ErrorProceso {
    let donde = donde.into();
    move |e| ErrorProceso::Duckdb {
        donde,
        detalle: e.to_string(),
    }
}

/// Identificador SQL entre comillas dobles.
fn id(nombre: &str) -> String {
    format!("\"{}\"", nombre.replace('"', "\"\""))
}

/// Literal de texto SQL entre comillas simples.
fn texto_sql(texto: &str) -> String {
    format!("'{}'", texto.replace('\'', "''"))
}

/// Ruta como literal SQL (barras normales, también en Windows).
fn ruta_sql(ruta: &Path) -> String {
    texto_sql(&ruta.display().to_string().replace('\\', "/"))
}

fn tipo_sql(tipo: TipoDato) -> &'static str {
    match tipo {
        TipoDato::Texto | TipoDato::Documento | TipoDato::Periodo => "VARCHAR",
        TipoDato::Entero => "BIGINT",
        TipoDato::Decimal => "DOUBLE",
        TipoDato::Fecha => "DATE",
        TipoDato::Booleano => "BOOLEAN",
    }
}

fn tipo_de_valor(valor: &Valor) -> &'static str {
    match valor {
        Valor::Nulo | Valor::Texto(_) => "VARCHAR",
        Valor::Entero(_) => "BIGINT",
        Valor::Decimal(_) => "DOUBLE",
        Valor::Fecha(_) => "DATE",
        Valor::Booleano(_) => "BOOLEAN",
    }
}

fn dias_desde_1970(fecha: NaiveDate) -> i32 {
    fecha.num_days_from_ce()
        - NaiveDate::from_ymd_opt(1970, 1, 1)
            .unwrap()
            .num_days_from_ce()
}

fn a_duckdb(valor: &Valor) -> Value {
    match valor {
        Valor::Nulo => Value::Null,
        Valor::Texto(t) => Value::Text(t.clone()),
        Valor::Entero(n) => Value::BigInt(*n),
        Valor::Decimal(n) => Value::Double(*n),
        Valor::Fecha(f) => Value::Date32(dias_desde_1970(*f)),
        Valor::Booleano(b) => Value::Boolean(*b),
    }
}

fn a_json(valor: Value) -> serde_json::Value {
    use serde_json::Value as J;
    let numero = |n: f64| serde_json::Number::from_f64(n).map_or(J::Null, J::Number);
    match valor {
        Value::Null => J::Null,
        Value::Boolean(b) => J::Bool(b),
        Value::TinyInt(n) => J::from(n),
        Value::SmallInt(n) => J::from(n),
        Value::Int(n) => J::from(n),
        Value::BigInt(n) => J::from(n),
        // `sum` de enteros devuelve HUGEINT en DuckDB.
        Value::HugeInt(n) => i64::try_from(n).map_or_else(|_| numero(n as f64), J::from),
        Value::Decimal(d) => d.to_string().parse::<f64>().map_or(J::Null, numero),
        Value::UTinyInt(n) => J::from(n),
        Value::USmallInt(n) => J::from(n),
        Value::UInt(n) => J::from(n),
        Value::UBigInt(n) => J::from(n),
        Value::Float(n) => numero(f64::from(n)),
        Value::Double(n) => numero(n),
        Value::Text(t) => J::String(t),
        Value::Date32(d) => J::String(
            (NaiveDate::from_ymd_opt(1970, 1, 1).unwrap() + chrono::TimeDelta::days(i64::from(d)))
                .format("%Y-%m-%d")
                .to_string(),
        ),
        otro => J::String(format!("{otro:?}")),
    }
}

impl Almacen {
    /// Base en memoria con la macro `normalizar(x)`.
    pub fn nuevo() -> Result<Almacen, ErrorProceso> {
        let conexion = Connection::open_in_memory().map_err(error("abrir DuckDB"))?;
        conexion
            .execute_batch(MACRO_NORMALIZAR)
            .map_err(error("crear la macro normalizar"))?;
        Ok(Almacen { conexion })
    }

    /// Crea `nombre` con las columnas de la tabla leída, más `constantes` y
    /// `archivo_origen`/`fila_origen` (trazabilidad, TR-D17).
    pub fn cargar(
        &self,
        nombre: &str,
        tabla: &TablaHoja,
        archivo: &str,
        constantes: &[(&str, Valor)],
    ) -> Result<(), ErrorProceso> {
        let mut columnas: Vec<String> = tabla
            .columnas
            .iter()
            .map(|c| format!("{} {}", id(&c.nombre), tipo_sql(c.tipo)))
            .collect();
        columnas.extend(
            constantes
                .iter()
                .map(|(n, v)| format!("{} {}", id(n), tipo_de_valor(v))),
        );
        columnas.push("archivo_origen VARCHAR".into());
        columnas.push("fila_origen BIGINT".into());
        let donde = format!("cargar la tabla {nombre}");
        self.conexion
            .execute_batch(&format!(
                "CREATE TABLE {} ({});",
                id(nombre),
                columnas.join(", ")
            ))
            .map_err(error(donde.clone()))?;

        let mut appender = self
            .conexion
            .appender(nombre)
            .map_err(error(donde.clone()))?;
        for (i, fila) in tabla.filas_origen.iter().enumerate() {
            let mut valores: Vec<Value> = tabla
                .columnas
                .iter()
                .map(|c| a_duckdb(&c.valores[i]))
                .collect();
            valores.extend(constantes.iter().map(|(_, v)| a_duckdb(v)));
            valores.push(Value::Text(archivo.to_string()));
            valores.push(Value::BigInt(i64::from(*fila)));
            appender
                .append_row(appender_params_from_iter(valores))
                .map_err(error(donde.clone()))?;
        }
        appender.flush().map_err(error(donde))
    }

    /// `CREATE TABLE destino AS SELECT * FROM a UNION ALL BY NAME SELECT * FROM b …`
    pub fn unir(&self, destino: &str, origenes: &[String]) -> Result<(), ErrorProceso> {
        let consulta = origenes
            .iter()
            .map(|o| format!("SELECT * FROM {}", id(o)))
            .collect::<Vec<_>>()
            .join(" UNION ALL BY NAME ");
        self.ejecutar(&format!("CREATE TABLE {} AS {consulta};", id(destino)))
    }

    /// Variables de DuckDB que usa el SQL (TR-D10, TR-D3). Las modalidades van normalizadas (TR-D24).
    pub fn fijar_variables(
        &self,
        p: &Parametros,
        fecha_corte: Option<NaiveDate>,
    ) -> Result<(), ErrorProceso> {
        let fecha = fecha_corte.map_or("NULL::DATE".to_string(), |f| {
            format!("DATE '{}'", f.format("%Y-%m-%d"))
        });
        let norm = |t: &str| texto_sql(&normalizar_cabecera(t));
        let sql = [
            format!("SET VARIABLE fecha_corte = {fecha};"),
            format!("SET VARIABLE edad_jubilacion = {};", p.jubilacion.edad),
            format!(
                "SET VARIABLE edad_jubilacion_extendida = {};",
                p.jubilacion.edad_extendida
            ),
            format!(
                "SET VARIABLE modalidad_obra = {};",
                norm(&p.contrato_obra.modalidad)
            ),
            format!(
                "SET VARIABLE anios_alerta_obra = {}::DOUBLE;",
                p.contrato_obra.anios_alerta
            ),
            format!(
                "SET VARIABLE modalidad_incremento = {};",
                norm(&p.contrato_incremento.modalidad)
            ),
            format!(
                "SET VARIABLE anios_alerta_incremento = {}::DOUBLE;",
                p.contrato_incremento.anios_alerta
            ),
            format!(
                "SET VARIABLE practicante_contiene = {};",
                norm(&p.practicante.modalidad_contiene)
            ),
            format!(
                "SET VARIABLE cp_condicion = {};",
                norm(&p.control_practicantes.condicion)
            ),
            format!(
                "SET VARIABLE cp_pc1_desde = {}; SET VARIABLE cp_pc1_hasta = {};",
                p.control_practicantes.por_cumplir_1[0], p.control_practicantes.por_cumplir_1[1]
            ),
            format!(
                "SET VARIABLE cp_c1_desde = {};",
                p.control_practicantes.cumplio_1
            ),
            format!(
                "SET VARIABLE es_cargo_practicante = {};",
                norm(&p.examenes_salida.cargo_practicante)
            ),
            format!(
                "SET VARIABLE cp_pc2_desde = {}; SET VARIABLE cp_pc2_hasta = {};",
                p.control_practicantes.por_cumplir_2[0], p.control_practicantes.por_cumplir_2[1]
            ),
        ]
        .join("\n");
        self.conexion
            .execute_batch(&sql)
            .map_err(error("fijar los parámetros"))
    }

    /// Vista `tabla` sobre un Parquet (Plan 5: el tablero lee `actual/` sin copiarlo).
    pub fn vista_parquet(&self, tabla: &str, ruta: &Path) -> Result<(), ErrorProceso> {
        self.ejecutar(&format!(
            "CREATE OR REPLACE VIEW {} AS SELECT * FROM read_parquet({});",
            id(tabla),
            ruta_sql(ruta)
        ))
    }

    /// Copia un Parquet a una tabla en memoria. El tablero la usa para leer `actual/` una sola vez y soltar
    /// los archivos enseguida (revisión final del Plan 5: `procesar` puede reemplazar `actual/` en Windows).
    pub fn tabla_parquet(&self, tabla: &str, ruta: &Path) -> Result<(), ErrorProceso> {
        self.ejecutar(&format!(
            "CREATE OR REPLACE TABLE {} AS SELECT * FROM read_parquet({});",
            id(tabla),
            ruta_sql(ruta)
        ))
    }

    /// Tabla `filtros` con una fila: una columna `VARCHAR` por nombre. Los valores van como parámetros de
    /// una sentencia preparada (nunca dentro del texto SQL); los nombres son constantes del programa.
    pub fn fijar_filtros(&self, filtros: &[(&str, Option<&str>)]) -> Result<(), ErrorProceso> {
        if let Some((raro, _)) = filtros
            .iter()
            .find(|(n, _)| n.is_empty() || !n.bytes().all(|b| b.is_ascii_lowercase() || b == b'_'))
        {
            return Err(ErrorProceso::Datos(format!(
                "Nombre de filtro no válido: «{raro}»."
            )));
        }
        let columnas = filtros
            .iter()
            .map(|(n, _)| format!("?::VARCHAR AS {}", id(n)))
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!("CREATE OR REPLACE TABLE filtros AS SELECT {columnas}");
        let valores: Vec<Value> = filtros
            .iter()
            .map(|(_, v)| v.map_or(Value::Null, |t| Value::Text(t.to_string())))
            .collect();
        self.conexion
            .execute(&sql, duckdb::params_from_iter(valores))
            .map_err(error("fijar los filtros"))?;
        Ok(())
    }

    /// Ejecuta una consulta y devuelve cada fila como objeto JSON (columna → valor).
    /// Fechas como `AAAA-MM-DD`; un `DOUBLE` no finito pasa a `null` (JSON no admite `NaN`).
    pub fn consultar_json(
        &self,
        sql: &str,
    ) -> Result<Vec<serde_json::Map<String, serde_json::Value>>, ErrorProceso> {
        let mut sentencia = self.conexion.prepare(sql).map_err(error(sql))?;
        let mut filas = sentencia.query([]).map_err(error(sql))?;
        let nombres: Vec<String> = filas.as_ref().map(|s| s.column_names()).unwrap_or_default();
        let mut salida = Vec::new();
        while let Some(fila) = filas.next().map_err(error(sql))? {
            let mut objeto = serde_json::Map::new();
            for (i, nombre) in nombres.iter().enumerate() {
                let valor: Value = fila.get(i).map_err(error(sql))?;
                objeto.insert(nombre.clone(), a_json(valor));
            }
            salida.push(objeto);
        }
        Ok(salida)
    }

    /// Ejecuta un archivo `.sql`; un error nombra el archivo.
    pub fn ejecutar_archivo(&self, ruta: &Path) -> Result<(), ErrorProceso> {
        let donde = ruta.display().to_string();
        let sql = std::fs::read_to_string(ruta).map_err(|e| ErrorProceso::Duckdb {
            donde: donde.clone(),
            detalle: e.to_string(),
        })?;
        self.conexion.execute_batch(&sql).map_err(error(donde))
    }

    pub fn ejecutar(&self, sql: &str) -> Result<(), ErrorProceso> {
        self.conexion.execute_batch(sql).map_err(error(sql))
    }

    pub fn columnas(&self, tabla: &str) -> Result<Vec<String>, ErrorProceso> {
        let sql = format!(
            "SELECT column_name FROM information_schema.columns WHERE table_name = {} \
             ORDER BY ordinal_position",
            texto_sql(tabla)
        );
        Ok(self
            .consultar_texto(&sql)?
            .into_iter()
            .filter_map(|mut f| f.remove(0))
            .collect())
    }

    pub fn existe(&self, tabla: &str) -> Result<bool, ErrorProceso> {
        Ok(!self.columnas(tabla)?.is_empty())
    }

    pub fn contar(&self, tabla: &str) -> Result<u64, ErrorProceso> {
        let n: i64 = self
            .conexion
            .query_row(&format!("SELECT count(*) FROM {}", id(tabla)), [], |r| {
                r.get(0)
            })
            .map_err(error(format!("contar {tabla}")))?;
        Ok(n as u64)
    }

    /// Escribe la tabla en Parquet (creando la carpeta) y devuelve sus filas.
    pub fn exportar_parquet(&self, tabla: &str, ruta: &Path) -> Result<u64, ErrorProceso> {
        if let Some(carpeta) = ruta.parent() {
            std::fs::create_dir_all(carpeta).map_err(|fuente| ErrorProceso::Salida {
                ruta: carpeta.to_path_buf(),
                fuente,
            })?;
        }
        self.ejecutar(&format!(
            "COPY {} TO {} (FORMAT parquet);",
            id(tabla),
            ruta_sql(ruta)
        ))?;
        self.contar(tabla)
    }

    /// Escribe la tabla como Excel de revisión (Plan 6): una hoja con el nombre de la tabla sin `_gold`,
    /// cabeceras en la fila 1 y valores tipados. Devuelve las filas de datos.
    pub fn exportar_excel(&self, tabla: &str, ruta: &Path) -> Result<u64, ErrorProceso> {
        use rust_xlsxwriter::{ExcelDateTime, Format, Workbook};
        const MAX_FILAS_EXCEL: u64 = 1_048_575;
        let filas = self.contar(tabla)?;
        if filas > MAX_FILAS_EXCEL {
            return Err(ErrorProceso::Datos(format!(
                "La tabla {tabla} tiene {filas} filas: demasiadas filas para Excel."
            )));
        }
        if let Some(carpeta) = ruta.parent() {
            std::fs::create_dir_all(carpeta).map_err(|fuente| ErrorProceso::Salida {
                ruta: carpeta.to_path_buf(),
                fuente,
            })?;
        }
        let xlsx = |e: rust_xlsxwriter::XlsxError| ErrorProceso::Salida {
            ruta: ruta.to_path_buf(),
            fuente: std::io::Error::other(e.to_string()),
        };
        let mut libro = Workbook::new();
        let hoja = libro.add_worksheet();
        hoja.set_name(tabla.strip_suffix("_gold").unwrap_or(tabla))
            .map_err(xlsx)?;
        let formato_fecha = Format::new().set_num_format("dd/mm/yyyy");
        let sql = format!("SELECT * FROM {}", id(tabla));
        let mut sentencia = self.conexion.prepare(&sql).map_err(error(&sql))?;
        let mut cursor = sentencia.query([]).map_err(error(&sql))?;
        let nombres: Vec<String> = cursor
            .as_ref()
            .map(|s| s.column_names())
            .unwrap_or_default();
        for (c, nombre) in nombres.iter().enumerate() {
            hoja.write_string(0, c as u16, nombre).map_err(xlsx)?;
        }
        let mut fila: u32 = 0;
        while let Some(registro) = cursor.next().map_err(error(&sql))? {
            fila += 1;
            for c in 0..nombres.len() {
                let col = c as u16;
                let valor: Value = registro.get(c).map_err(error(&sql))?;
                if let Value::Date32(dias) = valor {
                    let f = NaiveDate::from_ymd_opt(1970, 1, 1).unwrap()
                        + chrono::TimeDelta::days(i64::from(dias));
                    // Excel solo admite 1900–9999: una fecha mal tipeada (0202) va como texto.
                    match u16::try_from(f.year()).ok().and_then(|a| {
                        ExcelDateTime::from_ymd(a, f.month() as u8, f.day() as u8).ok()
                    }) {
                        Some(d) => hoja.write_datetime_with_format(fila, col, &d, &formato_fecha),
                        None => hoja.write_string(fila, col, f.format("%Y-%m-%d").to_string()),
                    }
                    .map_err(xlsx)?;
                    continue;
                }
                match a_json(valor) {
                    serde_json::Value::Null => {}
                    serde_json::Value::Bool(b) => {
                        hoja.write_boolean(fila, col, b).map_err(xlsx)?;
                    }
                    serde_json::Value::Number(n) => {
                        hoja.write_number(fila, col, n.as_f64().unwrap_or(0.0))
                            .map_err(xlsx)?;
                    }
                    serde_json::Value::String(t) => {
                        hoja.write_string(fila, col, &t).map_err(xlsx)?;
                    }
                    otro => {
                        hoja.write_string(fila, col, otro.to_string())
                            .map_err(xlsx)?;
                    }
                }
            }
        }
        libro.save(ruta).map_err(xlsx)?;
        Ok(u64::from(fila))
    }

    /// Ejecuta una consulta y devuelve cada valor como texto (`NULL` → `None`).
    pub fn consultar_texto(&self, sql: &str) -> Result<Vec<Vec<Option<String>>>, ErrorProceso> {
        let envuelta = format!("SELECT CAST(COLUMNS(*) AS VARCHAR) FROM ({sql})");
        let mut sentencia = self.conexion.prepare(&envuelta).map_err(error(sql))?;
        let mut filas = sentencia.query([]).map_err(error(sql))?;
        let n = filas.as_ref().map_or(0, |s| s.column_count());
        let mut salida = Vec::new();
        while let Some(fila) = filas.next().map_err(error(sql))? {
            let valores = (0..n)
                .map(|i| fila.get::<_, Option<String>>(i))
                .collect::<Result<Vec<_>, _>>()
                .map_err(error(sql))?;
            salida.push(valores);
        }
        Ok(salida)
    }
}
