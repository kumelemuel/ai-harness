use crate::errors::NotCommunityOwner;
use crate::events::CommunityEvent;
use crate::value_objects::{CommunityId, CommunityName, CommunitySlug, OwnerId};
use kernel::AggregateRoot;

/// Agregado inmutable: el comportamiento devuelve un nuevo estado o un error
/// de dominio (HEX-7). Las reglas viven aquí, no en el handler.
///
/// Cada transición registra su evento en el estado nuevo. El handler los toma
/// con `take_events` y guarda el agregado sin eventos pendientes: no se
/// persisten.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Community {
    id: CommunityId,
    owner: OwnerId,
    slug: CommunitySlug,
    name: CommunityName,
    is_public: bool,
    events: Vec<CommunityEvent>,
}

impl Community {
    pub fn create(
        id: CommunityId,
        owner: OwnerId,
        slug: CommunitySlug,
        name: CommunityName,
        is_public: bool,
    ) -> Self {
        let created = CommunityEvent::Created {
            community_id: id.clone(),
            owner: owner.clone(),
        };
        Self {
            id,
            owner,
            slug,
            name,
            is_public,
            events: vec![created],
        }
    }

    /// Regla de negocio: solo el dueño puede renombrar.
    pub fn rename(
        &self,
        requested_by: &OwnerId,
        name: CommunityName,
    ) -> Result<Self, NotCommunityOwner> {
        if &self.owner != requested_by {
            return Err(NotCommunityOwner);
        }

        let mut events = self.events.clone();
        events.push(CommunityEvent::Renamed {
            community_id: self.id.clone(),
            name: name.clone(),
        });
        Ok(Self {
            name,
            events,
            ..self.clone()
        })
    }

    /// Eventos registrados desde la última vez que se tomaron.
    pub fn events(&self) -> &[CommunityEvent] {
        &self.events
    }

    /// Separa los eventos pendientes: el agregado que devuelve no tiene ninguno.
    pub fn take_events(mut self) -> (Self, Vec<CommunityEvent>) {
        let events = std::mem::take(&mut self.events);
        (self, events)
    }

    pub fn id(&self) -> &CommunityId {
        &self.id
    }

    pub fn owner(&self) -> &OwnerId {
        &self.owner
    }

    pub fn slug(&self) -> &CommunitySlug {
        &self.slug
    }

    pub fn name(&self) -> &CommunityName {
        &self.name
    }

    pub fn is_public(&self) -> bool {
        self.is_public
    }
}

impl AggregateRoot for Community {}

#[cfg(test)]
mod tests {
    use super::*;

    fn community(owner: &str) -> Community {
        Community::create(
            CommunityId::new("c-1".into()),
            OwnerId::new(owner.into()),
            CommunitySlug::new("rust-chile".into()).unwrap(),
            CommunityName::new("Rust Chile".into()).unwrap(),
            true,
        )
    }

    #[test]
    fn owner_can_rename() {
        let original = community("alice");
        let new_name = CommunityName::new("Rust Santiago".into()).unwrap();

        let renamed = original
            .rename(&OwnerId::new("alice".into()), new_name)
            .unwrap();

        assert_eq!(renamed.name().as_str(), "Rust Santiago");
        assert_eq!(
            original.name().as_str(),
            "Rust Chile",
            "el original no muta"
        );
    }

    #[test]
    fn creating_records_created() {
        let created = community("alice");

        assert_eq!(
            created.events(),
            [CommunityEvent::Created {
                community_id: CommunityId::new("c-1".into()),
                owner: OwnerId::new("alice".into()),
            }]
        );
    }

    #[test]
    fn renaming_appends_renamed_without_touching_the_original() {
        let original = community("alice");
        let new_name = CommunityName::new("Rust Santiago".into()).unwrap();

        let renamed = original
            .rename(&OwnerId::new("alice".into()), new_name.clone())
            .unwrap();

        assert_eq!(renamed.events().len(), 2);
        assert_eq!(
            renamed.events()[1],
            CommunityEvent::Renamed {
                community_id: CommunityId::new("c-1".into()),
                name: new_name,
            }
        );
        assert_eq!(original.events().len(), 1, "el original no muta");
    }

    #[test]
    fn taking_events_leaves_none_pending() {
        let (community, events) = community("alice").take_events();

        assert_eq!(events.len(), 1);
        assert!(community.events().is_empty());
    }

    #[test]
    fn non_owner_cannot_rename() {
        let new_name = CommunityName::new("Hijacked".into()).unwrap();

        let result = community("alice").rename(&OwnerId::new("mallory".into()), new_name);

        assert_eq!(result, Err(NotCommunityOwner));
    }
}
