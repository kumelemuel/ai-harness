//! Contrato de `AccessTokenService`: el comportamiento del que depende
//! `VerifyAccessTokenHandler`. Lo pasan el fake y todos los adaptadores reales.
//!
//! El puerto no declara errores, así que la fábrica no recibe un escenario. El
//! resultado depende del token, no de la configuración del adaptador: además
//! de la fábrica, la batería recibe `issue`, que emite un token válido para un
//! sujeto, como lo haría el inicio de sesión con esa misma clave.

use crate::ports::outbound::AccessTokenService;
use crate::ports::outbound::dtos::AccessTokenVerdict;

/// Un token emitido por el mismo emisor es válido y trae su sujeto.
pub async fn an_issued_token_is_valid_for_its_subject<R: AccessTokenService>(
    new_adapter: impl Fn() -> R,
    issue: impl Fn(&R, &str) -> String,
) {
    let adapter = new_adapter();
    let token = issue(&adapter, "alice");

    let result = adapter.verify(&token).await;

    assert!(
        matches!(&result, Ok(AccessTokenVerdict::Valid { subject }) if subject == "alice"),
        "{result:?}"
    );
}

/// Un token mal formado es un veredicto, no un `Fault` (ERR-3).
pub async fn a_malformed_token_is_invalid_not_a_fault<R: AccessTokenService>(
    new_adapter: impl Fn() -> R,
    _issue: impl Fn(&R, &str) -> String,
) {
    let adapter = new_adapter();

    let result = adapter.verify("not-a-token").await;

    assert!(
        matches!(result, Ok(AccessTokenVerdict::Invalid)),
        "mal formado es Ok(Invalid), no un error: {result:?}"
    );
}

/// Un token emitido y luego alterado ya no es válido.
pub async fn a_tampered_token_is_invalid<R: AccessTokenService>(
    new_adapter: impl Fn() -> R,
    issue: impl Fn(&R, &str) -> String,
) {
    let adapter = new_adapter();
    let mut token = issue(&adapter, "alice");
    token.push_str("tampered");

    let result = adapter.verify(&token).await;

    assert!(
        matches!(result, Ok(AccessTokenVerdict::Invalid)),
        "alterado es Ok(Invalid): {result:?}"
    );
}

/// Genera un `#[test]` por caso del contrato de `AccessTokenService`.
///
/// - `new_adapter`: construye un adaptador. Se evalúa en un módulo hijo del que
///   invoca la macro (`use super::*`).
/// - `issue`: `impl Fn(&R, &str) -> String`, emite un token válido para el
///   sujeto con la misma clave del adaptador.
/// - `run`: cómo ejecutar cada future (`block_on`, o un runtime real).
#[macro_export]
macro_rules! access_token_service_contract {
    (new_adapter: $new_adapter:expr, issue: $issue:expr, run: $run:expr $(,)?) => {
        mod access_token_service_contract {
            #[allow(unused_imports)]
            use super::*;
            use $crate::testing::contracts::access_token_service as contract;

            #[test]
            fn an_issued_token_is_valid_for_its_subject() {
                ($run)(contract::an_issued_token_is_valid_for_its_subject(
                    || $new_adapter,
                    $issue,
                ));
            }

            #[test]
            fn a_malformed_token_is_invalid_not_a_fault() {
                ($run)(contract::a_malformed_token_is_invalid_not_a_fault(
                    || $new_adapter,
                    $issue,
                ));
            }

            #[test]
            fn a_tampered_token_is_invalid() {
                ($run)(contract::a_tampered_token_is_invalid(
                    || $new_adapter,
                    $issue,
                ));
            }
        }
    };
}
