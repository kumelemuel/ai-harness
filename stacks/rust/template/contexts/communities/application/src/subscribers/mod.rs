//! Reacciones a los eventos del contexto (HEX-4): `{Verb}{Noun}Subscriber`, cada
//! una implementa `CommunityEventSubscriber`. Orquestan como un handler.

mod record_community_history;

pub use record_community_history::RecordCommunityHistorySubscriber;
