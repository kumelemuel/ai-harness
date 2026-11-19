use crate::errors::Unauthenticated;

/// Contrato de error acotado (ERR-2): la única hoja que este caso de uso produce.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerifyAccessTokenError {
    Unauthenticated(Unauthenticated),
}

impl From<Unauthenticated> for VerifyAccessTokenError {
    fn from(leaf: Unauthenticated) -> Self {
        Self::Unauthenticated(leaf)
    }
}
