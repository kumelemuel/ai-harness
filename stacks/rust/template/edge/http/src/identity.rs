/// Identidad verificada que el middleware de autenticación del composition root
/// inserta como `Extension`. Los adaptadores driving la leen para armar comandos.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticatedAccount {
    pub account_id: String,
}
