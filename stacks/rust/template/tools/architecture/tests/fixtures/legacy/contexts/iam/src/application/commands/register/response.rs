use crate::domain::Account;

/// Expone el dominio por dos caminos: el nombre importado y la ruta completa.
pub struct RegisterAccountResponse {
    pub account: Account,
    pub again: crate::domain::Account,
}
