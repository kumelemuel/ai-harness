use crate::errors::InvalidCommunitySlug;
use kernel::ValueObject;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CommunitySlug(String);

impl CommunitySlug {
    pub const MIN: usize = 3;
    pub const MAX: usize = 32;

    pub fn new(value: String) -> Result<Self, InvalidCommunitySlug> {
        let normalized = value.trim().to_ascii_lowercase();

        let valid_length = (Self::MIN..=Self::MAX).contains(&normalized.len());
        let valid_chars = normalized
            .chars()
            .all(|c| c.is_ascii_lowercase() || c == '-');
        let valid_hyphens = !normalized.starts_with('-')
            && !normalized.ends_with('-')
            && !normalized.contains("--");

        if !(valid_length && valid_chars && valid_hyphens) {
            return Err(InvalidCommunitySlug);
        }

        Ok(Self(normalized))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl ValueObject for CommunitySlug {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_to_lowercase() {
        let slug = CommunitySlug::new(" Rust-Chile ".into()).unwrap();

        assert_eq!(slug.as_str(), "rust-chile");
    }

    #[test]
    fn rejects_invalid_characters() {
        for value in ["rust 123", "rust123", "rust@!", "   "] {
            assert_eq!(
                CommunitySlug::new(value.into()),
                Err(InvalidCommunitySlug),
                "{value}"
            );
        }
    }

    #[test]
    fn rejects_misplaced_hyphens() {
        for value in ["-rust", "rust-", "rust--chile"] {
            assert_eq!(
                CommunitySlug::new(value.into()),
                Err(InvalidCommunitySlug),
                "{value}"
            );
        }
    }

    #[test]
    fn rejects_lengths_outside_bounds() {
        assert_eq!(CommunitySlug::new("ab".into()), Err(InvalidCommunitySlug));
        assert_eq!(
            CommunitySlug::new("a".repeat(33)),
            Err(InvalidCommunitySlug)
        );
    }
}
