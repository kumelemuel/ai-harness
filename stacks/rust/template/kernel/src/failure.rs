use crate::Fault;

/// Canal de error de casos de uso y de puertos con errores declarados.
///
/// Mantiene los dos árboles separados (ERR-1): `E` y `Fault` no comparten tipo
/// base; `Failure` solo es el canal por el que viajan.
#[derive(Debug)]
pub enum Failure<E> {
    /// Árbol A: el caso de uso puede y debe ramificar.
    Expected(E),
    /// Árbol B: se propaga hasta el manejador global.
    Unexpected(Fault),
}

impl<E> Failure<E> {
    pub fn expected(error: impl Into<E>) -> Self {
        Self::Expected(error.into())
    }

    /// Traduce el error esperado (p. ej. de un puerto a una hoja del caso de uso)
    /// sin tocar el árbol B.
    pub fn map_expected<F>(self, translate: impl FnOnce(E) -> F) -> Failure<F> {
        match self {
            Self::Expected(error) => Failure::Expected(translate(error)),
            Self::Unexpected(fault) => Failure::Unexpected(fault),
        }
    }
}

impl<E> From<Fault> for Failure<E> {
    fn from(fault: Fault) -> Self {
        Self::Unexpected(fault)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_expected_translates_only_the_expected_tree() {
        let expected: Failure<u8> = Failure::Expected(1);
        let unexpected: Failure<u8> = Failure::Unexpected(Fault::new("x"));

        assert!(matches!(
            expected.map_expected(|n| n + 1),
            Failure::Expected(2)
        ));
        assert!(matches!(
            unexpected.map_expected(|n| n + 1),
            Failure::Unexpected(_)
        ));
    }

    #[test]
    fn question_mark_propagates_faults_into_failure() {
        fn step() -> Result<(), Fault> {
            Err(Fault::new("down"))
        }
        fn use_case() -> Result<(), Failure<&'static str>> {
            step()?;
            Ok(())
        }

        assert!(matches!(use_case(), Err(Failure::Unexpected(_))));
    }
}
