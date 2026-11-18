//! Error que reciben los comandos en la UI: código estable, mensaje en español claro y problemas por celda.

use etl_local_core::error::{ErrorContrato, ErrorProceso, ErrorTablero};
use etl_local_core::informe::Problema;
use serde::Serialize;

#[derive(schemars::JsonSchema, Serialize, Debug, Clone, PartialEq)]
pub struct ErrorApp {
    /// `entradas`, `cancelado`, `archivo_abierto`, `en_curso`, `no_permitido`, `sin_excel` o `interno`.
    pub codigo: String,
    pub mensaje: String,
    pub problemas: Vec<Problema>,
}

impl ErrorApp {
    pub fn nuevo(codigo: &str, mensaje: impl Into<String>) -> Self {
        Self {
            codigo: codigo.into(),
            mensaje: mensaje.into(),
            problemas: Vec::new(),
        }
    }
}

impl From<ErrorProceso> for ErrorApp {
    fn from(e: ErrorProceso) -> Self {
        let mensaje = e.to_string();
        match e {
            ErrorProceso::Entradas(informes) => ErrorApp {
                codigo: "entradas".into(),
                mensaje,
                problemas: informes.into_iter().flat_map(|i| i.problemas).collect(),
            },
            ErrorProceso::Cancelado => ErrorApp::nuevo("cancelado", mensaje),
            ErrorProceso::Publicar { .. } => ErrorApp::nuevo("archivo_abierto", mensaje),
            ErrorProceso::CarpetaAjena(ruta) => ErrorApp::nuevo(
                "carpeta_ajena",
                format!(
                    "La carpeta {} ya tiene otros archivos y no la creó esta aplicación. Para no \
                     mezclar nada no la usamos: cámbiale el nombre o muévela, y vuelve a intentarlo.",
                    ruta.display()
                ),
            ),
            _ => ErrorApp::nuevo("interno", mensaje),
        }
    }
}

impl From<ErrorContrato> for ErrorApp {
    fn from(e: ErrorContrato) -> Self {
        ErrorApp::nuevo("interno", e.to_string())
    }
}

impl From<ErrorTablero> for ErrorApp {
    fn from(e: ErrorTablero) -> Self {
        ErrorApp::nuevo("interno", e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use etl_local_core::informe::{Informe, Severidad};

    fn problema(codigo: &str) -> Problema {
        Problema {
            codigo: codigo.into(),
            severidad: Severidad::Error,
            archivo: "a.xlsx".into(),
            hoja: None,
            celda: None,
            celda_encontrada: None,
            mensaje: "m".into(),
            detalle: None,
        }
    }

    #[test]
    fn error_app_desde_entradas() {
        let e = ErrorProceso::Entradas(vec![
            Informe {
                problemas: vec![problema("PRE-09")],
            },
            Informe {
                problemas: vec![problema("PRE-10"), problema("PRE-11")],
            },
        ]);
        let app = ErrorApp::from(e);
        assert_eq!(app.codigo, "entradas");
        assert_eq!(app.problemas.len(), 3);
    }

    #[test]
    fn error_app_cancelado_y_publicar() {
        assert_eq!(ErrorApp::from(ErrorProceso::Cancelado).codigo, "cancelado");
        let publicar = ErrorProceso::Publicar {
            ruta: "actual".into(),
            marca: "m".into(),
            fuente: std::io::Error::other("bloqueado"),
        };
        let app = ErrorApp::from(publicar);
        assert_eq!(app.codigo, "archivo_abierto");
        assert!(app.mensaje.contains("abierto"), "{}", app.mensaje);
    }

    #[test]
    fn carpeta_ajena_en_palabras_simples() {
        // Revisión final del Plan 6: sin «etl-local» ni «.etl-local» en el mensaje.
        let app = ErrorApp::from(ErrorProceso::CarpetaAjena(
            "C:/Docs/Reportes RR. HH.".into(),
        ));
        assert_eq!(app.codigo, "carpeta_ajena");
        assert!(!app.mensaje.contains("etl-local"), "{}", app.mensaje);
        assert!(app.mensaje.contains("Reportes RR. HH."), "{}", app.mensaje);
    }
}
