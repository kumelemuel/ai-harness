//! Nombres y vocabulario (HEX-4, HEX-5, HEX-6).

mod common;

use common::{
    CAPABILITIES, Crate, Layer, assert_no_violations, crates, parse, relative, rust_files,
};
use std::path::{Path, PathBuf};
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
/// Tipos del contrato de un caso de uso (HEX-4).
const CONTRACT_SUFFIXES: &[&str] = &["Command", "Query", "Response", "Error"];

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

/// Nombres con los que un archivo de `application` alcanza el crate de dominio.
struct DomainNames {
    /// Identificador del crate (`communities_domain`).
    krate: String,
    /// Nombres locales importados con `use` desde ese crate (tipos o módulos).
    local: Vec<String>,
    /// `use {ctx}_domain::…::*`: cualquier nombre podría venir del dominio.
    glob: bool,
}

impl DomainNames {
    fn of(krate: &str, file: &syn::File) -> Self {
        let mut names = Self {
            krate: krate.to_string(),
            local: Vec::new(),
            glob: false,
        };
        let uses: Vec<&syn::UseTree> = file
            .items
            .iter()
            .filter_map(|i| match i {
                syn::Item::Use(u) => Some(&u.tree),
                _ => None,
            })
            .collect();
        // Dos pasadas: `use communities_domain::value_objects;` puede venir
        // después de `use value_objects::CommunityId;`.
        for _ in 0..2 {
            for tree in &uses {
                for (path, local) in use_leaves(tree) {
                    if self_is_domain(&names, &path) {
                        match local {
                            Some(name) if !names.local.contains(&name) => names.local.push(name),
                            Some(_) => {}
                            None => names.glob = true,
                        }
                    }
                }
            }
        }
        names
    }

    fn is_domain(&self, path: &syn::Path) -> bool {
        path.segments
            .first()
            .is_some_and(|s| s.ident == self.krate || self.local.iter().any(|l| s.ident == l))
    }
}

fn self_is_domain(names: &DomainNames, path: &[String]) -> bool {
    path.first()
        .is_some_and(|first| *first == names.krate || names.local.contains(first))
}

/// Hojas de un `use`: `(ruta completa, nombre local)`; `None` si es un glob.
fn use_leaves(tree: &syn::UseTree) -> Vec<(Vec<String>, Option<String>)> {
    fn walk(
        tree: &syn::UseTree,
        prefix: &mut Vec<String>,
        out: &mut Vec<(Vec<String>, Option<String>)>,
    ) {
        match tree {
            syn::UseTree::Path(p) => {
                prefix.push(p.ident.to_string());
                walk(&p.tree, prefix, out);
                prefix.pop();
            }
            syn::UseTree::Name(n) if n.ident == "self" => {
                out.push((prefix.clone(), prefix.last().cloned()));
            }
            syn::UseTree::Name(n) => {
                let mut path = prefix.clone();
                path.push(n.ident.to_string());
                out.push((path, Some(n.ident.to_string())));
            }
            syn::UseTree::Rename(r) => {
                let mut path = prefix.clone();
                path.push(r.ident.to_string());
                out.push((path, Some(r.rename.to_string())));
            }
            syn::UseTree::Glob(_) => out.push((prefix.clone(), None)),
            syn::UseTree::Group(g) => {
                for item in &g.items {
                    walk(item, prefix, out);
                }
            }
        }
    }
    let mut out = Vec::new();
    walk(tree, &mut Vec::new(), &mut out);
    out
}

/// Rutas de dominio dentro de un fragmento de sintaxis (un tipo, una firma).
struct DomainPaths<'n> {
    names: &'n DomainNames,
    found: Vec<(usize, String)>,
}

impl<'ast> Visit<'ast> for DomainPaths<'_> {
    fn visit_path(&mut self, path: &'ast syn::Path) {
        if self.names.is_domain(path) {
            let text = path
                .segments
                .iter()
                .map(|s| s.ident.to_string())
                .collect::<Vec<_>>()
                .join("::");
            let line = path
                .segments
                .first()
                .map_or(0, |s| s.ident.span().start().line);
            self.found.push((line, text));
        }
        syn::visit::visit_path(self, path);
    }
}

fn domain_paths(names: &DomainNames, visit: impl FnOnce(&mut DomainPaths)) -> Vec<(usize, String)> {
    let mut paths = DomainPaths {
        names,
        found: Vec::new(),
    };
    visit(&mut paths);
    paths.found
}

fn is_test_module(module: &syn::ItemMod) -> bool {
    module.attrs.iter().any(|a| match &a.meta {
        syn::Meta::List(list) => {
            list.path.is_ident("cfg") && list.tokens.to_string().contains("test")
        }
        _ => false,
    })
}

/// Ítems de un archivo, incluidos los de módulos en línea que no son de test.
fn items_of(items: &[syn::Item]) -> Vec<&syn::Item> {
    items
        .iter()
        .flat_map(|item| match item {
            syn::Item::Mod(m) if !is_test_module(m) => m
                .content
                .as_ref()
                .map(|(_, inner)| items_of(inner))
                .unwrap_or_default(),
            other => vec![other],
        })
        .collect()
}

fn is_under(file: &Path, krate: &Crate, subdirs: &[&str]) -> bool {
    subdirs
        .iter()
        .any(|sub| file.starts_with(krate.dir.join(sub)))
}

/// El crate de dominio del mismo contexto, si `application` lo declara.
fn domain_crate_of(application: &Crate, all: &[Crate]) -> Option<String> {
    all.iter()
        .find(|c| c.layer == Layer::Domain && c.context == application.context)
        .filter(|d| application.dependencies().contains(&d.name))
        .map(|d| d.name.replace('-', "_"))
}

#[test]
fn hex_4_use_case_contracts_do_not_expose_domain_types() {
    let all = crates();
    let mut violations = Vec::new();

    for app in all.iter().filter(|c| c.layer == Layer::Application) {
        let Some(krate) = domain_crate_of(app, &all) else {
            continue;
        };
        for path in rust_files(&app.dir.join("src")) {
            let syntax = parse(&path);
            let names = DomainNames::of(&krate, &syntax);
            let file = relative(&path);
            let in_cases = is_under(&path, app, &["src/commands", "src/queries"]);
            let in_inbound = is_under(&path, app, &["src/ports/inbound"]);
            let mut contract_in_file = false;

            for item in items_of(&syntax.items) {
                match item {
                    // Reexportar el dominio lo vuelve parte de la superficie de application.
                    syn::Item::Use(u) if matches!(u.vis, syn::Visibility::Public(_)) => {
                        for (leaf, _) in use_leaves(&u.tree) {
                            if self_is_domain(&names, &leaf) {
                                let line = u.use_token.span.start().line;
                                violations.push(format!(
                                    "{file}:{line}: `pub use {}` reexporta el dominio",
                                    leaf.join("::")
                                ));
                            }
                        }
                    }
                    syn::Item::Struct(s)
                        if in_cases
                            && CONTRACT_SUFFIXES
                                .iter()
                                .any(|x| s.ident.to_string().ends_with(x)) =>
                    {
                        contract_in_file = true;
                        for field in s
                            .fields
                            .iter()
                            .filter(|f| matches!(f.vis, syn::Visibility::Public(_)))
                        {
                            for (line, found) in domain_paths(&names, |v| v.visit_type(&field.ty)) {
                                violations.push(format!(
                                    "{file}:{line}: el campo público de `{}` expone `{found}`",
                                    s.ident
                                ));
                            }
                        }
                    }
                    syn::Item::Enum(e)
                        if in_cases
                            && CONTRACT_SUFFIXES
                                .iter()
                                .any(|x| e.ident.to_string().ends_with(x)) =>
                    {
                        contract_in_file = true;
                        for variant in &e.variants {
                            for field in &variant.fields {
                                for (line, found) in
                                    domain_paths(&names, |v| v.visit_type(&field.ty))
                                {
                                    violations.push(format!(
                                        "{file}:{line}: la variante `{}::{}` expone `{found}`",
                                        e.ident, variant.ident
                                    ));
                                }
                            }
                        }
                    }
                    syn::Item::Trait(t) if in_inbound => {
                        contract_in_file = true;
                        let mut found = domain_paths(&names, |v| {
                            v.visit_generics(&t.generics);
                            for bound in &t.supertraits {
                                v.visit_type_param_bound(bound);
                            }
                            for trait_item in &t.items {
                                match trait_item {
                                    syn::TraitItem::Fn(f) => v.visit_signature(&f.sig),
                                    syn::TraitItem::Type(ty) => v.visit_trait_item_type(ty),
                                    _ => {}
                                }
                            }
                        });
                        found.dedup();
                        for (line, found) in found {
                            violations.push(format!(
                                "{file}:{line}: la firma de `{}` expone `{found}`",
                                t.ident
                            ));
                        }
                    }
                    _ => {}
                }
            }

            if names.glob && contract_in_file {
                violations.push(format!(
                    "{file}: `use {krate}::…::*` junto a un contrato impide verificar que no exponga el dominio"
                ));
            }
        }
    }

    assert_no_violations(
        "HEX-4: el contrato del caso de uso (Command, Query, Response, Error y el UseCase) \
         no expone tipos de domain; api no ve domain (HEX-1)",
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

/// Concepto del adaptador: el puerto sin su sufijo de rol (HEX-5) o, si es una
/// capacidad del kernel, su nombre completo (HEX-8).
fn concept_of(contract: &str) -> &str {
    if CAPABILITIES.contains(&contract) {
        return contract;
    }
    OUTBOUND_SUFFIXES
        .iter()
        .find_map(|suffix| contract.strip_suffix(suffix))
        .filter(|concept| !concept.is_empty())
        .unwrap_or(contract)
}

/// Archivos donde viven adaptadores de un contexto: todo `infrastructure` y los
/// dobles de test de `application` (`src/testing.rs` y `src/testing/`), que se
/// nombran como adaptadores con `Fake`, `Fixed` o `Stub` de prefijo (TDD-2).
fn adapter_files(context: &Crate) -> Vec<PathBuf> {
    match context.layer {
        Layer::Infrastructure => rust_files(&context.dir.join("src")),
        Layer::Application => {
            let module = context.dir.join("src/testing.rs");
            let mut files = rust_files(&context.dir.join("src/testing"));
            if module.exists() {
                files.insert(0, module);
            }
            files
        }
        _ => Vec::new(),
    }
}

#[test]
fn hex_5_adapters_are_named_by_technology_and_concept() {
    let all = crates();
    let mut violations = Vec::new();

    for krate in &all {
        let files = adapter_files(krate);
        if files.is_empty() {
            continue;
        }
        let ports: Vec<String> = all
            .iter()
            .filter(|c| c.layer == Layer::Application && c.context == krate.context)
            .flat_map(|app| scan(&[app.dir.join("src/ports/outbound")], Some("dtos")))
            .flat_map(|(_, items)| items.traits.into_iter().map(|t| t.ident.to_string()))
            .collect();

        for path in files {
            let mut items = Items::default();
            items.visit_file(&parse(&path));
            for imp in &items.impls {
                let Some((_, trait_path, _)) = &imp.trait_ else {
                    continue;
                };
                let contract = last_segment(trait_path);
                if !ports.contains(&contract) && !CAPABILITIES.contains(&contract.as_str()) {
                    continue;
                }
                let syn::Type::Path(self_ty) = &*imp.self_ty else {
                    continue;
                };
                let name = last_segment(&self_ty.path);
                let concept = concept_of(&contract);
                let named = name
                    .strip_suffix("Adapter")
                    .and_then(|stem| stem.strip_suffix(concept))
                    .is_some_and(|prefix| !prefix.is_empty());
                if !named {
                    let line = self_ty
                        .path
                        .segments
                        .last()
                        .map_or(0, |s| s.ident.span().start().line);
                    violations.push(format!(
                        "{}:{line}: `{name}` implementa `{contract}` y debe llamarse \
                         {{Prefijo}}{concept}Adapter",
                        relative(&path)
                    ));
                }
            }
        }
    }

    assert_no_violations(
        "HEX-5: un adaptador se llama {Tecnología}{Concepto}Adapter; Concepto es el puerto \
         sin su sufijo de rol o, si es una capacidad del kernel (HEX-8), su nombre completo",
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
