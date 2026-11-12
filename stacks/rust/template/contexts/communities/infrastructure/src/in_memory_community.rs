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

    communities_application::community_repository_contract! {
        new_adapter: InMemoryCommunityAdapter::default(),
        run: block_on,
    }
}
