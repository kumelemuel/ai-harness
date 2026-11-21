use kernel::{Fault, FaultReporter};

/// Registro del árbol B en stderr (HEX-5: `{Tecnología}{Concepto}Adapter`). En
/// producción, otro adaptador de la misma capacidad: tracing, Sentry, etc.
pub struct StderrFaultReporterAdapter;

impl StderrFaultReporterAdapter {
    /// Línea de registro: lleva el detalle técnico, que nunca sale al cliente (ERR-4).
    /// Recorre la cadena de `source()`: el `Debug` del error de primer nivel no
    /// siempre muestra su causa.
    fn line(fault: &Fault) -> String {
        let mut line = format!("unexpected failure: {}", fault.detail());
        let mut cause = fault.detail().source();
        while let Some(error) = cause {
            line.push_str(&format!(": caused by: {error}"));
            cause = error.source();
        }
        line
    }
}

impl FaultReporter for StderrFaultReporterAdapter {
    fn report(&self, fault: &Fault) {
        eprintln!("{}", Self::line(fault));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_log_line_carries_the_technical_detail() {
        let fault = Fault::new("connection refused: db.internal:5432");

        let line = StderrFaultReporterAdapter::line(&fault);

        assert!(line.contains("db.internal:5432"), "{line}");
    }

    /// Un error de alto nivel cuyo `Debug` no muestra su causa: la cadena de
    /// `source()` tiene que llegar igual al registro.
    #[derive(Debug)]
    struct SaveFailed {
        cause: std::io::Error,
    }

    impl std::fmt::Display for SaveFailed {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("save failed")
        }
    }

    impl std::error::Error for SaveFailed {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            Some(&self.cause)
        }
    }

    /// `Debug` propio que esconde la causa, como el de muchas librerías.
    struct Opaque(SaveFailed);

    impl std::fmt::Debug for Opaque {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("Opaque")
        }
    }

    impl std::fmt::Display for Opaque {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("repository error")
        }
    }

    impl std::error::Error for Opaque {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            Some(&self.0)
        }
    }

    #[test]
    fn the_log_line_carries_the_whole_source_chain() {
        let fault = Fault::new(Opaque(SaveFailed {
            cause: std::io::Error::other("connection refused: db.internal:5432"),
        }));

        let line = StderrFaultReporterAdapter::line(&fault);

        assert!(line.contains("repository error"), "{line}");
        assert!(line.contains("save failed"), "{line}");
        assert!(line.contains("db.internal:5432"), "{line}");
    }
}
