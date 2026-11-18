use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const CONFIG: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../config/fuentes");
const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../core/tests/fixtures/demo");

fn etl(args: &[&str], archivos: &[PathBuf], config: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_etl-local"))
        .args(args)
        .arg("--config")
        .arg(config)
        .args(archivos)
        .output()
        .unwrap()
}

fn texto(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn planillas_limpio() -> Vec<PathBuf> {
    let mut v: Vec<_> = std::fs::read_dir(Path::new(FIXTURES).join("limpio/planilla"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "xlsx"))
        .collect();
    v.sort();
    v
}

#[test]
fn validar_limpio_sale_0() {
    let salida = etl(
        &["validar", "--fuente", "planilla"],
        &planillas_limpio(),
        Path::new(CONFIG),
    );
    let stdout = texto(&salida.stdout);
    assert_eq!(
        salida.status.code(),
        Some(0),
        "{stdout}{}",
        texto(&salida.stderr)
    );
    let ultima = stdout.lines().last().unwrap();
    assert!(
        ultima.contains("3 archivos revisados: 0 con errores"),
        "{ultima}"
    );
}

#[test]
fn validar_caso_sale_1() {
    let archivo = Path::new(FIXTURES).join("casos/CASO-03/Planilla 2025-11 Empleados.xlsx");
    let salida = etl(
        &["validar", "--fuente", "planilla"],
        &[archivo],
        Path::new(CONFIG),
    );
    let stdout = texto(&salida.stdout);
    assert_eq!(salida.status.code(), Some(1), "{stdout}");
    assert!(
        stdout.contains("DNI/CEX") && stdout.contains("[PRE-11]"),
        "{stdout}"
    );
}

#[test]
fn fuente_desconocida_sale_2() {
    let salida = etl(
        &["validar", "--fuente", "nomina"],
        &planillas_limpio(),
        Path::new(CONFIG),
    );
    let stderr = texto(&salida.stderr);
    assert_eq!(salida.status.code(), Some(2), "{stderr}");
    assert!(
        stderr.contains("planilla") && stderr.contains("base_personal"),
        "{stderr}"
    );
}

#[test]
fn contrato_roto_sale_2() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("planilla.yaml"),
        "version: 1\nfuente: planilla\n",
    )
    .unwrap();
    let salida = etl(
        &["validar", "--fuente", "planilla"],
        &planillas_limpio(),
        dir.path(),
    );
    let stderr = texto(&salida.stderr);
    assert_eq!(salida.status.code(), Some(2), "{stderr}");
    assert!(stderr.contains("planilla.yaml"), "{stderr}");
}

#[test]
fn leer_json() {
    let carpeta = Path::new(FIXTURES).join("limpio");
    let base = std::fs::read_dir(carpeta.join("base_personal"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.extension().is_some_and(|x| x == "xlsx"))
        .unwrap();
    let salida = etl(
        &["leer", "--fuente", "base_personal", "--json"],
        &[base],
        Path::new(CONFIG),
    );
    assert_eq!(salida.status.code(), Some(0), "{}", texto(&salida.stderr));
    let json: serde_json::Value = serde_json::from_slice(&salida.stdout).unwrap();
    let manifiesto: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(carpeta.join("manifiesto.json")).unwrap())
            .unwrap();
    let b = &manifiesto["base_personal"];
    let total = b["empleados"].as_u64().unwrap() + b["practicantes"].as_u64().unwrap();
    assert_eq!(json[0]["hojas"][0]["filas"].as_u64(), Some(total));
    assert!(
        json[0]["fecha_nombre"].to_string().contains("2025-11-12"),
        "{json}"
    );
    assert!(json[0]["hojas"][0]["columnas"][0] == "numero_documento");
}

const CONFIG_RAIZ: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../config");

fn procesar(planillas: &[PathBuf], extra: &[&str], salida: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_etl-local"))
        .args(["procesar", "--config", CONFIG_RAIZ, "--salida"])
        .arg(salida)
        .arg("--planilla")
        .args(planillas)
        .args(extra)
        .output()
        .unwrap()
}

#[test]
fn procesar_limpio_sale_0() {
    let dir = tempfile::tempdir().unwrap();
    let limpio = Path::new(FIXTURES).join("limpio");
    let licencias = limpio.join("planilla/licencias/CONTROL DE LICENCIAS.xlsx");
    let base = limpio.join("base_personal/BD. 12.11.2025..xlsx");
    let salida = procesar(
        &planillas_limpio(),
        &[
            "--licencias",
            licencias.to_str().unwrap(),
            "--base-personal",
            base.to_str().unwrap(),
        ],
        dir.path(),
    );
    let stdout = texto(&salida.stdout);
    assert_eq!(
        salida.status.code(),
        Some(0),
        "{stdout}{}",
        texto(&salida.stderr)
    );
    assert!(stdout.contains("Resultados en"), "{stdout}");
    assert!(stdout.contains("base_personal"), "{stdout}");
    assert!(dir.path().join("actual/gold/planilla.parquet").is_file());
}

#[test]
fn procesar_con_caso_roto_sale_1() {
    let dir = tempfile::tempdir().unwrap();
    let caso = Path::new(FIXTURES).join("casos/CASO-03/Planilla 2025-11 Empleados.xlsx");
    let salida = procesar(&[caso], &[], dir.path());
    let stdout = texto(&salida.stdout);
    assert_eq!(
        salida.status.code(),
        Some(1),
        "{stdout}{}",
        texto(&salida.stderr)
    );
    assert!(stdout.contains("[PRE-11]"), "{stdout}");
    assert!(!dir.path().join("actual").exists());
}

/// Mata el proceso hijo aunque el test falle.
struct Hijo(std::process::Child);

impl Drop for Hijo {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn tablero_responde() {
    use std::io::{BufRead, Read, Write};
    let dir = tempfile::tempdir().unwrap();
    let limpio = Path::new(FIXTURES).join("limpio");
    let licencias = limpio.join("planilla/licencias/CONTROL DE LICENCIAS.xlsx");
    let base = limpio.join("base_personal/BD. 12.11.2025..xlsx");
    let hecho = procesar(
        &planillas_limpio(),
        &[
            "--licencias",
            licencias.to_str().unwrap(),
            "--base-personal",
            base.to_str().unwrap(),
        ],
        dir.path(),
    );
    assert_eq!(hecho.status.code(), Some(0), "{}", texto(&hecho.stderr));

    let mut hijo = Hijo(
        Command::new(env!("CARGO_BIN_EXE_etl-local"))
            .args([
                "tablero",
                "--config",
                CONFIG_RAIZ,
                "--puerto",
                "0",
                "--salida",
            ])
            .arg(dir.path())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap(),
    );
    let mut linea = String::new();
    std::io::BufReader::new(hijo.0.stdout.take().unwrap())
        .read_line(&mut linea)
        .unwrap();
    let puerto: u16 = linea
        .split("127.0.0.1:")
        .nth(1)
        .and_then(|resto| resto.split('/').next())
        .and_then(|p| p.parse().ok())
        .unwrap_or_else(|| panic!("sin puerto en «{linea}»"));
    assert!(linea.contains("Ctrl+C"), "{linea}");

    let mut s = std::net::TcpStream::connect(("127.0.0.1", puerto)).unwrap();
    write!(
        s,
        "GET /api/v1/estado HTTP/1.1\r\nHost: 127.0.0.1:{puerto}\r\nConnection: close\r\n\r\n"
    )
    .unwrap();
    let mut respuesta = String::new();
    s.read_to_string(&mut respuesta).unwrap();
    assert!(respuesta.starts_with("HTTP/1.1 200"), "{respuesta}");
    assert!(respuesta.contains("2025-11-12"), "{respuesta}");
}

#[test]
fn puerto_ocupado_sale_2() {
    // Revisión 5: un puerto ocupado da un mensaje que sugiere --puerto.
    let ocupado = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let puerto = ocupado.local_addr().unwrap().port().to_string();
    let dir = tempfile::tempdir().unwrap();
    let salida = Command::new(env!("CARGO_BIN_EXE_etl-local"))
        .args([
            "tablero",
            "--config",
            CONFIG_RAIZ,
            "--puerto",
            &puerto,
            "--salida",
        ])
        .arg(dir.path())
        .output()
        .unwrap();
    let stderr = texto(&salida.stderr);
    assert_eq!(salida.status.code(), Some(2), "{stderr}");
    assert!(
        stderr.contains("--puerto") && stderr.contains(&puerto),
        "{stderr}"
    );
}

#[test]
fn tablero_con_config_invalida_sale_2() {
    // Revisión final: sin config válida no se levanta un servidor que solo responde 500.
    let vacia = tempfile::tempdir().unwrap();
    let salida = Command::new(env!("CARGO_BIN_EXE_etl-local"))
        .args(["tablero", "--puerto", "0", "--config"])
        .arg(vacia.path())
        .arg("--salida")
        .arg(vacia.path())
        .output()
        .unwrap();
    let stderr = texto(&salida.stderr);
    assert_eq!(salida.status.code(), Some(2), "{stderr}");
    assert!(stderr.contains("indicadores.yaml"), "{stderr}");
}

// --- Plan 6: actualizar ---------------------------------------------------------------------------

fn actualizar(args: &[&str], salida: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_etl-local"))
        .args(["actualizar", "--config", CONFIG_RAIZ, "--salida"])
        .arg(salida)
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn actualizar_reprocesa_lo_guardado() {
    let dir = tempfile::tempdir().unwrap();
    let limpio = Path::new(FIXTURES).join("limpio");
    let licencias = limpio.join("planilla/licencias/CONTROL DE LICENCIAS.xlsx");
    let base = limpio.join("base_personal/BD. 12.11.2025..xlsx");
    let mut args = vec!["--planilla".to_string()];
    args.extend(
        planillas_limpio()
            .iter()
            .map(|p| p.to_string_lossy().into_owned()),
    );
    args.extend([
        "--licencias".into(),
        licencias.to_string_lossy().into_owned(),
        "--base-personal".into(),
        base.to_string_lossy().into_owned(),
    ]);
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let primera = actualizar(&refs, dir.path());
    assert_eq!(primera.status.code(), Some(0), "{}", texto(&primera.stderr));
    let segunda = actualizar(
        &["--base-personal", base.to_str().unwrap(), "--json"],
        dir.path(),
    );
    let stdout = texto(&segunda.stdout);
    assert_eq!(
        segunda.status.code(),
        Some(0),
        "{stdout}{}",
        texto(&segunda.stderr)
    );
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["periodos"].as_array().unwrap().len(), 3, "{stdout}");
    let sin = actualizar(&["--sin", "licencias", "--json"], dir.path());
    assert_eq!(sin.status.code(), Some(0), "{}", texto(&sin.stderr));
    assert!(
        dir.path()
            .join("entradas/licencias")
            .read_dir()
            .unwrap()
            .count()
            == 1
    );
}

#[test]
fn actualizar_sin_nada_sale_2() {
    let dir = tempfile::tempdir().unwrap();
    let salida = actualizar(&[], dir.path());
    assert_eq!(salida.status.code(), Some(2));
    assert!(
        texto(&salida.stderr).contains("No hay archivos para procesar."),
        "{}",
        texto(&salida.stderr)
    );
}

#[test]
fn tablero_ayuda_menciona_ui() {
    let salida = Command::new(env!("CARGO_BIN_EXE_etl-local"))
        .args(["tablero", "--help"])
        .output()
        .unwrap();
    assert!(
        texto(&salida.stdout).contains("--ui"),
        "{}",
        texto(&salida.stdout)
    );
}
