//! Modo legacy (adopción): un crate por contexto con las capas como módulos, y
//! `shared/` como kernel. Se borra cuando no queda legacy (README del stack).

mod common;

use common::{Layer, crates_at};
use std::path::{Path, PathBuf};

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/legacy")
}

fn layer_src(layer: Layer) -> Option<PathBuf> {
    crates_at(&fixture())
        .into_iter()
        .find(|c| c.layer == layer && c.context.as_deref() == Some("iam"))
        .map(|c| c.src)
}

#[test]
fn legacy_modules_are_virtual_layers() {
    let src = fixture().join("contexts/iam/src");

    assert_eq!(layer_src(Layer::Domain), Some(src.join("domain")));
    assert_eq!(layer_src(Layer::Application), Some(src.join("application")));
    assert_eq!(
        layer_src(Layer::Infrastructure),
        Some(src.join("infrastructure"))
    );
    assert_eq!(layer_src(Layer::Api), Some(src.join("infrastructure/http")));
}

#[test]
fn legacy_layers_share_the_manifest_and_have_no_tests_dir() {
    let crates = crates_at(&fixture());
    let iam: Vec<_> = crates
        .iter()
        .filter(|c| c.context.as_deref() == Some("iam"))
        .collect();

    assert_eq!(iam.len(), 4);
    for layer in &iam {
        assert!(layer.legacy, "{:?}", layer.layer);
        assert_eq!(layer.name, "iam");
        assert!(layer.dependencies().iter().any(|d| d == "axum"));
        assert_eq!(layer.path("tests"), None, "{:?}", layer.layer);
    }
}

#[test]
fn shared_is_the_kernel_when_there_is_no_kernel_dir() {
    let kernel = crates_at(&fixture())
        .into_iter()
        .find(|c| c.layer == Layer::Kernel)
        .map(|c| c.name);

    assert_eq!(kernel.as_deref(), Some("shared"));
}

fn files_of(context: &str, layer: Layer) -> Vec<String> {
    let root = fixture().join("contexts").join(context).join("src");
    crates_at(&fixture())
        .into_iter()
        .find(|c| c.layer == layer && c.context.as_deref() == Some(context))
        .map(|c| c.files("src"))
        .unwrap_or_default()
        .iter()
        .map(|f| f.strip_prefix(&root).unwrap_or(f).display().to_string())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect()
}

/// `src/domain.rs` es parte de la capa, junto a `src/domain/`.
#[test]
fn a_module_file_belongs_to_its_layer() {
    assert_eq!(
        files_of("billing", Layer::Domain),
        ["domain.rs", "domain/invoice.rs"]
    );
}

/// `infrastructure/http` es Api: no se lee también como Infrastructure.
#[test]
fn http_is_read_once_as_api() {
    assert_eq!(files_of("iam", Layer::Api), ["infrastructure/http/mod.rs"]);
    assert_eq!(
        files_of("iam", Layer::Infrastructure),
        ["infrastructure/ids.rs", "infrastructure/mod.rs"]
    );
}

/// `infrastructure/http.rs` (archivo, no directorio) también es Api, y no se
/// lee dos veces.
#[test]
fn an_http_file_is_read_once_as_api() {
    assert_eq!(files_of("billing", Layer::Api), ["infrastructure/http.rs"]);
    assert_eq!(
        files_of("billing", Layer::Infrastructure),
        ["infrastructure.rs"]
    );
}
