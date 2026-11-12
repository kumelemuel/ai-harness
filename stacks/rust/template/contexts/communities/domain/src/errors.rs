//! Errores de dominio. Son de este contexto y nunca cruzan hacia `api`:
//! el handler los traduce a una hoja del contrato (ERR-2).
//!
//! No implementan `std::error::Error` (ERR-1, RS-2): así el `impl From<E: Error>
//! for Fault` no los alcanza y `?` no los vuelve árbol B en silencio.

use std::fmt;

macro_rules! domain_error {
    ($name:ident, $message:literal) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub struct $name;

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str($message)
            }
        }
    };
}

domain_error!(InvalidCommunitySlug, "invalid community slug");
domain_error!(InvalidCommunityName, "invalid community name");
domain_error!(NotCommunityOwner, "only the owner can perform this action");
