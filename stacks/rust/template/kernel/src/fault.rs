use std::error::Error;
use std::fmt;

type Source = Box<dyn Error + Send + Sync + 'static>;

/// Árbol B (ERR-1): falla técnica que el caso de uso no puede interpretar.
///
/// Es opaca: `Display` nunca expone el detalle (ERR-4). El detalle solo se
/// entrega al [`FaultReporter`] en el borde.
///
/// `Fault` no implementa `Error` a propósito: así `?` convierte cualquier error
/// técnico (`sqlx::Error`, `io::Error`…) en `Fault` dentro de los adaptadores.
pub struct Fault {
    source: Source,
}

impl Fault {
    pub fn new(source: impl Into<Source>) -> Self {
        Self {
            source: source.into(),
        }
    }

    /// Detalle técnico, solo para el registro en el borde. Nunca para el cliente.
    pub fn detail(&self) -> &(dyn Error + Send + Sync + 'static) {
        &*self.source
    }
}

impl<E> From<E> for Fault
where
    E: Error + Send + Sync + 'static,
{
    fn from(error: E) -> Self {
        Self::new(error)
    }
}

impl fmt::Display for Fault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("unexpected failure")
    }
}

impl fmt::Debug for Fault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Fault")
            .field("source", &self.source)
            .finish()
    }
}

/// Registro del árbol B: capacidad técnica transversal (HEX-8). Lo invoca solo
/// el manejador global (ERR-4).
pub trait FaultReporter: Send + Sync {
    fn report(&self, fault: &Fault);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_never_exposes_the_technical_detail() {
        let fault = Fault::new("connection refused: db.internal:5432");

        assert_eq!(fault.to_string(), "unexpected failure");
        assert_eq!(
            fault.detail().to_string(),
            "connection refused: db.internal:5432"
        );
    }

    #[test]
    fn question_mark_converts_technical_errors_into_fault() {
        fn read() -> Result<(), Fault> {
            Err(std::io::Error::other("disk"))?
        }

        assert_eq!(read().unwrap_err().detail().to_string(), "disk");
    }
}
