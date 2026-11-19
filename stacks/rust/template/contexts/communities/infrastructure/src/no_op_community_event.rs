use communities_application::ports::outbound::CommunityEventPublisher;
use communities_domain::CommunityEvent;
use kernel::{BoxFuture, Fault};
use std::future::ready;

/// Adaptador de `CommunityEventPublisher` sin suscriptores: descarta los eventos.
///
/// Es el que cablea el composition root mientras ningún contexto reaccione a
/// los eventos de `communities`. Un despachador en proceso, con suscriptores,
/// lo reemplazará (`InProcessCommunityEventAdapter`).
pub struct NoOpCommunityEventAdapter;

impl CommunityEventPublisher for NoOpCommunityEventAdapter {
    fn publish<'a>(&'a self, _events: &'a [CommunityEvent]) -> BoxFuture<'a, Result<(), Fault>> {
        Box::pin(ready(Ok(())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use communities_application::testing::block_on;

    communities_application::community_event_publisher_contract! {
        new_adapter: NoOpCommunityEventAdapter,
        run: block_on,
    }
}
