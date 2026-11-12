//! Errores de dominio. Son de este contexto y nunca cruzan hacia `api`:
//! el handler los traduce a una hoja del contrato (ERR-2).

use std::error::Error;
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

        impl Error for $name {}
    };
}

domain_error!(InvalidCommunitySlug, "invalid community slug");
domain_error!(InvalidCommunityName, "invalid community name");
domain_error!(NotCommunityOwner, "only the owner can perform this action");
