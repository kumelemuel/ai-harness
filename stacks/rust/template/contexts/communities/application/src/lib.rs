//! Aplicación del contexto `communities`: la frontera hexagonal (HEX-3).

pub mod commands;
pub mod errors;
pub mod ports;

#[cfg(any(test, feature = "testing"))]
pub mod testing;
