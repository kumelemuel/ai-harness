//! Errores de dominio de `iam`. No implementan `std::error::Error` (ERR-1,
//! RS-2): el handler los traduce a una hoja del contrato (ERR-2).

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidAccountId;

impl fmt::Display for InvalidAccountId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid account id")
    }
}
