use crate::ports::inbound::CommunityEventSubscriber;
use crate::ports::outbound::CommunityHistoryRepository;
use communities_domain::CommunityEvent;
use kernel::{BoxFuture, Fault};
use std::sync::Arc;

/// Registra cada cambio de una comunidad en su historial.
pub struct RecordCommunityHistorySubscriber {
    history: Arc<dyn CommunityHistoryRepository>,
}

impl RecordCommunityHistorySubscriber {
    pub fn new(history: Arc<dyn CommunityHistoryRepository>) -> Self {
        Self { history }
    }
}

impl CommunityEventSubscriber for RecordCommunityHistorySubscriber {
    fn on<'a>(&'a self, event: &'a CommunityEvent) -> BoxFuture<'a, Result<(), Fault>> {
        self.history.append(event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{FakeCommunityHistoryAdapter, block_on};
    use communities_domain::value_objects::{CommunityId, OwnerId};

    fn created(id: &str) -> CommunityEvent {
        CommunityEvent::Created {
            community_id: CommunityId::new(id.into()),
            owner: OwnerId::new("alice".into()),
        }
    }

    #[test]
    fn records_the_event_in_the_community_history() {
        let history = Arc::new(FakeCommunityHistoryAdapter::empty());
        let subscriber = RecordCommunityHistorySubscriber::new(history.clone());

        block_on(subscriber.on(&created("c-1"))).unwrap();

        let recorded = block_on(history.history_of(&CommunityId::new("c-1".into()))).unwrap();
        assert_eq!(recorded, vec![created("c-1")]);
    }

    #[test]
    fn a_history_failure_is_unexpected() {
        let subscriber =
            RecordCommunityHistorySubscriber::new(Arc::new(FakeCommunityHistoryAdapter::failing()));

        let result = block_on(subscriber.on(&created("c-1")));

        assert!(result.is_err());
    }
}
