//! Contrato de `CommunityRepository`: el comportamiento del que dependen los
//! handlers. Lo pasan el fake y todos los adaptadores reales.
//!
//! El puerto no declara errores (ERR-3): cualquier falla es `Fault`, así que no
//! hay casos por variante ni parámetros de la fábrica para provocarlas.

use crate::ports::outbound::CommunityRepository;
use communities_domain::Community;
use communities_domain::value_objects::{CommunityId, CommunityName, CommunitySlug, OwnerId};

fn slug(value: &str) -> CommunitySlug {
    CommunitySlug::new(value.into()).expect("invariante: slug de prueba válido")
}

fn community(id: &str, slug_value: &str) -> Community {
    Community::create(
        CommunityId::new(id.into()),
        OwnerId::new("alice".into()),
        slug(slug_value),
        CommunityName::new("Rust Chile".into()).expect("invariante: nombre de prueba válido"),
        true,
    )
}

async fn save(adapter: &impl CommunityRepository, community: &Community) {
    adapter
        .save(community)
        .await
        .expect("contrato: save no falla con el adaptador sano");
}

async fn find(adapter: &impl CommunityRepository, value: &str) -> Option<Community> {
    adapter
        .find_by_slug(&slug(value))
        .await
        .expect("contrato: find_by_slug no falla con el adaptador sano")
}

/// Lo que se guarda se encuentra por su slug, igual a como se guardó.
pub async fn finds_a_saved_community_by_slug<R: CommunityRepository>(new_adapter: impl Fn() -> R) {
    let adapter = new_adapter();
    let saved = community("c-1", "rust-chile");
    save(&adapter, &saved).await;

    assert_eq!(find(&adapter, "rust-chile").await, Some(saved));
}

/// "No existe" no es un error: `find_by_slug` devuelve `None` (ERR-1).
pub async fn unknown_slug_is_none_not_an_error<R: CommunityRepository>(
    new_adapter: impl Fn() -> R,
) {
    let adapter = new_adapter();

    assert_eq!(find(&adapter, "rust-chile").await, None);
}

/// El slug se compara entero: ni prefijo ni extensión de uno guardado.
pub async fn find_by_slug_matches_the_whole_slug<R: CommunityRepository>(
    new_adapter: impl Fn() -> R,
) {
    let adapter = new_adapter();
    save(&adapter, &community("c-1", "rust-chile")).await;

    assert_eq!(find(&adapter, "rust").await, None);
    assert_eq!(find(&adapter, "rust-chile-sur").await, None);
}

/// Con varias comunidades, cada slug encuentra la suya.
pub async fn finds_each_community_by_its_own_slug<R: CommunityRepository>(
    new_adapter: impl Fn() -> R,
) {
    let adapter = new_adapter();
    let chile = community("c-1", "rust-chile");
    let peru = community("c-2", "rust-peru");
    save(&adapter, &chile).await;
    save(&adapter, &peru).await;

    assert_eq!(find(&adapter, "rust-chile").await, Some(chile));
    assert_eq!(find(&adapter, "rust-peru").await, Some(peru));
}

/// Guardar un agregado con un id existente lo reemplaza: no lo duplica ni deja
/// visible el estado anterior.
pub async fn save_replaces_the_community_with_the_same_id<R: CommunityRepository>(
    new_adapter: impl Fn() -> R,
) {
    let adapter = new_adapter();
    save(&adapter, &community("c-1", "first")).await;
    let second = community("c-1", "second");
    save(&adapter, &second).await;

    assert_eq!(find(&adapter, "first").await, None);
    assert_eq!(find(&adapter, "second").await, Some(second));
}

/// Genera un `#[test]` por caso del contrato de `CommunityRepository`.
///
/// - `new_adapter`: expresión que construye un adaptador nuevo y vacío. Se
///   evalúa en un módulo hijo del que invoca la macro (`use super::*`).
/// - `run`: cómo ejecutar cada future (`block_on`, o un runtime real).
///
/// Un caso nuevo se agrega aquí y como `async fn` arriba.
#[macro_export]
macro_rules! community_repository_contract {
    (new_adapter: $new_adapter:expr, run: $run:expr $(,)?) => {
        mod community_repository_contract {
            #[allow(unused_imports)]
            use super::*;
            use $crate::testing::contracts::community_repository as contract;

            #[test]
            fn finds_a_saved_community_by_slug() {
                ($run)(contract::finds_a_saved_community_by_slug(|| $new_adapter));
            }

            #[test]
            fn unknown_slug_is_none_not_an_error() {
                ($run)(contract::unknown_slug_is_none_not_an_error(|| $new_adapter));
            }

            #[test]
            fn find_by_slug_matches_the_whole_slug() {
                ($run)(contract::find_by_slug_matches_the_whole_slug(|| {
                    $new_adapter
                }));
            }

            #[test]
            fn finds_each_community_by_its_own_slug() {
                ($run)(contract::finds_each_community_by_its_own_slug(|| {
                    $new_adapter
                }));
            }

            #[test]
            fn save_replaces_the_community_with_the_same_id() {
                ($run)(contract::save_replaces_the_community_with_the_same_id(
                    || $new_adapter,
                ));
            }
        }
    };
}
