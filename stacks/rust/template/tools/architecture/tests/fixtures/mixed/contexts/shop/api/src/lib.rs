//! hex_1_driving_adapters_see_other_legacy_contexts: un `api` ya separado que
//! nombra el dominio y un puerto saliente de contextos legacy, con el nombre
//! que el código les da (renombre y `[lib] name`).
use identity::application::ports::outbound::PasswordHashingService as _;

pub fn owner() -> Option<identity::domain::Account> {
    None
}

pub fn secret() -> Option<vault_core::domain::Secret> {
    None
}

/// `iam` es `iam-client`, no el contexto: no cuenta.
pub fn client() -> Option<iam::domain::Session> {
    None
}
