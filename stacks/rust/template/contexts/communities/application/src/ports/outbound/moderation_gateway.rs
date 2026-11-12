use super::dtos::{ModerationError, ModerationVerdict, ReviewContentRequest};
use kernel::{BoxFuture, Failure};

/// Puerto saliente hacia un servicio de moderación de terceros (HEX-5).
///
/// Semántica (ERR-3): si el servicio respondió, aprobando o rechazando, es éxito
/// del puerto y el veredicto viaja en `ModerationVerdict`. Si no pudo completar,
/// es el error declarado `ModerationError`. Cualquier otra falla técnica es
/// `Fault` (árbol B).
pub trait ModerationGateway: Send + Sync {
    fn review_content(
        &self,
        request: ReviewContentRequest,
    ) -> BoxFuture<'_, Result<ModerationVerdict, Failure<ModerationError>>>;
}
