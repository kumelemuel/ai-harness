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
        assert!(layer.dependencies().contains(&"axum".to_string()));
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
