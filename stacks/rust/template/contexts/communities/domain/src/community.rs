use crate::errors::NotCommunityOwner;
use crate::value_objects::{CommunityId, CommunityName, CommunitySlug, OwnerId};
use kernel::AggregateRoot;

/// Agregado inmutable: el comportamiento devuelve un nuevo estado o un error
/// de dominio (HEX-7). Las reglas viven aquí, no en el handler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Community {
    id: CommunityId,
    owner: OwnerId,
    slug: CommunitySlug,
    name: CommunityName,
    is_public: bool,
}

impl Community {
    pub fn create(
        id: CommunityId,
        owner: OwnerId,
        slug: CommunitySlug,
        name: CommunityName,
        is_public: bool,
    ) -> Self {
        Self {
            id,
            owner,
            slug,
            name,
            is_public,
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

        Ok(Self {
            name,
            ..self.clone()
        })
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

impl AggregateRoot for Community {
    type Id = CommunityId;

    fn id(&self) -> &CommunityId {
        &self.id
    }
}

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
    fn non_owner_cannot_rename() {
        let new_name = CommunityName::new("Hijacked".into()).unwrap();

        let result = community("alice").rename(&OwnerId::new("mallory".into()), new_name);

        assert_eq!(result, Err(NotCommunityOwner));
    }
}
