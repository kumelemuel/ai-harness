use communities_application::ports::outbound::CommunityHistoryRepository;
use communities_domain::CommunityEvent;
use communities_domain::value_objects::CommunityId;
use kernel::{BoxFuture, Fault};
use std::future::ready;
use std::sync::Mutex;

/// Adaptador de producción en memoria de `CommunityHistoryRepository`, para
/// correr sin base de datos. Pasa la misma batería que el fake.
#[derive(Default)]
pub struct InMemoryCommunityHistoryAdapter {
    events: Mutex<Vec<CommunityEvent>>,
}

impl CommunityHistoryRepository for InMemoryCommunityHistoryAdapter {
    fn append<'a>(&'a self, event: &'a CommunityEvent) -> BoxFuture<'a, Result<(), Fault>> {
        let result = match self.events.lock() {
            Ok(mut events) => {
                events.push(event.clone());
                Ok(())
            }
            Err(_) => Err(Fault::new("community history lock poisoned")),
        };
        Box::pin(ready(result))
    }

    fn history_of<'a>(
        &'a self,
        community_id: &'a CommunityId,
    ) -> BoxFuture<'a, Result<Vec<CommunityEvent>, Fault>> {
        let result = match self.events.lock() {
            Ok(events) => Ok(events
                .iter()
                .filter(|event| event.community_id() == community_id)
                .cloned()
                .collect()),
            Err(_) => Err(Fault::new("community history lock poisoned")),
        };
        Box::pin(ready(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use communities_application::testing::block_on;

    communities_application::community_history_repository_contract! {
        new_adapter: InMemoryCommunityHistoryAdapter::default(),
        run: block_on,
    }
}
