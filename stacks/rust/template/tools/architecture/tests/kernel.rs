//! Kernel compartido delgado (HEX-8).

mod common;

use common::{
    CAPABILITIES, Layer, MODEL_MARKERS, assert_no_violations, crates, parse, relative, rust_files,
};

const DEPENDENCY_SECTIONS: &[&str] = &["dependencies", "dev-dependencies", "build-dependencies"];

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

    for section in DEPENDENCY_SECTIONS {
        if let Some(table) = kernel.manifest.get(*section).and_then(|v| v.as_table()) {
            for dep in table.keys() {
                violations.push(format!("kernel/Cargo.toml: [{section}] declara `{dep}`"));
            }
        }
    }
    // `[target.'cfg(…)'.dependencies]` también es una dependencia.
    if let Some(targets) = kernel.manifest.get("target").and_then(|v| v.as_table()) {
        for (cfg, table) in targets {
            for section in DEPENDENCY_SECTIONS {
                if let Some(deps) = table.get(*section).and_then(|v| v.as_table()) {
                    for dep in deps.keys() {
                        violations.push(format!(
                            "kernel/Cargo.toml: [target.{cfg}.{section}] declara `{dep}`"
                        ));
                    }
                }
            }
        }
    }

    assert_no_violations(
        "HEX-8: el kernel no tiene dependencias, solo std \
         (así las firmas de sus capacidades usan solo std y kernel)",
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

    for path in rust_files(&kernel.dir.join("src")) {
        let mut traits = Vec::new();
        public_traits(&parse(&path).items, &mut traits);
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
        "HEX-8: todo trait público del kernel es un marcador de modelo o una capacidad \
         técnica transversal de la lista cerrada; un puerto de un contexto va en su \
         application/ports (HEX-3)",
        violations,
    );
}
