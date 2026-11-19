use iam_domain::AccountId;

/// Contrato de salida: sin tipos de dominio, porque `api` no ve `domain` (HEX-4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifyAccessTokenResponse {
    pub account_id: String,
}

impl From<&AccountId> for VerifyAccessTokenResponse {
    fn from(account: &AccountId) -> Self {
        Self {
            account_id: account.as_str().to_string(),
        }
    }
}
