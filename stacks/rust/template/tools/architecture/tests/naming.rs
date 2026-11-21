//! Nombres y vocabulario (HEX-4, HEX-5, HEX-6, HEX-7).

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
    "Publisher",
    "Service",
];
/// Puertos salientes con nombre propio, sin sufijo de rol (HEX-5): patrones
/// conocidos cuyo nombre ya dice el rol. Su Concepto es el nombre completo.
const NAMED_OUTBOUND_PORTS: &[&str] = &["UnitOfWork"];
/// Puertos entrantes (HEX-4): sufijo del trait y su único método.
const INBOUND_KINDS: &[(&str, &str)] = &[("UseCase", "execute"), ("EventSubscriber", "on")];
const RESERVED_SUFFIXES: &[&str] = &["Command", "Query", "Response", "Result"];
/// Tipos del contrato de un caso de uso (HEX-4).
const CONTRACT_SUFFIXES: &[&str] = &["Command", "Query", "Response", "Error"];

/// `true` si `file` (relativo a la raíz, como lo da `scan`) está bajo alguno de
/// `dirs`. Compara rutas resueltas por capa, no texto: en legacy,
/// `src/workflows` es `src/application/workflows`.
fn under(file: &str, dirs: &[PathBuf]) -> bool {
    let absolute = common::root().join(file);
    dirs.iter().any(|d| absolute.starts_with(d))
}

fn dirs_of(layer: Layer, sub: &str) -> Vec<PathBuf> {
    crates()
        .into_iter()
        .filter(|c| c.layer == layer)
        .filter_map(|c| c.path(sub))
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
    let mut traits = 0;

    for (file, items) in scan(&dirs_of(Layer::Application, "src/ports/inbound"), None) {
        for t in &items.traits {
            traits += 1;
            let name = t.ident.to_string();
            // Dos tipos de puerto entrante: el caso de uso, que invoca un cliente,
            // y el suscriptor, que invoca el despachador de eventos (HEX-7).
            let Some((_, method)) = INBOUND_KINDS
                .iter()
                .find(|(suffix, _)| name.ends_with(suffix) && name != *suffix)
            else {
                violations.push(format!(
                    "{file}: `{name}` debe llamarse {{Verb}}{{Noun}}UseCase \
                     o {{Concepto}}EventSubscriber"
                ));
                continue;
            };
            let methods: Vec<String> = t
                .items
                .iter()
                .filter_map(|i| match i {
                    syn::TraitItem::Fn(f) => Some(f.sig.ident.to_string()),
                    _ => None,
                })
                .collect();
            if methods != [*method] {
                violations.push(format!(
                    "{file}: `{name}` debe declarar solo `{method}`, tiene {methods:?}"
                ));
            }
        }
    }

    assert_no_violations(
        "hex_4_inbound_ports_are_single_method_use_cases",
        "HEX-4: un puerto entrante es un `UseCase` con un único `execute` \
         o un `EventSubscriber` con un único `on`",
        &[("traits de ports/inbound", traits)],
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
    let mut implementations = 0;

    for (file, items) in scan(&dirs, None) {
        for imp in &items.impls {
            let Some((_, trait_path, _)) = &imp.trait_ else {
                continue;
            };
            if !last_segment(trait_path).ends_with("UseCase") {
                continue;
            }
            implementations += 1;
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
        "hex_4_use_case_implementations_are_handlers",
        "HEX-4: la implementación de un caso de uso se llama {Verb}{Noun}Handler",
        &[(
            "implementaciones de un UseCase en commands/ y queries/",
            implementations,
        )],
        violations,
    );
}

#[test]
fn hex_4_api_depends_on_use_cases_never_on_handlers() {
    let dirs = [dirs_of(Layer::Api, "src"), dirs_of(Layer::Api, "tests")].concat();
    let mut violations = Vec::new();
    let files = scan(&dirs, None);
    let scanned = files.len();

    for (file, items) in files {
        for ident in items
            .idents
            .iter()
            .filter(|i| i.ends_with("Handler") && *i != "Handler")
        {
            violations.push(format!("{file}: referencia a `{ident}`"));
        }
    }

    assert_no_violations(
        "hex_4_api_depends_on_use_cases_never_on_handlers",
        "HEX-4: api depende del UseCase, nunca del Handler",
        &[("archivos de api (src/ y tests/)", scanned)],
        violations,
    );
}

/// Nombres con los que un archivo de `application` alcanza el crate de dominio.
struct DomainNames {
    /// Ruta raíz del dominio: `["communities_domain"]`, o `["crate", "domain"]`
    /// en un contexto legacy.
    root: Vec<String>,
    /// Nombres locales importados con `use` desde ese crate (tipos o módulos).
    local: Vec<String>,
    /// `use {ctx}_domain::…::*`: cualquier nombre podría venir del dominio.
    glob: bool,
    /// legacy: ruta del módulo del archivo (`["application", "commands", …]`),
    /// para resolver `super::` y `self::` contra `crate::`.
    module: Option<Vec<String>>,
}

/// legacy: ruta de módulo de `file` dentro de `src` (sin `mod`, `lib` ni `main`).
fn module_path(src: &Path, file: &Path) -> Vec<String> {
    let relative = file.strip_prefix(src).unwrap_or(file).with_extension("");
    relative
        .iter()
        .map(|c| c.to_string_lossy().to_string())
        .filter(|c| !matches!(c.as_str(), "mod" | "lib" | "main"))
        .collect()
}

impl DomainNames {
    /// legacy: ruta absoluta (`crate::…`) de una que empieza por `super` o `self`.
    fn absolute(&self, path: &[String]) -> Vec<String> {
        let Some(module) = &self.module else {
            return path.to_vec();
        };
        let ups = path.iter().take_while(|s| *s == "super").count();
        let rest = if ups > 0 {
            &path[ups..]
        } else if path.first().is_some_and(|s| s == "self") {
            &path[1..]
        } else {
            return path.to_vec();
        };
        let base = &module[..module.len().saturating_sub(ups)];
        std::iter::once("crate".to_string())
            .chain(base.iter().cloned())
            .chain(rest.iter().cloned())
            .collect()
    }

    fn of(root: &[String], module: Option<Vec<String>>, file: &syn::File) -> Self {
        let mut names = Self {
            root: root.to_vec(),
            module,
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
        let segments: Vec<String> = path.segments.iter().map(|s| s.ident.to_string()).collect();
        self_is_domain(self, &segments)
    }
}

fn self_is_domain(names: &DomainNames, path: &[String]) -> bool {
    names.absolute(path).starts_with(&names.root)
        || path
            .first()
            .is_some_and(|first| names.local.contains(first))
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
        .any(|sub| krate.path(sub).is_some_and(|p| file.starts_with(p)))
}

/// Ruta del dominio del mismo contexto, vista desde `application`: el crate
/// `{ctx}-domain` si lo declara, o `crate::domain` en un contexto legacy.
fn domain_crate_of(application: &Crate, all: &[Crate]) -> Option<Vec<String>> {
    let domain = all
        .iter()
        .find(|c| c.layer == Layer::Domain && c.context == application.context)?;
    // legacy: el dominio es un módulo del mismo crate.
    if application.legacy {
        return Some(vec!["crate".into(), "domain".into()]);
    }
    application
        .dependencies()
        .contains(&domain.name)
        .then(|| vec![domain.name.replace('-', "_")])
}

/// Violaciones de HEX-4 en los contratos de `all`, y cuántos contratos revisó.
fn contract_violations(all: &[Crate]) -> (Vec<String>, usize) {
    let mut violations = Vec::new();
    let mut contracts = 0;

    for app in all.iter().filter(|c| c.layer == Layer::Application) {
        let Some(krate) = domain_crate_of(app, all) else {
            continue;
        };
        for path in app.files("src") {
            let syntax = parse(&path);
            let module = app.legacy.then(|| module_path(&app.dir.join("src"), &path));
            let names = DomainNames::of(&krate, module, &syntax);
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
                        contracts += 1;
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
                        contracts += 1;
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
                    // Un `EventSubscriber` lo invoca el despachador de eventos, que
                    // vive en infraestructura y ve `domain` (HEX-1): recibe el evento
                    // de dominio. Solo el `UseCase` es contrato de `api`.
                    syn::Item::Trait(t)
                        if in_inbound && !t.ident.to_string().ends_with("EventSubscriber") =>
                    {
                        contract_in_file = true;
                        contracts += 1;
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
                    "{file}: `use {}::…::*` junto a un contrato impide verificar que no exponga el dominio",
                    krate.join("::")
                ));
            }
        }
    }

    (violations, contracts)
}

#[test]
fn hex_4_use_case_contracts_do_not_expose_domain_types() {
    let (violations, contracts) = contract_violations(&crates());
    assert_no_violations(
        "hex_4_use_case_contracts_do_not_expose_domain_types",
        "HEX-4: el contrato del caso de uso (Command, Query, Response, Error y el UseCase) \
         no expone tipos de domain; api no ve domain (HEX-1)",
        &[(
            "tipos de contrato (Command, Query, Response, Error) y traits UseCase",
            contracts,
        )],
        violations,
    );
}

#[test]
fn hex_5_outbound_ports_are_named_by_role() {
    let mut violations = Vec::new();
    let mut ports = 0;

    for (file, items) in scan(
        &dirs_of(Layer::Application, "src/ports/outbound"),
        Some("dtos"),
    ) {
        for t in &items.traits {
            ports += 1;
            let name = t.ident.to_string();
            let named = NAMED_OUTBOUND_PORTS.contains(&name.as_str());
            if !named
                && (name.ends_with("Port") || !OUTBOUND_SUFFIXES.iter().any(|s| name.ends_with(s)))
            {
                violations.push(format!(
                    "{file}: `{name}` debe terminar en uno de {OUTBOUND_SUFFIXES:?} \
                     o ser uno de {NAMED_OUTBOUND_PORTS:?}"
                ));
            }
        }
    }

    assert_no_violations(
        "hex_5_outbound_ports_are_named_by_role",
        "HEX-5: puertos salientes con sufijo de rol o nombre propio de la lista, nunca `Port`",
        &[("traits de ports/outbound", ports)],
        violations,
    );
}

/// Concepto del adaptador: el puerto sin su sufijo de rol (HEX-5) o, si es una
/// capacidad del kernel, su nombre completo (HEX-8).
fn concept_of(contract: &str) -> &str {
    if CAPABILITIES.contains(&contract) || NAMED_OUTBOUND_PORTS.contains(&contract) {
        return contract;
    }
    OUTBOUND_SUFFIXES
        .iter()
        .find_map(|suffix| contract.strip_suffix(suffix))
        .filter(|concept| !concept.is_empty())
        .unwrap_or(contract)
}

/// Prefijos de los dobles de test (TDD-2): lista cerrada. `Fake` tiene estado,
/// `Fixed` y `Stub` responden fijo, `Spy` registra llamadas. Ningún adaptador
/// de producción los usa (HEX-5).
const DOUBLE_PREFIXES: &[&str] = &["Fake", "Fixed", "Stub", "Spy"];

/// ¿Un `cfg` exige o habilita `test`? `cfg(test)`, `cfg(all(test, …))` y
/// `cfg(any(test, feature = "testing"))` sí: es código de test o de soporte a
/// tests (los dobles de `testing`). `cfg(not(test))` no.
fn mentions_test(attrs: &[syn::Attribute]) -> bool {
    fn predicate(meta: &syn::Meta) -> bool {
        match meta {
            syn::Meta::Path(p) => p.is_ident("test"),
            syn::Meta::List(l) if l.path.is_ident("all") || l.path.is_ident("any") => l
                .parse_args_with(
                    syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
                )
                .is_ok_and(|args| args.iter().any(predicate)),
            _ => false,
        }
    }
    attrs.iter().any(|attr| {
        attr.path().is_ident("cfg")
            && attr
                .parse_args::<syn::Meta>()
                .is_ok_and(|meta| predicate(&meta))
    })
}

/// `#[test]`, `#[tokio::test]`: un `impl` dentro de la función es de test.
fn is_test_fn(attrs: &[syn::Attribute]) -> bool {
    attrs
        .iter()
        .any(|a| a.path().segments.last().is_some_and(|s| s.ident == "test"))
}

/// Implementaciones de trait de un archivo, marcadas si están en código de test.
struct TraitImpls {
    in_test: bool,
    found: Vec<(syn::ItemImpl, bool)>,
    /// `#[cfg(test)] mod x;`: archivos externos que son de test.
    test_modules: Vec<String>,
}

impl TraitImpls {
    fn within(&mut self, test: bool, visit: impl FnOnce(&mut Self)) {
        let outer = self.in_test;
        self.in_test |= test;
        visit(self);
        self.in_test = outer;
    }
}

impl<'ast> Visit<'ast> for TraitImpls {
    fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
        let test = mentions_test(&node.attrs);
        if node.content.is_none() && (test || self.in_test) {
            self.test_modules.push(node.ident.to_string());
        }
        self.within(test, |v| syn::visit::visit_item_mod(v, node));
    }
    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        let test = mentions_test(&node.attrs) || is_test_fn(&node.attrs);
        self.within(test, |v| syn::visit::visit_item_fn(v, node));
    }
    fn visit_item_impl(&mut self, node: &'ast syn::ItemImpl) {
        let test = self.in_test || mentions_test(&node.attrs);
        if node.trait_.is_some() {
            self.found.push((node.clone(), test));
        }
        self.within(test, |v| syn::visit::visit_item_impl(v, node));
    }
}

/// Directorio donde viven los submódulos de un archivo.
fn module_dir(file: &Path) -> PathBuf {
    let dir = file.parent().expect("invariante: archivo en un directorio");
    match file.file_stem().and_then(|s| s.to_str()) {
        Some("lib" | "main" | "mod") | None => dir.to_path_buf(),
        Some(stem) => dir.join(stem),
    }
}

/// Implementaciones de trait de un crate, en `src/` y `tests/`: `(archivo, impl, ¿test?)`.
///
/// Es de test todo `tests/`, lo que está bajo un `cfg` que menciona `test`
/// (también los archivos de un `#[cfg(test)] mod x;` y de
/// `#[cfg(any(test, feature = "testing"))] mod testing;`) y lo que está dentro
/// de una función `#[test]`.
fn trait_impls(krate: &Crate) -> Vec<(PathBuf, syn::ItemImpl, bool)> {
    let tests_dir = krate.path("tests");
    let files = [krate.files("src"), krate.files("tests")].concat();
    let mut parsed = Vec::new();
    let mut test_paths: Vec<PathBuf> = Vec::new();
    for file in files {
        let syntax = parse(&file);
        let mut impls = TraitImpls {
            in_test: mentions_test(&syntax.attrs), // `#![cfg(test)]`
            found: Vec::new(),
            test_modules: Vec::new(),
        };
        impls.visit_file(&syntax);
        let dir = module_dir(&file);
        for name in &impls.test_modules {
            test_paths.push(dir.join(format!("{name}.rs")));
            test_paths.push(dir.join(name)); // `x/mod.rs` y los submódulos de `x.rs`
        }
        parsed.push((file, impls.found));
    }
    parsed
        .into_iter()
        .flat_map(|(file, found)| {
            let file_is_test = tests_dir.as_ref().is_some_and(|t| file.starts_with(t))
                || test_paths.iter().any(|t| file.starts_with(t));
            found
                .into_iter()
                .map(move |(imp, test)| (file.clone(), imp, test || file_is_test))
        })
        .collect::<Vec<_>>()
}

/// Primera palabra de un identificador en CamelCase: `NoOp` → `No`, `Fake` → `Fake`.
fn first_word(name: &str) -> &str {
    let end = name
        .char_indices()
        .skip(1)
        .find(|(_, c)| c.is_uppercase())
        .map_or(name.len(), |(i, _)| i);
    &name[..end]
}

#[test]
fn hex_5_adapters_are_named_by_technology_and_concept() {
    let all = crates();
    let ports: Vec<String> = all
        .iter()
        .filter(|c| c.layer == Layer::Application)
        .flat_map(|app| {
            scan(
                &app.path("src/ports/outbound")
                    .into_iter()
                    .collect::<Vec<_>>(),
                Some("dtos"),
            )
        })
        .flat_map(|(_, items)| items.traits.into_iter().map(|t| t.ident.to_string()))
        .collect();
    let mut violations = Vec::new();
    let (mut production, mut doubles) = (0, 0);

    for krate in &all {
        for (path, imp, test) in trait_impls(krate) {
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
            let line = self_ty
                .path
                .segments
                .last()
                .map_or(0, |s| s.ident.span().start().line);
            let at = format!("{}:{line}", relative(&path));
            let prefix = name
                .strip_suffix("Adapter")
                .and_then(|stem| stem.strip_suffix(concept))
                .filter(|prefix| !prefix.is_empty());

            if test {
                doubles += 1;
                if !prefix.is_some_and(|p| DOUBLE_PREFIXES.contains(&p)) {
                    violations.push(format!(
                        "{at}: `{name}` es un doble de `{contract}` y debe llamarse \
                         {{{}}}{concept}Adapter (TDD-2)",
                        DOUBLE_PREFIXES.join("|")
                    ));
                }
                continue;
            }

            production += 1;
            match prefix {
                None => violations.push(format!(
                    "{at}: `{name}` implementa `{contract}` y debe llamarse \
                     {{Tecnología}}{concept}Adapter"
                )),
                Some(p) if DOUBLE_PREFIXES.contains(&first_word(p)) => {
                    violations.push(format!(
                        "{at}: `{name}` es de producción y usa el prefijo de doble `{}` \
                         (reservados a los tests: {DOUBLE_PREFIXES:?})",
                        first_word(p)
                    ));
                }
                Some(_) => {}
            }
            if krate.layer != Layer::Infrastructure {
                violations.push(format!(
                    "{at}: `{name}` es un adaptador de producción en `{}` ({:?}); los \
                     adaptadores viven en el anillo de infraestructura",
                    krate.name, krate.layer
                ));
            }
        }
    }

    assert_no_violations(
        "hex_5_adapters_are_named_by_technology_and_concept",
        "HEX-5 / TDD-2: un adaptador se llama {Tecnología}{Concepto}Adapter y vive en \
         infraestructura; un doble, {Fake|Fixed|Stub|Spy}{Concepto}Adapter. Concepto es el \
         puerto sin su sufijo de rol o, si es una capacidad del kernel (HEX-8), su nombre completo",
        &[
            ("adaptadores de producción", production),
            ("dobles de test", doubles),
        ],
        violations,
    );
}

#[test]
fn hex_6_outbound_dtos_do_not_use_reserved_vocabulary() {
    let mut violations = Vec::new();
    let mut dtos = 0;

    for (file, items) in scan(
        &dirs_of(Layer::Application, "src/ports/outbound/dtos"),
        None,
    ) {
        for name in &items.types {
            dtos += 1;
            if let Some(s) = RESERVED_SUFFIXES.iter().find(|s| name.ends_with(*s)) {
                violations.push(format!("{file}: `{name}` usa el sufijo reservado `{s}`"));
            }
        }
    }

    assert_no_violations(
        "hex_6_outbound_dtos_do_not_use_reserved_vocabulary",
        "HEX-6: DTOs de puerto saliente con `Request` o el nombre del concepto",
        &[("tipos de ports/outbound/dtos", dtos)],
        violations,
    );
}

#[test]
fn hex_6_transport_types_carry_the_transport_name() {
    let mut violations = Vec::new();
    let mut transport = 0;

    for (file, items) in scan(&dirs_of(Layer::Api, "src"), None) {
        for name in &items.types {
            let is_transport = name.ends_with("Request") || name.ends_with("Response");
            let named = name.ends_with("HttpRequest") || name.ends_with("HttpResponse");
            transport += usize::from(is_transport);
            if is_transport && !named {
                violations.push(format!(
                    "{file}: `{name}` debe ser {{Caso}}HttpRequest / {{Caso}}HttpResponse"
                ));
            }
        }
    }

    assert_no_violations(
        "hex_6_transport_types_carry_the_transport_name",
        "HEX-6: tipos de transporte con el nombre del transporte",
        &[("tipos *Request/*Response de api", transport)],
        violations,
    );
}

#[test]
fn hex_7_service_suffix_is_only_an_outbound_role() {
    let mut violations = Vec::new();
    let mut checked = 0;

    let core: Vec<(String, Items)> = scan(&dirs_of(Layer::Domain, "src"), None)
        .into_iter()
        .chain(
            scan(&dirs_of(Layer::Application, "src"), None)
                .into_iter()
                .filter(|(file, _)| {
                    !under(file, &dirs_of(Layer::Application, "src/ports/outbound"))
                }),
        )
        .collect();

    for (file, items) in core {
        let traits = items.traits.iter().map(|t| t.ident.to_string());
        for name in items.types.iter().cloned().chain(traits) {
            checked += 1;
            if name.ends_with("Service") {
                violations.push(format!(
                    "{file}: `{name}`: un servicio de dominio se nombra por la regla que \
                     evalúa (`PasswordPolicy`); un caso de uso, `{{Verb}}{{Noun}}UseCase` \
                     + `Handler` (HEX-4)"
                ));
            }
        }
    }

    assert_no_violations(
        "hex_7_service_suffix_is_only_an_outbound_role",
        "HEX-7: `Service` es solo un rol de puerto saliente (HEX-5, último recurso); \
         en domain y application ningún tipo se llama `…Service`",
        &[(
            "tipos de domain y application fuera de ports/outbound",
            checked,
        )],
        violations,
    );
}

fn is_workflow_type(name: &str) -> bool {
    (name.ends_with("Workflow") && name != "Workflow")
        || (name.ends_with("WorkflowError") && name != "WorkflowError")
}

#[test]
fn hex_4_workflows_are_internal_to_application() {
    let mut violations = Vec::new();

    // `api` depende de casos de uso: un workflow no es parte del contrato.
    let api_dirs = [dirs_of(Layer::Api, "src"), dirs_of(Layer::Api, "tests")].concat();
    let api_files = scan(&api_dirs, None);
    let api_scanned = api_files.len();
    for (file, items) in api_files {
        for ident in items.idents.iter().filter(|i| is_workflow_type(i)) {
            violations.push(format!(
                "{file}: referencia a `{ident}`: api ve casos de uso, no workflows"
            ));
        }
    }

    // Un `*Workflow` vive en `application/src/workflows/` y no es un caso de uso.
    let application = scan(&dirs_of(Layer::Application, "src"), None);
    let application_scanned = application.len();
    for (file, items) in application {
        let in_workflows = under(&file, &dirs_of(Layer::Application, "src/workflows"));
        if !in_workflows {
            for name in items.types.iter().filter(|n| is_workflow_type(n)) {
                violations.push(format!(
                    "{file}: `{name}` debe vivir en application/src/workflows/{{verb_noun}}/"
                ));
            }
            continue;
        }
        for imp in &items.impls {
            let Some((_, trait_path, _)) = &imp.trait_ else {
                continue;
            };
            let contract = last_segment(trait_path);
            if contract.ends_with("UseCase") {
                violations.push(format!(
                    "{file}: un workflow implementa `{contract}`: un caso de uso es un \
                     `{{Verb}}{{Noun}}Handler` en commands/ o queries/"
                ));
            }
        }
    }

    assert_no_violations(
        "hex_4_workflows_are_internal_to_application",
        "HEX-4: un `{Verb}{Noun}Workflow` es orquestación compartida de application: \
         vive en workflows/, no implementa un UseCase y api nunca lo nombra",
        &[
            ("archivos de api (src/ y tests/)", api_scanned),
            ("archivos de application", application_scanned),
        ],
        violations,
    );
}

#[test]
fn hex_4_event_subscribers_are_named_and_placed() {
    let mut violations = Vec::new();
    let mut implementations = 0;

    for krate in crates()
        .into_iter()
        .filter(|c| c.layer == Layer::Application)
    {
        for path in krate.files("src") {
            let file = relative(&path);
            let in_subscribers = is_under(&path, &krate, &["src/subscribers"]);
            for item in items_of(&parse(&path).items) {
                let syn::Item::Impl(imp) = item else {
                    continue;
                };
                let Some((_, trait_path, _)) = &imp.trait_ else {
                    continue;
                };
                let contract = last_segment(trait_path);
                let syn::Type::Path(self_ty) = &*imp.self_ty else {
                    continue;
                };
                let name = last_segment(&self_ty.path);
                if contract.ends_with("EventSubscriber") && contract != "EventSubscriber" {
                    implementations += 1;
                    if !name.ends_with("Subscriber") || name.ends_with("EventSubscriber") {
                        violations.push(format!(
                            "{file}: `{name}` implementa `{contract}` y debe llamarse \
                             {{Verb}}{{Noun}}Subscriber (la reacción, no el evento)"
                        ));
                    }
                    if !in_subscribers {
                        violations.push(format!(
                            "{file}: `{name}` debe vivir en application/src/subscribers/"
                        ));
                    }
                } else if in_subscribers && contract.ends_with("UseCase") {
                    violations.push(format!(
                        "{file}: `{name}` implementa `{contract}` en subscribers/: un caso \
                         de uso es un Handler en commands/ o queries/"
                    ));
                }
            }
        }
    }

    assert_no_violations(
        "hex_4_event_subscribers_are_named_and_placed",
        "HEX-4: la reacción a un evento es un `{Verb}{Noun}Subscriber` en \
         application/src/subscribers/ que implementa un `{Concepto}EventSubscriber`",
        &[(
            "implementaciones de un EventSubscriber en application (fuera de tests)",
            implementations,
        )],
        violations,
    );
}

#[test]
fn tdd_2_inbound_doubles_are_named_by_prefix_and_port() {
    let all = crates();
    let ports: Vec<String> = all
        .iter()
        .filter(|c| c.layer == Layer::Application)
        .flat_map(|app| {
            scan(
                &app.path("src/ports/inbound")
                    .into_iter()
                    .collect::<Vec<_>>(),
                None,
            )
        })
        .flat_map(|(_, items)| items.traits.into_iter().map(|t| t.ident.to_string()))
        .collect();
    let mut violations = Vec::new();
    let mut doubles = 0;

    for krate in &all {
        for (path, imp, test) in trait_impls(krate) {
            if !test {
                continue;
            }
            let Some((_, trait_path, _)) = &imp.trait_ else {
                continue;
            };
            let contract = last_segment(trait_path);
            if !ports.contains(&contract) {
                continue;
            }
            let syn::Type::Path(self_ty) = &*imp.self_ty else {
                continue;
            };
            doubles += 1;
            let name = last_segment(&self_ty.path);
            let prefix = name.strip_suffix(contract.as_str());
            if !prefix.is_some_and(|p| DOUBLE_PREFIXES.contains(&p)) {
                let line = self_ty
                    .path
                    .segments
                    .last()
                    .map_or(0, |s| s.ident.span().start().line);
                violations.push(format!(
                    "{}:{line}: `{name}` es un doble de `{contract}` y debe llamarse \
                     {{{}}}{contract} (TDD-2)",
                    relative(&path),
                    DOUBLE_PREFIXES.join("|")
                ));
            }
        }
    }

    assert_no_violations(
        "tdd_2_inbound_doubles_are_named_by_prefix_and_port",
        "TDD-2: un doble de un puerto entrante (UseCase, EventSubscriber) se llama \
         {Fake|Fixed|Stub|Spy}{Puerto}, sin `Adapter`: no es un adaptador",
        &[("dobles de puertos entrantes", doubles)],
        violations,
    );
}

/// legacy: en un contexto legacy, el dominio de application es `crate::domain`: HEX-4
/// lo revisa igual que a un crate `{ctx}-domain`.
#[test]
fn hex_4_contracts_are_checked_in_legacy_contexts() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/legacy");

    let (violations, contracts) = contract_violations(&common::crates_at(&fixture));

    assert_eq!(contracts, 2, "{violations:#?}");
    let found = |needle: &str| violations.iter().any(|v| v.contains(needle));
    assert!(
        found("`RegisterAccountResponse` expone `Account`"),
        "{violations:#?}"
    );
    assert!(found("expone `crate::domain::Account`"), "{violations:#?}");
    assert!(
        found("`RegisterAccountUseCase` expone `domain::Account`"),
        "{violations:#?}"
    );
    assert!(
        found("expone `super::super::super::super::domain::Account`"),
        "{violations:#?}"
    );
}
