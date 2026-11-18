# ADR-0002: Python con uv en `demo/` para generar datos sintéticos y verificar paridad

- **Estado:** Aceptado
- **Fecha:** 2026-11-14
- **Relacionado:** [ADR-0001](0001-stack-rust-tauri.md)

## Contexto

El proyecto no puede usar datos reales: el legacy procesó información personal de un cliente
y su historial git contiene archivos reales ([01-analisis-legacy.md §8](../01-analisis-legacy.md#8-riesgos-para-publicar-como-open-source)).
Aun así necesitamos:

1. **Datos de demo** para que cualquiera pueda clonar el repo, procesar y ver el dashboard.
2. **Datos de prueba** que reproduzcan los casos sucios reales: cabeceras desplazadas o combinadas,
   filas de totales, `#N/D`, DNIs con `.0`, fechas como texto, claves duplicadas, columnas
   dinámicas de subsidio, etc.
3. **Resultados esperados (golden)** para verificar que el motor en Rust reproduce la lógica del legacy.

El legacy está escrito en Python, de modo que el mismo entorno puede generar los datos y también
ejecutar el legacy para producir los resultados esperados.

## Decisión

- Todo el tooling Python vive en **`demo/`**, como proyecto independiente con su propio
  `pyproject.toml` y `uv.lock`, gestionado con **uv** y un **venv local** (`demo/.venv`).
- Python es **solo una herramienta de desarrollo y demo**: el producto (Rust + Tauri) no depende de Python en tiempo de ejecución.
- El generador:
  - se basa en **contratos**: lee los mismos YAML de contrato y esquema que usa el motor;
  - es **determinista**: semilla fija (`--seed`);
  - tiene dos modos: `clean` (datos válidos) y `dirty` (inyecta los casos límite documentados en `docs/logica-negocio/`);
  - produce Excel con el layout real de cada dominio (fila y columna de cabecera, hojas, patrón de nombre de archivo).
- Un script de **paridad** ejecuta el legacy (de solo lectura) sobre los datos generados y guarda los
  Gold resultantes como *fixtures* esperados para los tests del motor Rust.

## Alternativas consideradas

| Alternativa | Motivo del descarte |
|---|---|
| Generador en Rust | Más lento de escribir; no permite ejecutar el legacy para la paridad |
| Datos estáticos versionados a mano | No escalan, no cubren variantes y se desactualizan cuando cambian los contratos |
| pip + requirements.txt | uv es más rápido, tiene lockfile reproducible y ya se usaba en el legacy |

## Consecuencias

- **Positivas:** demo reproducible, cobertura sistemática de casos sucios y paridad medible contra el legacy.
- **Negativas:** el repo convive con tres toolchains (Rust, Node y Python). Se mitiga con comandos
  documentados en `AGENTS.md` y con `demo/` totalmente aislado.
- **Regla:** nunca se suben datos reales a `demo/` ni a ninguna otra parte del repo; los Excel generados
  se regeneran desde la semilla y por defecto no se versionan, salvo *fixtures* pequeños de tests.

## Notas

- **2026-11-16 (Plan 3):** el generador **no** lee los contratos. En su lugar, los tests de `crates/core` validan
  su salida contra `config/fuentes/`: `limpio` y `sucio` sin errores, y cada `CASO-xx` con su error exacto. Si el
  generador y los contratos se desalinean, falla un test. Los nombres de opciones y modos quedaron en español
  (`--semilla`, `limpio`/`sucio`/`casos`), según la convención de AGENTS.md §5.
