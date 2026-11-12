use kernel::ValueObject;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CommunityId(String);

impl CommunityId {
    pub fn new(value: String) -> Self {
        Self(value)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl ValueObject for CommunityId {}
