use crate::domain::Account;

/// Expone el dominio por tres caminos: el nombre importado, la ruta completa y
/// la ruta relativa.
pub struct RegisterAccountResponse {
    pub account: Account,
    pub again: crate::domain::Account,
    pub relative: super::super::super::super::domain::Account,
}
