//! Eventos de dominio de `communities` (HEX-7): los registra el agregado en sus
//! transiciones; el handler los toma y los publica por un puerto.

use crate::value_objects::{CommunityId, CommunityName, OwnerId};
use kernel::DomainEvent;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommunityEvent {
    Created {
        community_id: CommunityId,
        owner: OwnerId,
    },
    Renamed {
        community_id: CommunityId,
        name: CommunityName,
    },
}

impl DomainEvent for CommunityEvent {}
