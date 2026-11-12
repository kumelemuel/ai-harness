#[derive(Debug, Clone)]
pub struct CreateCommunityCommand {
    pub owner_id: String,
    pub slug: String,
    pub name: String,
    pub is_public: bool,
}
