//! Dirección de dependencias entre crates (HEX-1, HEX-2).
//!
//! Cargo ya impide usar lo que no está declarado; estos tests impiden
//! *declarar* lo que la arquitectura no permite.

mod common;

use common::{Crate, Layer, assert_no_violations, crates};
use std::collections::HashMap;

/// Frameworks de servidor: solo en el borde driving y en el composition root.
const SERVER_FRAMEWORKS: &[&str] = &["axum", "actix-web", "rocket", "warp", "poem", "tower-http"];

fn may_depend_on(from: &Crate, to: &Crate) -> bool {
    use Layer::*;
    let same_context = from.context == to.context;
    match (from.layer, to.layer) {
        (Kernel, _) => false,
        (_, Kernel) => true,
        (Domain, _) => false,
        (Application, Domain) => same_context,
        (Infrastructure, Domain | Application) => same_context,
        // Un api de contexto solo ve su application; uno central (`apps/api`) ve todos.
        (Api, Application) => from.context.is_none() || same_context,
        (Api, Edge) => true,
        (Edge, _) => false,
        (Bootstrap, _) => true,
        _ => false,
    }
}

#[test]
fn hex_1_dependencies_point_inward() {
    let crates = crates();
    let by_name: HashMap<&str, &Crate> = crates.iter().map(|c| (c.name.as_str(), c)).collect();

    let violations = crates
        .iter()
        .flat_map(|from| {
            from.dependencies().into_iter().filter_map(|dep| {
                let to = by_name.get(dep.as_str())?;
                (!may_depend_on(from, to)).then(|| {
                    format!(
                        "{} ({:?}) → {} ({:?})",
                        from.name, from.layer, to.name, to.layer
                    )
                })
            })
        })
        .collect();

    assert_no_violations(
        "HEX-1: las dependencias apuntan hacia adentro; api solo ve el application de su contexto y edge; \
         api e infrastructure no se ven entre sí; sin dependencias entre contextos",
        violations,
    );
}

/// Cubre también TDD-2: `Crate::dependencies` incluye `[dev-dependencies]`, y
/// Cargo impide usar en un test lo que no está declarado. Así, los tests de
/// dominio y de caso de uso no pueden usar `tokio` ni un framework.
#[test]
fn hex_2_core_depends_only_on_std_and_the_kernel() {
    let crates = crates();
    let internal: Vec<&str> = crates.iter().map(|c| c.name.as_str()).collect();

    let violations = crates
        .iter()
        .filter(|c| matches!(c.layer, Layer::Kernel | Layer::Domain | Layer::Application))
        .flat_map(|c| {
            c.dependencies()
                .into_iter()
                .filter(|dep| !internal.contains(&dep.as_str()))
                .map(move |dep| format!("{} ({:?}) usa el crate externo `{dep}`", c.name, c.layer))
        })
        .collect();

    assert_no_violations(
        "HEX-2 / TDD-2: domain, application y kernel solo usan std y el kernel, \
         también en [dev-dependencies] (sus tests no levantan runtime ni framework)",
        violations,
    );
}

#[test]
fn hex_2_server_frameworks_only_at_the_edge() {
    let violations = crates()
        .iter()
        .filter(|c| !matches!(c.layer, Layer::Api | Layer::Edge | Layer::Bootstrap))
        .flat_map(|c| {
            c.dependencies()
                .into_iter()
                .filter(|dep| SERVER_FRAMEWORKS.contains(&dep.as_str()))
                .map(move |dep| format!("{} ({:?}) usa `{dep}`", c.name, c.layer))
        })
        .collect();

    assert_no_violations(
        "HEX-2: los frameworks de servidor solo viven en api, edge y bootstrap",
        violations,
    );
}
