use iam_application::ports::outbound::AccessTokenService;
use iam_application::ports::outbound::dtos::AccessTokenVerdict;
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode};
use kernel::{BoxFuture, Fault};
use serde::Deserialize;
use std::future::ready;

/// Adaptador de `AccessTokenService` con JWT firmados con HS256.
///
/// Verifica firma y vencimiento (`exp`, obligatorio). Cualquier token que no
/// pase es `Invalid` (ERR-3): el motivo no se distingue ni se registra, porque
/// el caso de uso no ramifica por él.
pub struct JwtAccessTokenAdapter {
    key: DecodingKey,
    validation: Validation,
}

impl JwtAccessTokenAdapter {
    pub fn new(secret: &[u8]) -> Self {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_required_spec_claims(&["exp", "sub"]);
        Self {
            key: DecodingKey::from_secret(secret),
            validation,
        }
    }
}

#[derive(Deserialize)]
struct Claims {
    sub: String,
}

impl AccessTokenService for JwtAccessTokenAdapter {
    fn verify<'a>(&'a self, token: &'a str) -> BoxFuture<'a, Result<AccessTokenVerdict, Fault>> {
        let verdict = match decode::<Claims>(token, &self.key, &self.validation) {
            Ok(data) => AccessTokenVerdict::Valid {
                subject: data.claims.sub,
            },
            Err(_) => AccessTokenVerdict::Invalid,
        };
        Box::pin(ready(Ok(verdict)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iam_application::testing::block_on;
    use jsonwebtoken::{EncodingKey, Header, encode, get_current_timestamp};
    use serde::Serialize;

    const SECRET: &[u8] = b"test-secret";

    #[derive(Serialize)]
    struct IssuedClaims<'a> {
        sub: &'a str,
        exp: u64,
    }

    /// Emite como lo haría el inicio de sesión: HS256 con la misma clave.
    fn issue_with(secret: &[u8], subject: &str, exp: u64) -> String {
        encode(
            &Header::new(Algorithm::HS256),
            &IssuedClaims { sub: subject, exp },
            &EncodingKey::from_secret(secret),
        )
        .expect("invariante: HS256 siempre firma")
    }

    fn issue(_adapter: &JwtAccessTokenAdapter, subject: &str) -> String {
        issue_with(SECRET, subject, get_current_timestamp() + 3600)
    }

    iam_application::access_token_service_contract! {
        new_adapter: JwtAccessTokenAdapter::new(SECRET),
        issue: issue,
        run: block_on,
    }

    #[test]
    fn an_expired_token_is_invalid() {
        let adapter = JwtAccessTokenAdapter::new(SECRET);
        // Más allá del margen (`leeway`) de 60 s de la validación.
        let token = issue_with(SECRET, "alice", get_current_timestamp() - 120);

        let result = block_on(adapter.verify(&token));

        assert!(
            matches!(result, Ok(AccessTokenVerdict::Invalid)),
            "{result:?}"
        );
    }

    #[test]
    fn a_token_signed_with_another_key_is_invalid() {
        let adapter = JwtAccessTokenAdapter::new(SECRET);
        let token = issue_with(b"another-secret", "alice", get_current_timestamp() + 3600);

        let result = block_on(adapter.verify(&token));

        assert!(
            matches!(result, Ok(AccessTokenVerdict::Invalid)),
            "{result:?}"
        );
    }
}
