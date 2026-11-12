//! Borde HTTP compartido por los adaptadores driving de todos los contextos.
//!
//! Capa Edge: ve solo el kernel. Contiene lo transversal al transporte (Problem
//! Details, identidad autenticada, manejador del árbol B), nunca tipos de un contexto.

mod identity;
mod problem;

pub use identity::AuthenticatedAccount;
pub use problem::{CONTENT_TYPE_PROBLEM, Problem, unexpected};
