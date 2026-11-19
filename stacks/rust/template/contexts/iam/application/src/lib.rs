//! Aplicación del contexto `iam`: la frontera hexagonal (HEX-3).

pub mod errors;
pub mod ports;
pub mod queries;

#[cfg(any(test, feature = "testing"))]
pub mod testing;
