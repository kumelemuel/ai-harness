use crate::errors::{Conflict, ContentRejected, DependencyUnavailable, Validation};

/// Contrato de error acotado (ERR-2): solo las hojas que este caso de uso
/// puede producir.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CreateCommunityError {
    Validation(Validation),
    Conflict(Conflict),
    ContentRejected(ContentRejected),
    DependencyUnavailable(DependencyUnavailable),
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

impl From<ContentRejected> for CreateCommunityError {
    fn from(leaf: ContentRejected) -> Self {
        Self::ContentRejected(leaf)
    }
}

impl From<DependencyUnavailable> for CreateCommunityError {
    fn from(leaf: DependencyUnavailable) -> Self {
        Self::DependencyUnavailable(leaf)
    }
}
