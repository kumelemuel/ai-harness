//! Payloads de `ModerationGateway`.

/// Lo que se le pide revisar al servicio de moderación.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewContentRequest {
    pub content: String,
}

/// Respuesta del servicio. Un rechazo es éxito del puerto: el servicio respondió
/// (ERR-3). Qué hacer con él lo decide el caso de uso.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModerationVerdict {
    Approved,
    Rejected { reason: String },
}

/// Error declarado (ERR-3): el servicio no pudo completar la revisión.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModerationError {
    /// No respondió a tiempo o no está disponible.
    Unavailable,
}
