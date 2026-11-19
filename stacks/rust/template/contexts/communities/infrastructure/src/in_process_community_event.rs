use communities_application::ports::inbound::CommunityEventSubscriber;
use communities_application::ports::outbound::CommunityEventPublisher;
use communities_domain::CommunityEvent;
use kernel::{BoxFuture, Fault};
use std::sync::Arc;

/// Adaptador de `CommunityEventPublisher` que despacha dentro del proceso: cada
/// evento, en orden, a cada suscriptor, en el orden en que se registraron.
///
/// Es síncrono respecto del handler: la publicación termina cuando terminan
/// todos los suscriptores, y la primera falla corta y se propaga como `Fault`.
/// Así, dentro de un `UnitOfWork`, revierte el guardado del handler.
pub struct InProcessCommunityEventAdapter {
    subscribers: Vec<Arc<dyn CommunityEventSubscriber>>,
}

impl InProcessCommunityEventAdapter {
    pub fn new(subscribers: Vec<Arc<dyn CommunityEventSubscriber>>) -> Self {
        Self { subscribers }
    }
}

impl CommunityEventPublisher for InProcessCommunityEventAdapter {
    fn publish<'a>(&'a self, events: &'a [CommunityEvent]) -> BoxFuture<'a, Result<(), Fault>> {
        Box::pin(async move {
            for event in events {
                for subscriber in &self.subscribers {
                    subscriber.on(event).await?;
                }
            }
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use communities_application::testing::block_on;
    use communities_domain::value_objects::{CommunityId, OwnerId};
    use std::future::ready;
    use std::sync::Mutex;

    communities_application::community_event_publisher_contract! {
        new_adapter: InProcessCommunityEventAdapter::new(Vec::new()),
        run: block_on,
    }

    /// Registra en un diario compartido qué recibió y quién lo recibió.
    struct SpyCommunityEventSubscriber {
        name: &'static str,
        journal: Arc<Mutex<Vec<String>>>,
        failing: bool,
    }

    impl CommunityEventSubscriber for SpyCommunityEventSubscriber {
        fn on<'a>(&'a self, event: &'a CommunityEvent) -> BoxFuture<'a, Result<(), Fault>> {
            self.journal
                .lock()
                .expect("invariante: spy sin envenenar")
                .push(format!("{}:{}", self.name, event.community_id().as_str()));
            let result = if self.failing {
                Err(Fault::new("subscriber outage"))
            } else {
                Ok(())
            };
            Box::pin(ready(result))
        }
    }

    fn spy(
        name: &'static str,
        journal: &Arc<Mutex<Vec<String>>>,
        failing: bool,
    ) -> Arc<dyn CommunityEventSubscriber> {
        Arc::new(SpyCommunityEventSubscriber {
            name,
            journal: journal.clone(),
            failing,
        })
    }

    fn created(id: &str) -> CommunityEvent {
        CommunityEvent::Created {
            community_id: CommunityId::new(id.into()),
            owner: OwnerId::new("alice".into()),
        }
    }

    #[test]
    fn delivers_each_event_to_each_subscriber_in_order() {
        let journal = Arc::new(Mutex::new(Vec::new()));
        let adapter = InProcessCommunityEventAdapter::new(vec![
            spy("a", &journal, false),
            spy("b", &journal, false),
        ]);

        block_on(adapter.publish(&[created("c-1"), created("c-2")])).expect("publica");

        assert_eq!(
            *journal.lock().expect("invariante: sin envenenar"),
            ["a:c-1", "b:c-1", "a:c-2", "b:c-2"]
        );
    }

    #[test]
    fn the_first_failing_subscriber_stops_and_fails_the_publication() {
        let journal = Arc::new(Mutex::new(Vec::new()));
        let adapter = InProcessCommunityEventAdapter::new(vec![
            spy("a", &journal, true),
            spy("b", &journal, false),
        ]);

        let result = block_on(adapter.publish(&[created("c-1")]));

        assert!(result.is_err());
        assert_eq!(
            *journal.lock().expect("invariante: sin envenenar"),
            ["a:c-1"]
        );
    }
}
