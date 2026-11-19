use communities_domain::CommunityEvent;
use kernel::{BoxFuture, Fault};

/// Puerto saliente que despacha los eventos de dominio dentro del proceso
/// (HEX-5, rol `Publisher`). El handler lo invoca con los eventos que tomó del
/// agregado (`take_events`), después de guardarlo. Con un `UnitOfWork`, ambas
/// cosas van dentro de la misma transacción.
///
/// No declara errores (ERR-3): cualquier falla es `Fault`.
pub trait CommunityEventPublisher: Send + Sync {
    fn publish<'a>(&'a self, events: &'a [CommunityEvent]) -> BoxFuture<'a, Result<(), Fault>>;
}
