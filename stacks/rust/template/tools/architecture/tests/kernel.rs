//! Kernel compartido delgado (HEX-8).

mod common;

use common::{
    CAPABILITIES, Layer, MODEL_MARKERS, assert_no_violations, crates, declared_dependencies,
    dependency_key, parse, relative,
};

fn kernel() -> common::Crate {
    crates()
        .into_iter()
        .find(|c| c.layer == Layer::Kernel)
        .expect("invariante: el workspace tiene kernel")
}

#[test]
fn hex_8_kernel_depends_only_on_std() {
    let kernel = kernel();
    let mut violations = Vec::new();

    // También `[target.'cfg(…)'.*]`: es una dependencia igual.
    let dir = relative(&kernel.dir);
    for (section, dep, _) in declared_dependencies(&kernel.manifest) {
        violations.push(format!(
            "{}: [{section}] declara `{dep}`",
            dependency_key(std::path::Path::new(&dir), &section, &dep)
        ));
    }

    assert_no_violations(
        "hex_8_kernel_depends_only_on_std",
        "HEX-8: el kernel no tiene dependencias, solo std \
         (así las firmas de sus capacidades usan solo std y kernel)",
        // Lee un solo manifiesto, no recorre un conjunto: que no declare
        // dependencias es justo lo esperado.
        &[],
        violations,
    );
}

/// Traits públicos de un archivo, también en módulos en línea (salvo los de test).
fn public_traits(items: &[syn::Item], out: &mut Vec<(usize, String)>) {
    for item in items {
        match item {
            syn::Item::Trait(t) if matches!(t.vis, syn::Visibility::Public(_)) => {
                out.push((t.ident.span().start().line, t.ident.to_string()));
            }
            syn::Item::Mod(m) if !is_test_module(m) => {
                if let Some((_, inner)) = &m.content {
                    public_traits(inner, out);
                }
            }
            _ => {}
        }
    }
}

fn is_test_module(module: &syn::ItemMod) -> bool {
    module.attrs.iter().any(|a| match &a.meta {
        syn::Meta::List(list) => {
            list.path.is_ident("cfg") && list.tokens.to_string().contains("test")
        }
        _ => false,
    })
}

#[test]
fn hex_8_kernel_traits_are_model_markers_or_listed_capabilities() {
    let kernel = kernel();
    let mut violations = Vec::new();
    let mut checked = 0;

    for path in kernel.files("src") {
        let mut traits = Vec::new();
        public_traits(&parse(&path).items, &mut traits);
        checked += traits.len();
        for (line, name) in traits {
            if !MODEL_MARKERS.contains(&name.as_str()) && !CAPABILITIES.contains(&name.as_str()) {
                violations.push(format!(
                    "{}:{line}: `{name}` no es un marcador {MODEL_MARKERS:?} \
                     ni una capacidad {CAPABILITIES:?}",
                    relative(&path)
                ));
            }
        }
    }

    assert_no_violations(
        "hex_8_kernel_traits_are_model_markers_or_listed_capabilities",
        "HEX-8: todo trait público del kernel es un marcador de modelo o una capacidad \
         técnica transversal de la lista cerrada; un puerto de un contexto va en su \
         application/ports (HEX-3)",
        &[("traits públicos del kernel", checked)],
        violations,
    );
}

/// HEX-8: un marcador de modelo no impone API. Es un trait sin ítems (sin
/// métodos, tipos ni constantes asociados); los supertraits sí se admiten
/// (`ValueObject: Clone + Eq`). Cada tipo expone lo suyo.
#[test]
fn hex_8_model_markers_have_no_items() {
    let kernel = kernel();
    let mut violations = Vec::new();
    let mut markers = 0;
    for path in kernel.files("src") {
        for item in &parse(&path).items {
            let syn::Item::Trait(t) = item else {
                continue;
            };
            let name = t.ident.to_string();
            if !MODEL_MARKERS.contains(&name.as_str()) {
                continue;
            }
            markers += 1;
            let items: Vec<String> = t.items.iter().map(trait_item_name).collect();
            if !items.is_empty() {
                violations.push(format!(
                    "{}:{}: el marcador `{name}` declara {items:?}: un marcador no tiene ítems",
                    relative(&path),
                    t.ident.span().start().line
                ));
            }
        }
    }

    assert_no_violations(
        "hex_8_model_markers_have_no_items",
        "HEX-8: un marcador de modelo del kernel es un trait sin ítems",
        &[("marcadores de modelo del kernel", markers)],
        violations,
    );
}

fn trait_item_name(item: &syn::TraitItem) -> String {
    match item {
        syn::TraitItem::Fn(f) => format!("fn {}", f.sig.ident),
        syn::TraitItem::Type(t) => format!("type {}", t.ident),
        syn::TraitItem::Const(c) => format!("const {}", c.ident),
        _ => "un ítem".into(),
    }
}
