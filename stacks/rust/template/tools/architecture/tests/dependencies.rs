//! Dirección de dependencias entre crates (HEX-1, HEX-2).
//!
//! Cargo ya impide usar lo que no está declarado; estos tests impiden
//! *declarar* lo que la arquitectura no permite.

mod common;

use common::{
    Crate, Layer, assert_no_violations, crates, declared_dependencies, dependency_key,
    dependency_names, expand_members, non_inherited, relative, workspace_members,
};
use std::collections::HashMap;

/// Frameworks de servidor: solo en el borde driving y en el composition root.
const SERVER_FRAMEWORKS: &[&str] = &["axum", "actix-web", "rocket", "warp", "poem", "tower-http"];

/// Clave de la violación en la línea base: `ruta/Cargo.toml#sección.paquete`.
fn key_of(dir: &std::path::Path, section: &str, dep: &str) -> String {
    dependency_key(std::path::Path::new(&relative(dir)), section, dep)
}

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
    let edges = crates
        .iter()
        .flat_map(|c| c.dependencies())
        .filter(|dep| by_name.contains_key(dep.as_str()))
        .count();

    let violations = crates
        .iter()
        .flat_map(|from| {
            from.sectioned_dependencies()
                .into_iter()
                .filter_map(|(section, dep)| {
                    let to = by_name.get(dep.as_str())?;
                    (!may_depend_on(from, to)).then(|| {
                        format!(
                            "{}: {} ({:?}) → {} ({:?})",
                            key_of(&from.dir, &section, &dep),
                            from.name,
                            from.layer,
                            to.name,
                            to.layer
                        )
                    })
                })
        })
        .collect();

    assert_no_violations(
        "hex_1_dependencies_point_inward",
        "HEX-1: las dependencias apuntan hacia adentro; api solo ve el application de su contexto y edge; \
         api e infrastructure no se ven entre sí; sin dependencias entre contextos",
        &[
            ("crates", crates.len()),
            ("dependencias entre crates del workspace", edges),
        ],
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
    let core: Vec<&Crate> = crates
        .iter()
        .filter(|c| matches!(c.layer, Layer::Kernel | Layer::Domain | Layer::Application))
        .collect();

    let violations = core
        .iter()
        .flat_map(|c| {
            c.sectioned_dependencies()
                .into_iter()
                .filter(|(_, dep)| !internal.contains(&dep.as_str()))
                .map(move |(section, dep)| {
                    format!(
                        "{}: {} ({:?}) usa el crate externo `{dep}`",
                        key_of(&c.dir, &section, &dep),
                        c.name,
                        c.layer
                    )
                })
        })
        .collect();

    assert_no_violations(
        "hex_2_core_depends_only_on_std_and_the_kernel",
        "HEX-2 / TDD-2: domain, application y kernel solo usan std y el kernel, \
         también en [dev-dependencies] (sus tests no levantan runtime ni framework)",
        &[("crates de kernel, domain y application", core.len())],
        violations,
    );
}

#[test]
fn hex_2_server_frameworks_only_at_the_edge() {
    let crates = crates();
    let inner: Vec<&Crate> = crates
        .iter()
        .filter(|c| !matches!(c.layer, Layer::Api | Layer::Edge | Layer::Bootstrap))
        .collect();
    let violations = inner
        .iter()
        .flat_map(|c| {
            c.sectioned_dependencies()
                .into_iter()
                .filter(|(_, dep)| SERVER_FRAMEWORKS.contains(&dep.as_str()))
                .map(move |(section, dep)| {
                    format!(
                        "{}: {} ({:?}) usa `{dep}`",
                        key_of(&c.dir, &section, &dep),
                        c.name,
                        c.layer
                    )
                })
        })
        .collect();

    assert_no_violations(
        "hex_2_server_frameworks_only_at_the_edge",
        "HEX-2: los frameworks de servidor solo viven en api, edge y bootstrap",
        &[(
            "crates fuera de api, edge y bootstrap (kernel, domain, application, infrastructure)",
            inner.len(),
        )],
        violations,
    );
}

/// RS-1: las versiones de los crates se declaran una sola vez, en
/// `[workspace.dependencies]`. Un miembro que fija la suya puede divergir del
/// resto sin que nada lo muestre.
#[test]
fn rs_1_dependencies_inherit_from_the_workspace() {
    let members = workspace_members();
    let mut declared = 0;
    let mut violations = Vec::new();
    for (dir, manifest) in &members {
        declared += declared_dependencies(manifest).len();
        for (section, dep) in non_inherited(manifest) {
            violations.push(format!(
                "{}: [{section}] `{dep}` no hereda del workspace \
                 (declararla en [workspace.dependencies] y usar `{dep}.workspace = true`)",
                key_of(dir, &section, &dep)
            ));
        }
    }

    assert_no_violations(
        "rs_1_dependencies_inherit_from_the_workspace",
        "RS-1: cada dependencia de un miembro hereda del workspace (`workspace = true`)",
        &[
            ("miembros del workspace", members.len()),
            ("dependencias declaradas", declared),
        ],
        violations,
    );
}

/// Resolución de nombres que usan HEX-1 y HEX-2. Sin ella, un renombre
/// (`foo = { package = "axum" }`) esconde el paquete real detrás de la clave.
mod resolution {
    use super::*;

    fn table(text: &str) -> toml::Table {
        text.parse().expect("invariante: TOML de prueba válido")
    }

    fn names(manifest: &str, workspace: &str) -> Vec<String> {
        let workspace = table(workspace);
        let workspace = workspace
            .get("workspace")
            .and_then(|w| w.get("dependencies"))
            .and_then(|d| d.as_table())
            .cloned()
            .unwrap_or_default();
        dependency_names(&table(manifest), &workspace)
    }

    #[test]
    fn a_workspace_rename_resolves_to_the_real_package() {
        let found = names(
            "[dependencies]\nfoo.workspace = true\n",
            "[workspace.dependencies]\nfoo = { package = \"axum\", version = \"0.8\" }\n",
        );

        assert_eq!(found, ["axum"]);
    }

    #[test]
    fn a_local_rename_resolves_to_the_real_package() {
        let found = names(
            "[dependencies]\nfoo = { package = \"axum\", version = \"0.8\" }\n",
            "",
        );

        assert_eq!(found, ["axum"]);
    }

    #[test]
    fn target_specific_dependencies_are_read() {
        let found = names(
            "[target.'cfg(unix)'.dev-dependencies]\ntokio.workspace = true\n",
            "[workspace.dependencies]\ntokio = \"1\"\n",
        );

        assert_eq!(found, ["tokio"]);
    }

    #[test]
    fn a_dependency_violation_is_keyed_by_manifest_section_and_package() {
        let key = dependency_key(
            std::path::Path::new("contexts/x/domain"),
            "target.'cfg(unix)'.dev-dependencies",
            "tokio",
        );

        assert_eq!(
            key,
            "contexts/x/domain/Cargo.toml#target.'cfg(unix)'.dev-dependencies.tokio"
        );
    }

    #[test]
    fn a_plain_dependency_keeps_its_key() {
        let found = names(
            "[dependencies]\nkernel.workspace = true\n",
            "[workspace.dependencies]\nkernel = { path = \"kernel\" }\n",
        );

        assert_eq!(found, ["kernel"]);
    }

    /// Un árbol temporal con un `Cargo.toml` en cada ruta dada.
    fn tree(name: &str, crates: &[&str]) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!("arch-members-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for dir in crates {
            let dir = root.join(dir);
            std::fs::create_dir_all(&dir).expect("invariante: tmp escribible");
            std::fs::write(dir.join("Cargo.toml"), "").expect("invariante: tmp escribible");
        }
        root
    }

    fn members_of(root: &std::path::Path, manifest: &str) -> Vec<String> {
        expand_members(root, &table(manifest))
            .iter()
            .map(|d| {
                d.strip_prefix(root)
                    .map(|p| p.display().to_string())
                    .unwrap_or_default()
            })
            .collect()
    }

    #[test]
    fn members_come_from_the_workspace_globs_not_from_the_layout() {
        let root = tree(
            "globs",
            &[
                "libs/util",
                "contexts/iam",
                "contexts/communities/domain",
                "old/x",
            ],
        );

        let found = members_of(
            &root,
            "[workspace]\nmembers = [\"libs/*\", \"contexts/iam\", \"contexts/communities/*\", \"old/*\"]\n\
             exclude = [\"old/x\"]\n",
        );

        assert_eq!(
            found,
            ["contexts/communities/domain", "contexts/iam", "libs/util"]
        );
    }

    #[test]
    fn a_root_package_is_a_member() {
        let root = tree("root", &[]);

        let found = members_of(
            &root,
            "[package]\nname = \"app\"\n[workspace]\nmembers = []\n",
        );

        assert_eq!(found, [""]);
    }

    #[test]
    fn only_workspace_true_counts_as_inherited() {
        let manifest = table(
            "[dependencies]\n\
             a.workspace = true\n\
             b = { workspace = true, features = [\"x\"] }\n\
             c = \"1\"\n\
             d = { path = \"../d\" }\n\
             [target.'cfg(unix)'.build-dependencies]\n\
             e = { version = \"1\" }\n",
        );

        let found = non_inherited(&manifest);

        assert_eq!(
            found,
            [
                ("dependencies".to_string(), "c".to_string()),
                ("dependencies".to_string(), "d".to_string()),
                (
                    "target.'cfg(unix)'.build-dependencies".to_string(),
                    "e".to_string()
                ),
            ]
        );
    }
}
