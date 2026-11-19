use crate::errors::InvalidAccountId;
use kernel::ValueObject;

/// Identidad de una cuenta. Otros contextos no la importan: modelan su propia
/// vista (`OwnerId` en `communities`) a partir de `edge_http::AuthenticatedAccount`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AccountId(String);

impl AccountId {
    pub fn new(value: String) -> Result<Self, InvalidAccountId> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(InvalidAccountId);
        }
        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl ValueObject for AccountId {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_a_non_blank_value_trimmed() {
        let id = AccountId::new("  alice ".into()).expect("valor válido");

        assert_eq!(id.as_str(), "alice");
    }

    #[test]
    fn rejects_a_blank_value() {
        assert_eq!(AccountId::new("   ".into()), Err(InvalidAccountId));
    }
}
