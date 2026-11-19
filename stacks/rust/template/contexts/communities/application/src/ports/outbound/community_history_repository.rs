use communities_domain::CommunityEvent;
use communities_domain::value_objects::CommunityId;
use kernel::{BoxFuture, Fault};

/// Puerto saliente del historial de cambios de cada comunidad (HEX-5):
/// persistencia de solo inserción, en orden.
pub trait CommunityHistoryRepository: Send + Sync {
    fn append<'a>(&'a self, event: &'a CommunityEvent) -> BoxFuture<'a, Result<(), Fault>>;

    /// Los eventos de la comunidad, en el orden en que se agregaron.
    fn history_of<'a>(
        &'a self,
        community_id: &'a CommunityId,
    ) -> BoxFuture<'a, Result<Vec<CommunityEvent>, Fault>>;
}
