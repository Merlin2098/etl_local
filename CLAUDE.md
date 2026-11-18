# CLAUDE.md

El contrato de agentes de este repositorio está en [AGENTS.md](AGENTS.md). Léelo antes de
cualquier tarea: sus reglas, límites de autonomía y convenciones aplican igual a Claude.

Recordatorios rápidos:

- `HR_Data_Integration_Pipeline/` es el **legacy de solo lectura** y contiene referencias al cliente.
  Nunca copies de ahí nombres de cliente ni datos reales (AGENTS.md §3.1).
- Antes de implementar una regla de negocio, abre su documento en `docs/logica-negocio/` y cita su ID.
- Las decisiones de arquitectura están en `docs/adr/`. Si una tarea las contradice, propón un ADR
  nuevo en lugar de desviarte.
- No hagas commits, push ni PRs sin pedido explícito.
