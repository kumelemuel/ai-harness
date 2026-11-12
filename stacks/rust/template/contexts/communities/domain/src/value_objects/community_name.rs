use crate::errors::InvalidCommunityName;
use kernel::ValueObject;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommunityName(String);

impl CommunityName {
    pub const MIN: usize = 3;
    pub const MAX: usize = 64;

    pub fn new(value: String) -> Result<Self, InvalidCommunityName> {
        let trimmed = value.trim();
        let length = trimmed.chars().count();

        if !(Self::MIN..=Self::MAX).contains(&length) {
            return Err(InvalidCommunityName);
        }

        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl ValueObject for CommunityName {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_surrounding_whitespace() {
        let name = CommunityName::new("  Rust Chile  ".into()).unwrap();

        assert_eq!(name.as_str(), "Rust Chile");
    }

    #[test]
    fn rejects_names_outside_bounds() {
        assert_eq!(CommunityName::new("ab".into()), Err(InvalidCommunityName));
        assert_eq!(
            CommunityName::new("a".repeat(65)),
            Err(InvalidCommunityName)
        );
        assert_eq!(CommunityName::new("   ".into()), Err(InvalidCommunityName));
    }
}
