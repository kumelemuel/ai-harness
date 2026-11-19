//! Dobles de los puertos salientes y baterías de contrato (TDD-2).
//!
//! Los dobles se nombran como adaptadores (HEX-5) con un prefijo de la lista
//! cerrada de TDD-2. Cada doble pasa la misma batería de contrato
//! (`contracts`) que los adaptadores reales.

use crate::ports::outbound::AccessTokenService;
use crate::ports::outbound::dtos::AccessTokenVerdict;
use kernel::{BoxFuture, Fault};
use std::collections::HashMap;
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

/// Doble de `AccessTokenService`: solo es válido un token que él emitió.
#[derive(Default)]
pub struct FakeAccessTokenAdapter {
    issued: Mutex<HashMap<String, String>>,
    failing: bool,
}

impl FakeAccessTokenAdapter {
    /// Simula una falla técnica no declarada (árbol B).
    pub fn failing() -> Self {
        Self {
            failing: true,
            ..Self::default()
        }
    }

    /// Emite un token para `subject`, como lo haría el inicio de sesión.
    pub fn issue(&self, subject: &str) -> String {
        let mut issued = self.issued.lock().expect("invariante: fake sin envenenar");
        let token = format!("fake-token-{}", issued.len());
        issued.insert(token.clone(), subject.to_string());
        token
    }
}

impl AccessTokenService for FakeAccessTokenAdapter {
    fn verify<'a>(&'a self, token: &'a str) -> BoxFuture<'a, Result<AccessTokenVerdict, Fault>> {
        let result = if self.failing {
            Err(Fault::new("fake token verifier outage"))
        } else {
            let issued = self.issued.lock().expect("invariante: fake sin envenenar");
            Ok(match issued.get(token) {
                Some(subject) => AccessTokenVerdict::Valid {
                    subject: subject.clone(),
                },
                None => AccessTokenVerdict::Invalid,
            })
        };
        Box::pin(ready(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::access_token_service_contract! {
        new_adapter: FakeAccessTokenAdapter::default(),
        issue: |adapter: &FakeAccessTokenAdapter, subject: &str| adapter.issue(subject),
        run: block_on,
    }
}
