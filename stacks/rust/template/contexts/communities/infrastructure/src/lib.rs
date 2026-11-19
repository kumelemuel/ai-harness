//! Adaptadores driven del contexto `communities` (HEX-5: `{Tecnología}{Concepto}Adapter`).

mod in_memory_community;
mod in_memory_community_history;
mod in_process_community_event;
mod no_op_moderation;
mod uuid_id_generator;

pub use in_memory_community::InMemoryCommunityAdapter;
pub use in_memory_community_history::InMemoryCommunityHistoryAdapter;
pub use in_process_community_event::InProcessCommunityEventAdapter;
pub use no_op_moderation::NoOpModerationAdapter;
pub use uuid_id_generator::UuidIdGeneratorAdapter;
