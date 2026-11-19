//! Contrato de `CommunityHistoryRepository`: el comportamiento del que dependen
//! los suscriptores. Lo pasan el fake y todos los adaptadores reales.
//!
//! El puerto no declara errores (ERR-3): cualquier falla es `Fault`.

use crate::ports::outbound::CommunityHistoryRepository;
use communities_domain::CommunityEvent;
use communities_domain::value_objects::{CommunityId, CommunityName, OwnerId};

fn id(value: &str) -> CommunityId {
    CommunityId::new(value.into())
}

fn created(community: &str) -> CommunityEvent {
    CommunityEvent::Created {
        community_id: id(community),
        owner: OwnerId::new("alice".into()),
    }
}

fn renamed(community: &str) -> CommunityEvent {
    CommunityEvent::Renamed {
        community_id: id(community),
        name: CommunityName::new("Rust Santiago".into())
            .expect("invariante: nombre de prueba válido"),
    }
}

async fn append(adapter: &impl CommunityHistoryRepository, event: &CommunityEvent) {
    adapter
        .append(event)
        .await
        .expect("contrato: append no falla con el adaptador sano");
}

async fn history(
    adapter: &impl CommunityHistoryRepository,
    community: &str,
) -> Vec<CommunityEvent> {
    adapter
        .history_of(&id(community))
        .await
        .expect("contrato: history_of no falla con el adaptador sano")
}

/// El historial conserva el orden en que se agregaron los eventos.
pub async fn keeps_appended_events_in_order<R: CommunityHistoryRepository>(
    new_adapter: impl Fn() -> R,
) {
    let adapter = new_adapter();

    append(&adapter, &created("c-1")).await;
    append(&adapter, &renamed("c-1")).await;

    assert_eq!(
        history(&adapter, "c-1").await,
        vec![created("c-1"), renamed("c-1")]
    );
}

/// Una comunidad sin eventos tiene el historial vacío, no un error.
pub async fn unknown_community_has_an_empty_history<R: CommunityHistoryRepository>(
    new_adapter: impl Fn() -> R,
) {
    let adapter = new_adapter();

    assert!(history(&adapter, "c-1").await.is_empty());
}

/// Cada comunidad ve solo sus eventos.
pub async fn history_is_per_community<R: CommunityHistoryRepository>(new_adapter: impl Fn() -> R) {
    let adapter = new_adapter();

    append(&adapter, &created("c-1")).await;
    append(&adapter, &created("c-2")).await;

    assert_eq!(history(&adapter, "c-2").await, vec![created("c-2")]);
}

/// Genera un `#[test]` por caso del contrato de `CommunityHistoryRepository`.
#[macro_export]
macro_rules! community_history_repository_contract {
    (new_adapter: $new_adapter:expr, run: $run:expr $(,)?) => {
        mod community_history_repository_contract {
            #[allow(unused_imports)]
            use super::*;
            use $crate::testing::contracts::community_history_repository as contract;

            #[test]
            fn keeps_appended_events_in_order() {
                ($run)(contract::keeps_appended_events_in_order(|| $new_adapter));
            }

            #[test]
            fn unknown_community_has_an_empty_history() {
                ($run)(contract::unknown_community_has_an_empty_history(|| {
                    $new_adapter
                }));
            }

            #[test]
            fn history_is_per_community() {
                ($run)(contract::history_is_per_community(|| $new_adapter));
            }
        }
    };
}
