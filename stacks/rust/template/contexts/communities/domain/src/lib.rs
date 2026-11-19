//! Dominio del contexto `communities`: modelos y reglas. Sin puertos (HEX-3).

pub mod community;
pub mod errors;
pub mod events;
pub mod value_objects;

pub use community::Community;
pub use events::CommunityEvent;
