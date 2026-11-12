//! Kernel compartido (HEX-8): solo primitivas, solo `std`.
//!
//! Nunca agregados de un contexto, puertos de un caso de uso ni datos sensibles.

mod failure;
mod fault;
mod future;
mod id;
mod model;

pub use failure::Failure;
pub use fault::{Fault, FaultReporter};
pub use future::BoxFuture;
pub use id::IdGenerator;
pub use model::{AggregateRoot, ValueObject};
