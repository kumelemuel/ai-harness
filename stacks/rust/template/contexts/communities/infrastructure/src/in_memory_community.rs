use communities_application::ports::outbound::CommunityRepository;
use communities_domain::Community;
use communities_domain::value_objects::{CommunityId, CommunitySlug};
use kernel::{BoxFuture, Fault};
use std::collections::HashMap;
use std::future::ready;
use std::sync::{Mutex, MutexGuard};

/// Adaptador en memoria de `CommunityRepository`.
#[derive(Default)]
pub struct InMemoryCommunityAdapter {
    communities: Mutex<HashMap<CommunityId, Community>>,
}

impl InMemoryCommunityAdapter {
    /// Un lock envenenado es una falla técnica no declarada: árbol B (ERR-3).
    fn store(&self) -> Result<MutexGuard<'_, HashMap<CommunityId, Community>>, Fault> {
        self.communities
            .lock()
            .map_err(|_| Fault::new("community store lock poisoned"))
    }
}

impl CommunityRepository for InMemoryCommunityAdapter {
    fn find_by_slug<'a>(
        &'a self,
        slug: &'a CommunitySlug,
    ) -> BoxFuture<'a, Result<Option<Community>, Fault>> {
        let result = self
            .store()
            .map(|store| store.values().find(|c| c.slug() == slug).cloned());
        Box::pin(ready(result))
    }

    fn save<'a>(&'a self, community: &'a Community) -> BoxFuture<'a, Result<(), Fault>> {
        let result = self.store().map(|mut store| {
            store.insert(community.id().clone(), community.clone());
        });
        Box::pin(ready(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use communities_application::testing::block_on;
    use communities_domain::value_objects::{CommunityName, OwnerId};

    fn community(id: &str, slug: &str) -> Community {
        Community::create(
            CommunityId::new(id.into()),
            OwnerId::new("alice".into()),
            CommunitySlug::new(slug.into()).unwrap(),
            CommunityName::new("Rust Chile".into()).unwrap(),
            true,
        )
    }

    #[test]
    fn finds_a_saved_community_by_slug() {
        let adapter = InMemoryCommunityAdapter::default();
        let saved = community("c-1", "rust-chile");
        block_on(adapter.save(&saved)).unwrap();

        let found = block_on(adapter.find_by_slug(saved.slug())).unwrap();

        assert_eq!(found, Some(saved));
    }

    #[test]
    fn unknown_slug_is_none_not_an_error() {
        let adapter = InMemoryCommunityAdapter::default();
        let slug = CommunitySlug::new("nope".into()).unwrap();

        assert_eq!(block_on(adapter.find_by_slug(&slug)).unwrap(), None);
    }

    #[test]
    fn save_overwrites_by_id() {
        let adapter = InMemoryCommunityAdapter::default();
        block_on(adapter.save(&community("c-1", "first"))).unwrap();
        block_on(adapter.save(&community("c-1", "second"))).unwrap();

        let first = CommunitySlug::new("first".into()).unwrap();

        assert_eq!(block_on(adapter.find_by_slug(&first)).unwrap(), None);
    }
}
