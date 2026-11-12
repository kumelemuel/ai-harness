//! Adaptadores driven del contexto `communities` (HEX-5: `{Tecnología}{Concepto}Adapter`).

mod in_memory_community;
mod uuid_id_generator;

pub use in_memory_community::InMemoryCommunityAdapter;
pub use uuid_id_generator::UuidIdGeneratorAdapter;
