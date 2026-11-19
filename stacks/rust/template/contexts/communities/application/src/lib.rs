//! Aplicación del contexto `communities`: la frontera hexagonal (HEX-3).

pub mod commands;
pub mod errors;
pub mod ports;
pub mod subscribers;

#[cfg(any(test, feature = "testing"))]
pub mod testing;
