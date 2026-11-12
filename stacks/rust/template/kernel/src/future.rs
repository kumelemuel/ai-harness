use std::future::Future;
use std::pin::Pin;

/// Future en caja, compatible con `dyn Trait` y solo con `std`.
///
/// Los puertos async lo devuelven para poder usarse como `Arc<dyn Puerto>`
/// sin depender de crates externos en el núcleo (HEX-2).
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
