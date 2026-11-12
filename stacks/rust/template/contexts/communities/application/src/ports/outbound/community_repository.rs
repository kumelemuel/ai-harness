use communities_domain::Community;
use communities_domain::value_objects::CommunitySlug;
use kernel::{BoxFuture, Fault};

/// Puerto saliente de persistencia durable (HEX-5).
///
/// Sin errores declarados: cualquier falla es árbol B (`Fault`). "No existe"
/// no es un error: `find_*` devuelve `Option` (ERR-1).
pub trait CommunityRepository: Send + Sync {
    fn find_by_slug<'a>(
        &'a self,
        slug: &'a CommunitySlug,
    ) -> BoxFuture<'a, Result<Option<Community>, Fault>>;

    fn save<'a>(&'a self, community: &'a Community) -> BoxFuture<'a, Result<(), Fault>>;
}
