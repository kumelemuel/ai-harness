//! Nombres y vocabulario (HEX-4, HEX-5, HEX-6).

mod common;

use common::{Layer, assert_no_violations, crates, parse, relative, rust_files};
use std::path::PathBuf;
use syn::visit::Visit;

const OUTBOUND_SUFFIXES: &[&str] = &[
    "Repository",
    "Cache",
    "Gateway",
    "Client",
    "Synchronizer",
    "Service",
];
const RESERVED_SUFFIXES: &[&str] = &["Command", "Query", "Response", "Result"];

fn dirs_of(layer: Layer, sub: &str) -> Vec<PathBuf> {
    crates()
        .into_iter()
        .filter(|c| c.layer == layer)
        .map(|c| c.dir.join(sub))
        .filter(|d| d.exists())
        .collect()
}

#[derive(Default)]
struct Items {
    traits: Vec<syn::ItemTrait>,
    types: Vec<String>,
    impls: Vec<syn::ItemImpl>,
    idents: Vec<String>,
}

impl<'ast> Visit<'ast> for Items {
    fn visit_item_trait(&mut self, node: &'ast syn::ItemTrait) {
        self.traits.push(node.clone());
        syn::visit::visit_item_trait(self, node);
    }
    fn visit_item_struct(&mut self, node: &'ast syn::ItemStruct) {
        self.types.push(node.ident.to_string());
        syn::visit::visit_item_struct(self, node);
    }
    fn visit_item_enum(&mut self, node: &'ast syn::ItemEnum) {
        self.types.push(node.ident.to_string());
        syn::visit::visit_item_enum(self, node);
    }
    fn visit_item_impl(&mut self, node: &'ast syn::ItemImpl) {
        self.impls.push(node.clone());
        syn::visit::visit_item_impl(self, node);
    }
    fn visit_ident(&mut self, node: &'ast proc_macro2::Ident) {
        self.idents.push(node.to_string());
    }
}

fn scan(dirs: &[PathBuf], skip: Option<&str>) -> Vec<(String, Items)> {
    dirs.iter()
        .flat_map(|d| rust_files(d))
        .filter(|f| skip.is_none_or(|s| !f.components().any(|c| c.as_os_str() == s)))
        .map(|f| {
            let mut items = Items::default();
            items.visit_file(&parse(&f));
            (relative(&f), items)
        })
        .collect()
}

fn last_segment(path: &syn::Path) -> String {
    path.segments
        .last()
        .map(|s| s.ident.to_string())
        .unwrap_or_default()
}

#[test]
fn hex_4_inbound_ports_are_single_method_use_cases() {
    let mut violations = Vec::new();

    for (file, items) in scan(&dirs_of(Layer::Application, "src/ports/inbound"), None) {
        for t in &items.traits {
            let name = t.ident.to_string();
            if !name.ends_with("UseCase") {
                violations.push(format!(
                    "{file}: `{name}` debe llamarse {{Verb}}{{Noun}}UseCase"
                ));
            }
            let methods: Vec<String> = t
                .items
                .iter()
                .filter_map(|i| match i {
                    syn::TraitItem::Fn(f) => Some(f.sig.ident.to_string()),
                    _ => None,
                })
                .collect();
            if methods != ["execute"] {
                violations.push(format!(
                    "{file}: `{name}` debe declarar solo `execute`, tiene {methods:?}"
                ));
            }
        }
    }

    assert_no_violations(
        "HEX-4: un puerto entrante = un `UseCase` con un único `execute`",
        violations,
    );
}

#[test]
fn hex_4_use_case_implementations_are_handlers() {
    let dirs: Vec<PathBuf> = ["src/commands", "src/queries"]
        .iter()
        .flat_map(|sub| dirs_of(Layer::Application, sub))
        .collect();
    let mut violations = Vec::new();

    for (file, items) in scan(&dirs, None) {
        for imp in &items.impls {
            let Some((_, trait_path, _)) = &imp.trait_ else {
                continue;
            };
            if !last_segment(trait_path).ends_with("UseCase") {
                continue;
            }
            let syn::Type::Path(self_ty) = &*imp.self_ty else {
                continue;
            };
            let name = last_segment(&self_ty.path);
            if !name.ends_with("Handler") {
                violations.push(format!(
                    "{file}: `{name}` implementa un UseCase y debe llamarse *Handler"
                ));
            }
        }
    }

    assert_no_violations(
        "HEX-4: la implementación de un caso de uso se llama {Verb}{Noun}Handler",
        violations,
    );
}

#[test]
fn hex_4_api_depends_on_use_cases_never_on_handlers() {
    let dirs = [dirs_of(Layer::Api, "src"), dirs_of(Layer::Api, "tests")].concat();
    let mut violations = Vec::new();

    for (file, items) in scan(&dirs, None) {
        for ident in items
            .idents
            .iter()
            .filter(|i| i.ends_with("Handler") && *i != "Handler")
        {
            violations.push(format!("{file}: referencia a `{ident}`"));
        }
    }

    assert_no_violations(
        "HEX-4: api depende del UseCase, nunca del Handler",
        violations,
    );
}

#[test]
fn hex_5_outbound_ports_are_named_by_role() {
    let mut violations = Vec::new();

    for (file, items) in scan(
        &dirs_of(Layer::Application, "src/ports/outbound"),
        Some("dtos"),
    ) {
        for t in &items.traits {
            let name = t.ident.to_string();
            if name.ends_with("Port") || !OUTBOUND_SUFFIXES.iter().any(|s| name.ends_with(s)) {
                violations.push(format!(
                    "{file}: `{name}` debe terminar en uno de {OUTBOUND_SUFFIXES:?}"
                ));
            }
        }
    }

    assert_no_violations(
        "HEX-5: puertos salientes con sufijo de rol, nunca `Port`",
        violations,
    );
}

#[test]
fn hex_6_outbound_dtos_do_not_use_reserved_vocabulary() {
    let mut violations = Vec::new();

    for (file, items) in scan(
        &dirs_of(Layer::Application, "src/ports/outbound/dtos"),
        None,
    ) {
        for name in &items.types {
            if let Some(s) = RESERVED_SUFFIXES.iter().find(|s| name.ends_with(*s)) {
                violations.push(format!("{file}: `{name}` usa el sufijo reservado `{s}`"));
            }
        }
    }

    assert_no_violations(
        "HEX-6: DTOs de puerto saliente con `Request` o el nombre del concepto",
        violations,
    );
}

#[test]
fn hex_6_transport_types_carry_the_transport_name() {
    let mut violations = Vec::new();

    for (file, items) in scan(&dirs_of(Layer::Api, "src"), None) {
        for name in &items.types {
            let is_transport = name.ends_with("Request") || name.ends_with("Response");
            let named = name.ends_with("HttpRequest") || name.ends_with("HttpResponse");
            if is_transport && !named {
                violations.push(format!(
                    "{file}: `{name}` debe ser {{Caso}}HttpRequest / {{Caso}}HttpResponse"
                ));
            }
        }
    }

    assert_no_violations(
        "HEX-6: tipos de transporte con el nombre del transporte",
        violations,
    );
}
