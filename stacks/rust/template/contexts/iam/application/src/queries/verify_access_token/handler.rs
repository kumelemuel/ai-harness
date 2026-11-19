use super::{VerifyAccessTokenError, VerifyAccessTokenQuery, VerifyAccessTokenResponse};
use crate::errors::Unauthenticated;
use crate::ports::inbound::VerifyAccessTokenUseCase;
use crate::ports::outbound::AccessTokenService;
use crate::ports::outbound::dtos::AccessTokenVerdict;
use iam_domain::AccountId;
use kernel::{BoxFuture, Failure};
use std::sync::Arc;

/// Orquesta (HEX-7): pide el veredicto y arma la identidad con el value object.
pub struct VerifyAccessTokenHandler {
    tokens: Arc<dyn AccessTokenService>,
}

impl VerifyAccessTokenHandler {
    pub fn new(tokens: Arc<dyn AccessTokenService>) -> Self {
        Self { tokens }
    }
}

type Outcome = Result<VerifyAccessTokenResponse, Failure<VerifyAccessTokenError>>;

impl VerifyAccessTokenUseCase for VerifyAccessTokenHandler {
    fn execute(&self, query: VerifyAccessTokenQuery) -> BoxFuture<'_, Outcome> {
        Box::pin(async move {
            // `?` propaga el `Fault` como árbol B.
            let verdict = self.tokens.verify(&query.token).await?;

            // `Invalid` es éxito del puerto que este caso de uso trata como
            // error (ERR-3). Un sujeto vacío tampoco identifica a nadie: el
            // error de dominio va a la misma hoja, sin envolver (ERR-2).
            let AccessTokenVerdict::Valid { subject } = verdict else {
                return Err(Failure::expected(Unauthenticated));
            };
            let account =
                AccountId::new(subject).map_err(|_| Failure::expected(Unauthenticated))?;

            Ok(VerifyAccessTokenResponse::from(&account))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{FakeAccessTokenAdapter, block_on};

    fn verify(tokens: FakeAccessTokenAdapter, token: &str) -> Outcome {
        let handler = VerifyAccessTokenHandler::new(Arc::new(tokens));
        block_on(handler.execute(VerifyAccessTokenQuery {
            token: token.into(),
        }))
    }

    #[test]
    fn a_valid_token_identifies_its_account() {
        let tokens = FakeAccessTokenAdapter::default();
        let token = tokens.issue("alice");

        let result = verify(tokens, &token);

        assert_eq!(
            result.unwrap(),
            VerifyAccessTokenResponse {
                account_id: "alice".into()
            }
        );
    }

    #[test]
    fn an_invalid_token_is_unauthenticated() {
        let result = verify(FakeAccessTokenAdapter::default(), "not-a-token");

        assert!(matches!(
            result,
            Err(Failure::Expected(VerifyAccessTokenError::Unauthenticated(
                Unauthenticated
            )))
        ));
    }

    #[test]
    fn a_valid_token_without_subject_is_unauthenticated() {
        let tokens = FakeAccessTokenAdapter::default();
        let token = tokens.issue("   ");

        let result = verify(tokens, &token);

        assert!(matches!(
            result,
            Err(Failure::Expected(VerifyAccessTokenError::Unauthenticated(
                Unauthenticated
            )))
        ));
    }

    #[test]
    fn a_verifier_failure_is_unexpected_not_unauthenticated() {
        let result = verify(FakeAccessTokenAdapter::failing(), "any");

        assert!(matches!(result, Err(Failure::Unexpected(_))));
    }
}
