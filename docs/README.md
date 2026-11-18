# Documentación — rework open source de HR Data Integration Pipeline

| Documento | Contenido |
|---|---|
| [00-objetivo.md](00-objetivo.md) | Objetivo, alcance, principios, opciones de stack (Go+Wails / Rust+Tauri) y dashboard en localhost |
| [01-analisis-legacy.md](01-analisis-legacy.md) | Análisis del proyecto Python legacy: arquitectura, dominios, reglas de negocio, deuda técnica y riesgos para publicar |
| [diseno/](diseno/README.md) | Usuarios, principios de UI, pantallas y lenguaje visual (mocks v2) |
| [logica-negocio/](logica-negocio/README.md) | Especificación detallada de toda la lógica de negocio, por dominio (~365 reglas con referencia a código), más reglas transversales y decisiones pendientes |

Las decisiones de arquitectura están en [adr/](adr/README.md): Rust + Tauri (ADR-0001) y Python + uv para la demo (ADR-0002).
El contrato para agentes está en [../AGENTS.md](../AGENTS.md).
El código legacy vive en [`../HR_Data_Integration_Pipeline/`](../HR_Data_Integration_Pipeline/) y solo se usa como referencia.
