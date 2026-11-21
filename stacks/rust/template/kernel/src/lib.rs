//! Kernel compartido (HEX-8): solo primitivas, solo `std`.
//!
//! Traits: marcadores de modelo y las capacidades técnicas transversales de la
//! lista cerrada (`Clock`, `IdGenerator`, `FaultReporter`). Nunca agregados de
//! un contexto, puertos de un caso de uso ni datos sensibles.

mod clock;
mod failure;
mod fault;
mod future;
mod id;
mod model;

pub use clock::Clock;
pub use failure::Failure;
pub use fault::{Fault, FaultReporter};
pub use future::BoxFuture;
pub use id::IdGenerator;
pub use model::{AggregateRoot, DomainEvent, ValueObject};
