use kernel::{Fault, FaultReporter};

/// Registro del árbol B en stderr (HEX-5: `{Tecnología}{Concepto}Adapter`). En
/// producción, otro adaptador de la misma capacidad: tracing, Sentry, etc.
pub struct StderrFaultReporterAdapter;

impl StderrFaultReporterAdapter {
    /// Línea de registro: lleva el detalle técnico, que nunca sale al cliente (ERR-4).
    fn line(fault: &Fault) -> String {
        format!("unexpected failure: {:?}", fault.detail())
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
}
