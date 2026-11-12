use communities_domain::Community;

/// Contrato de salida: sin tipos de dominio, porque `api` no ve `domain` (HEX-4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateCommunityResponse {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub is_public: bool,
}

impl From<&Community> for CreateCommunityResponse {
    fn from(community: &Community) -> Self {
        Self {
            id: community.id().as_str().to_string(),
            slug: community.slug().as_str().to_string(),
            name: community.name().as_str().to_string(),
            is_public: community.is_public(),
        }
    }
}
