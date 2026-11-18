//! App de escritorio (Plan 6): ajustes, servicio (lógica probada sin Tauri) y comandos.
//!
//! `tauri::generate_context!()` vive en `main.rs` para que `cargo test --lib` no necesite `app/ui/dist`.

pub mod ajustes;
pub mod comandos;
pub mod error;
pub mod servicio;

#[cfg(test)]
mod esquema;
