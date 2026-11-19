use communities_domain::CommunityEvent;
use kernel::{BoxFuture, Fault};

/// Puerto entrante (HEX-4) de las reacciones a los eventos de `communities`.
///
/// No lo invoca un cliente, sino el despachador de eventos
/// (`InProcessCommunityEventAdapter`, infraestructura). Por eso recibe el evento
/// de dominio: quien lo invoca ve `domain` (HEX-1). Un solo método, `on`.
///
/// Corre dentro de la publicación del handler, así que su falla es la del
/// handler: `Fault` (árbol B). No tiene árbol A.
pub trait CommunityEventSubscriber: Send + Sync {
    fn on<'a>(&'a self, event: &'a CommunityEvent) -> BoxFuture<'a, Result<(), Fault>>;
}
