//! Contrato de `CommunityEventPublisher`: el comportamiento del que dependen
//! los handlers. Lo pasan el doble y todos los adaptadores reales.
//!
//! El puerto no declara errores (ERR-3), así que no hay escenarios: el handler
//! solo depende de que publicar, con o sin eventos, termine bien.

use crate::ports::outbound::CommunityEventPublisher;
use communities_domain::CommunityEvent;
use communities_domain::value_objects::{CommunityId, OwnerId};

fn created() -> CommunityEvent {
    CommunityEvent::Created {
        community_id: CommunityId::new("c-1".into()),
        owner: OwnerId::new("alice".into()),
    }
}

/// Un caso de uso que no produjo eventos puede publicar la lista vacía.
pub async fn publishing_no_events_succeeds<R: CommunityEventPublisher>(
    new_adapter: impl Fn() -> R,
) {
    let adapter = new_adapter();

    let result = adapter.publish(&[]).await;

    assert!(result.is_ok(), "{result:?}");
}

/// Publicar eventos termina bien con el adaptador sano.
pub async fn publishing_events_succeeds<R: CommunityEventPublisher>(new_adapter: impl Fn() -> R) {
    let adapter = new_adapter();

    let result = adapter.publish(&[created(), created()]).await;

    assert!(result.is_ok(), "{result:?}");
}

/// Genera un `#[test]` por caso del contrato de `CommunityEventPublisher`.
#[macro_export]
macro_rules! community_event_publisher_contract {
    (new_adapter: $new_adapter:expr, run: $run:expr $(,)?) => {
        mod community_event_publisher_contract {
            #[allow(unused_imports)]
            use super::*;
            use $crate::testing::contracts::community_event_publisher as contract;

            #[test]
            fn publishing_no_events_succeeds() {
                ($run)(contract::publishing_no_events_succeeds(|| $new_adapter));
            }

            #[test]
            fn publishing_events_succeeds() {
                ($run)(contract::publishing_events_succeeds(|| $new_adapter));
            }
        }
    };
}
