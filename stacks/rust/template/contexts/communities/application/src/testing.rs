//! Dobles de los puertos salientes y baterías de contrato (TDD-2): código de
//! primera clase.
//!
//! Los dobles se nombran como adaptadores (HEX-5) con la "tecnología" de
//! prefijo: `Fake` si tienen estado, `Fixed` o `Stub` si responden fijo. Cada
//! doble pasa la misma batería de contrato (`contracts`) que los adaptadores
//! reales. Disponibles para los tests de este crate y, con la feature
//! `testing`, para los de otras capas.

use crate::ports::outbound::CommunityRepository;
use communities_domain::Community;
use communities_domain::value_objects::CommunitySlug;
use kernel::{BoxFuture, Fault, IdGenerator};
use std::future::{Future, ready};
use std::pin::pin;
use std::sync::Mutex;
use std::task::{Context, Poll, Waker};

pub mod contracts;

/// Ejecuta un future sin runtime externo.
///
/// Pensado para fakes que resuelven de inmediato; no sirve para IO real.
pub fn block_on<F: Future>(future: F) -> F::Output {
    let mut future = pin!(future);
    let mut context = Context::from_waker(Waker::noop());
    loop {
        if let Poll::Ready(output) = future.as_mut().poll(&mut context) {
            return output;
        }
        std::thread::yield_now();
    }
}

pub struct FakeCommunityAdapter {
    communities: Mutex<Vec<Community>>,
    failing: bool,
}

impl FakeCommunityAdapter {
    pub fn empty() -> Self {
        Self {
            communities: Mutex::new(Vec::new()),
            failing: false,
        }
    }

    /// Simula una falla técnica no declarada (árbol B).
    pub fn failing() -> Self {
        Self {
            communities: Mutex::new(Vec::new()),
            failing: true,
        }
    }

    pub fn saved(&self) -> Vec<Community> {
        self.communities
            .lock()
            .expect("invariante: fake sin envenenar")
            .clone()
    }

    fn outage<T>(&self) -> Result<T, Fault> {
        Err(Fault::new("fake repository outage"))
    }
}

impl CommunityRepository for FakeCommunityAdapter {
    fn find_by_slug<'a>(
        &'a self,
        slug: &'a CommunitySlug,
    ) -> BoxFuture<'a, Result<Option<Community>, Fault>> {
        let result = if self.failing {
            self.outage()
        } else {
            Ok(self.saved().into_iter().find(|c| c.slug() == slug))
        };
        Box::pin(ready(result))
    }

    fn save<'a>(&'a self, community: &'a Community) -> BoxFuture<'a, Result<(), Fault>> {
        let result = if self.failing {
            self.outage()
        } else {
            let mut communities = self
                .communities
                .lock()
                .expect("invariante: fake sin envenenar");
            // Mismo id: reemplaza, como cualquier repositorio (contrato).
            match communities.iter_mut().find(|c| c.id() == community.id()) {
                Some(existing) => *existing = community.clone(),
                None => communities.push(community.clone()),
            }
            Ok(())
        };
        Box::pin(ready(result))
    }
}

pub struct FixedIdGeneratorAdapter(pub &'static str);

impl IdGenerator for FixedIdGeneratorAdapter {
    fn next_id(&self) -> String {
        self.0.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::community_repository_contract! {
        new_adapter: FakeCommunityAdapter::empty(),
        run: block_on,
    }
}
