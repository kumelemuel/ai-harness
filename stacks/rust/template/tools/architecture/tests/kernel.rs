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
/// métodos, tipos ni constantes asociados). Admite supertraits estándar u otro
/// marcador (`ValueObject: Clone + Eq`, `Identifier: ValueObject`), nunca una
/// capacidad. Cada tipo expone lo suyo.
#[test]
fn hex_8_model_markers_have_no_items() {
    let kernel = kernel();
    let mut violations = Vec::new();
    let mut markers = 0;
    for path in kernel.files("src") {
        let syntax = parse(&path);
        markers += markers_in(&syntax);
        for (line, v) in marker_violations(&syntax) {
            violations.push(format!("{}:{line}: {v}", relative(&path)));
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

/// Un marcador dentro de un módulo en línea también cuenta, y un marcador no
/// puede exigir una capacidad del kernel como supertrait.
#[test]
fn hex_8_marker_violations_cover_inline_modules_and_supertraits() {
    let syntax: syn::File = syn::parse_str(
        "pub mod inline { pub trait Entity { fn id(&self) -> u32; } }
         pub trait Identifier: ValueObject {}
         pub trait AggregateRoot: Clock {}
         #[cfg(test)] mod tests { pub trait ValueObject { fn x(&self); } }",
    )
    .expect("invariante: archivo de prueba válido");

    let found: Vec<String> = marker_violations(&syntax)
        .into_iter()
        .map(|(_, v)| v)
        .collect();

    assert_eq!(
        found,
        [
            "el marcador `Entity` declara [\"fn id\"]: un marcador no tiene ítems",
            "el marcador `AggregateRoot` exige `Clock`, una capacidad: un marcador no la impone",
        ]
    );
}

/// Violaciones de marcadores de un archivo del kernel: `(línea, mensaje)`.
fn marker_violations(syntax: &syn::File) -> Vec<(usize, String)> {
    fn walk(items: &[syn::Item], out: &mut Vec<(usize, String)>) {
        for item in items {
            match item {
                syn::Item::Trait(t) if MODEL_MARKERS.contains(&t.ident.to_string().as_str()) => {
                    let name = t.ident.to_string();
                    let line = t.ident.span().start().line;
                    let items: Vec<String> = t.items.iter().map(trait_item_name).collect();
                    if !items.is_empty() {
                        out.push((
                            line,
                            format!(
                                "el marcador `{name}` declara {items:?}: un marcador no tiene ítems"
                            ),
                        ));
                    }
                    for bound in &t.supertraits {
                        if let syn::TypeParamBound::Trait(b) = bound
                            && let Some(last) = b.path.segments.last()
                            && CAPABILITIES.contains(&last.ident.to_string().as_str())
                        {
                            out.push((
                                line,
                                format!(
                                    "el marcador `{name}` exige `{}`, una capacidad: un marcador no la impone",
                                    last.ident
                                ),
                            ));
                        }
                    }
                }
                syn::Item::Mod(m) if !is_test_module(m) => {
                    if let Some((_, inner)) = &m.content {
                        walk(inner, out);
                    }
                }
                _ => {}
            }
        }
    }
    let mut out = Vec::new();
    walk(&syntax.items, &mut out);
    out
}

/// Marcadores de modelo declarados en un archivo, también en módulos en línea.
fn markers_in(syntax: &syn::File) -> usize {
    fn walk(items: &[syn::Item]) -> usize {
        items
            .iter()
            .map(|item| match item {
                syn::Item::Trait(t) if MODEL_MARKERS.contains(&t.ident.to_string().as_str()) => 1,
                syn::Item::Mod(m) if !is_test_module(m) => {
                    m.content.as_ref().map_or(0, |(_, inner)| walk(inner))
                }
                _ => 0,
            })
            .sum()
    }
    walk(&syntax.items)
}
