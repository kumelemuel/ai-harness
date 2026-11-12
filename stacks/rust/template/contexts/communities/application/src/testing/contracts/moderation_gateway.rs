//! Contrato de `ModerationGateway`: el comportamiento del que depende
//! `CreateCommunityHandler`. Lo pasan el fake y todos los adaptadores reales.
//!
//! El puerto declara errores (ERR-3), así que la fábrica recibe el escenario que
//! debe provocar (`impl Fn(Scenario) -> R`) y hay un caso por resultado: aprueba,
//! rechaza y uno por cada variante de `ModerationError`.

use crate::ports::outbound::ModerationGateway;
use crate::ports::outbound::dtos::{ModerationError, ModerationVerdict, ReviewContentRequest};
use kernel::Failure;

/// Lo que la fábrica configura en el adaptador antes de cada caso.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scenario {
    /// El servicio responde y aprueba.
    Approves,
    /// El servicio responde y rechaza, con un motivo.
    Rejects,
    /// El servicio no puede completar: `ModerationError::Unavailable`.
    Unavailable,
}

/// Escenario que provoca cada variante del error declarado. El `match` es
/// exhaustivo: una variante nueva de `ModerationError` no compila hasta tener
/// su escenario, y su caso abajo.
pub const fn scenario_of(error: &ModerationError) -> Scenario {
    match error {
        ModerationError::Unavailable => Scenario::Unavailable,
    }
}

fn request() -> ReviewContentRequest {
    ReviewContentRequest {
        content: "Rust Chile".into(),
    }
}

/// Si el servicio aprueba, el puerto devuelve `Ok(Approved)`.
pub async fn approval_is_an_approved_verdict<R: ModerationGateway>(
    new_adapter: impl Fn(Scenario) -> R,
) {
    let adapter = new_adapter(Scenario::Approves);

    let result = adapter.review_content(request()).await;

    assert!(
        matches!(result, Ok(ModerationVerdict::Approved)),
        "{result:?}"
    );
}

/// Un rechazo es éxito del puerto: el servicio respondió (ERR-3). El veredicto
/// trae el motivo; el caso de uso decide qué hacer con él.
pub async fn rejection_is_a_verdict_not_an_error<R: ModerationGateway>(
    new_adapter: impl Fn(Scenario) -> R,
) {
    let adapter = new_adapter(Scenario::Rejects);

    let result = adapter.review_content(request()).await;

    assert!(
        matches!(&result, Ok(ModerationVerdict::Rejected { reason }) if !reason.trim().is_empty()),
        "un rechazo es Ok(Rejected) con motivo, no un error: {result:?}"
    );
}

/// No disponible es el error declarado, no un `Fault` ni un rechazo.
pub async fn unavailable_is_the_declared_error<R: ModerationGateway>(
    new_adapter: impl Fn(Scenario) -> R,
) {
    let adapter = new_adapter(scenario_of(&ModerationError::Unavailable));

    let result = adapter.review_content(request()).await;

    assert!(
        matches!(result, Err(Failure::Expected(ModerationError::Unavailable))),
        "no disponible es Err(Expected(Unavailable)): {result:?}"
    );
}

/// Genera un `#[test]` por caso del contrato de `ModerationGateway`.
///
/// - `new_adapter`: `impl Fn(Scenario) -> R`, construye un adaptador que
///   produce el escenario. Se evalúa en un módulo hijo del que invoca la macro
///   (`use super::*`).
/// - `run`: cómo ejecutar cada future (`block_on`, o un runtime real).
/// - Un campo por caso, **todos obligatorios**: `supported` o
///   `unsupported("motivo")`. Un adaptador que no puede producir un escenario
///   (un `NoOp` no rechaza ni falla) lo declara con su motivo; el caso queda
///   como `#[ignore = "motivo"]`, visible en la salida de `cargo test` y
///   ejecutable con `--include-ignored`. Omitir un campo no compila: no hay
///   forma de saltarse un caso en silencio, y un caso nuevo obliga a cada
///   adaptador a decidir.
///
/// Un caso nuevo se agrega aquí, como campo, y como `async fn` arriba.
#[macro_export]
macro_rules! moderation_gateway_contract {
    (
        new_adapter: $new_adapter:expr,
        run: $run:expr,
        approves: $approves:ident $(($approves_reason:literal))?,
        rejects: $rejects:ident $(($rejects_reason:literal))?,
        unavailable: $unavailable:ident $(($unavailable_reason:literal))? $(,)?
    ) => {
        mod moderation_gateway_contract {
            #[allow(unused_imports)]
            use super::*;
            use $crate::testing::contracts::moderation_gateway as contract;

            $crate::moderation_gateway_contract!(@case approval_is_an_approved_verdict,
                $new_adapter, $run, $approves $(($approves_reason))?);
            $crate::moderation_gateway_contract!(@case rejection_is_a_verdict_not_an_error,
                $new_adapter, $run, $rejects $(($rejects_reason))?);
            $crate::moderation_gateway_contract!(@case unavailable_is_the_declared_error,
                $new_adapter, $run, $unavailable $(($unavailable_reason))?);
        }
    };
    (@case $case:ident, $new_adapter:expr, $run:expr, supported) => {
        #[test]
        fn $case() {
            ($run)(contract::$case($new_adapter));
        }
    };
    (@case $case:ident, $new_adapter:expr, $run:expr, unsupported($reason:literal)) => {
        #[test]
        #[ignore = $reason]
        fn $case() {
            ($run)(contract::$case($new_adapter));
        }
    };
}
