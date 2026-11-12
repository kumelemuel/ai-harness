//! Códigos estables de error para clientes (miembro de extensión `code` del
//! Problem Details). Son vocabulario de transporte: viven solo en `api`.

pub const INVALID_COMMUNITY_SLUG: &str = "COMMUNITIES_INVALID_COMMUNITY_SLUG";
pub const INVALID_COMMUNITY_NAME: &str = "COMMUNITIES_INVALID_COMMUNITY_NAME";
pub const INVALID_REQUEST: &str = "COMMUNITIES_INVALID_REQUEST";
pub const SLUG_ALREADY_EXISTS: &str = "COMMUNITIES_SLUG_ALREADY_EXISTS";
