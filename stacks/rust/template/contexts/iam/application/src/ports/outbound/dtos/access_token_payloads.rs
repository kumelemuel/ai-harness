//! Payloads de `AccessTokenService`.

/// Respuesta del verificador. `Invalid` es éxito del puerto (ERR-3): qué hacer
/// con él lo decide el caso de uso.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccessTokenVerdict {
    /// Firma válida y vigente; `subject` es la cuenta a la que se emitió.
    Valid { subject: String },
    /// Mal formado, vencido o firmado con otra clave. El motivo no viaja: el
    /// caso de uso no ramifica por él.
    Invalid,
}
