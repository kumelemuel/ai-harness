use crate::errors::{Conflict, Validation};

/// Contrato de error acotado (ERR-2): solo las hojas que este caso de uso
/// puede producir.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CreateCommunityError {
    Validation(Validation),
    Conflict(Conflict),
}

impl From<Validation> for CreateCommunityError {
    fn from(leaf: Validation) -> Self {
        Self::Validation(leaf)
    }
}

impl From<Conflict> for CreateCommunityError {
    fn from(leaf: Conflict) -> Self {
        Self::Conflict(leaf)
    }
}
