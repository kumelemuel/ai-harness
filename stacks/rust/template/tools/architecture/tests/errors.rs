//! Errores (ERR-1, RS-2): ni el dominio ni los errores de `application` implementan
//! `std::error::Error`.

mod common;

use common::{Layer, assert_no_violations, crates, parse, relative, rust_files};
use proc_macro2::{TokenStream, TokenTree};
use std::collections::BTreeMap;
use std::path::Path;
use syn::visit::Visit;

/// Rutas con las que se nombra `std::error::Error` sin alias.
const ERROR_PATHS: &[&str] = &[
    "Error",
    "error::Error",
    "std::error::Error",
    "core::error::Error",
    "::std::error::Error",
    "::core::error::Error",
];

#[derive(Default)]
struct ErrorImpls {
    /// Alias locales de `std::error::Error` (`use std::error::Error as StdError`).
    aliases: Vec<String>,
    /// `(línea, tipo, origen)` de cada `impl Error for …` encontrado.
    found: Vec<(usize, String, &'static str)>,
}

impl ErrorImpls {
    fn is_error_trait(&self, path: &str) -> bool {
        ERROR_PATHS.contains(&path) || self.aliases.iter().any(|a| a == path)
    }
}

fn path_string(path: &syn::Path) -> String {
    let segments = path
        .segments
        .iter()
        .map(|s| s.ident.to_string())
        .collect::<Vec<_>>()
        .join("::");
    if path.leading_colon.is_some() {
        format!("::{segments}")
    } else {
        segments
    }
}

/// Recorre un `use` y registra los alias que apuntan a `…error::Error`.
fn collect_aliases(tree: &syn::UseTree, prefix: &mut Vec<String>, aliases: &mut Vec<String>) {
    match tree {
        syn::UseTree::Path(p) => {
            prefix.push(p.ident.to_string());
            collect_aliases(&p.tree, prefix, aliases);
            prefix.pop();
        }
        syn::UseTree::Rename(r)
            if r.ident == "Error" && prefix.last().is_some_and(|s| s == "error") =>
        {
            aliases.push(r.rename.to_string());
        }
        syn::UseTree::Group(g) => {
            for item in &g.items {
                collect_aliases(item, prefix, aliases);
            }
        }
        _ => {}
    }
}

/// Busca `impl [<…>] {Error} for {Tipo}` en el cuerpo de un `macro_rules!`.
/// El cuerpo tiene metavariables (`$name`), así que no se parsea como ítem:
/// se recorre como tokens.
fn scan_macro_body(tokens: TokenStream, impls: &mut ErrorImpls) {
    let tokens: Vec<TokenTree> = tokens.into_iter().collect();
    for (start, token) in tokens.iter().enumerate() {
        match token {
            TokenTree::Group(group) => scan_macro_body(group.stream(), impls),
            TokenTree::Ident(ident) if ident == "impl" => {
                let mut depth = 0i32;
                let mut trait_path = String::new();
                for (offset, next) in tokens[start + 1..].iter().enumerate() {
                    match next {
                        TokenTree::Punct(p) if p.as_char() == '<' => depth += 1,
                        TokenTree::Punct(p) if p.as_char() == '>' => depth -= 1,
                        TokenTree::Ident(i) if depth == 0 && i == "for" => {
                            if impls.is_error_trait(&trait_path) {
                                let target: String = tokens[start + offset + 2..]
                                    .iter()
                                    .take_while(|t| !matches!(t, TokenTree::Group(_)))
                                    .map(ToString::to_string)
                                    .collect();
                                impls.found.push((
                                    ident.span().start().line,
                                    target,
                                    "macro_rules!",
                                ));
                            }
                            break;
                        }
                        TokenTree::Group(_) => break,
                        other if depth == 0 => trait_path.push_str(&other.to_string()),
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
}

impl<'ast> Visit<'ast> for ErrorImpls {
    fn visit_item_use(&mut self, node: &'ast syn::ItemUse) {
        collect_aliases(&node.tree, &mut Vec::new(), &mut self.aliases);
    }

    fn visit_item_impl(&mut self, node: &'ast syn::ItemImpl) {
        if let Some((_, path, _)) = &node.trait_
            && self.is_error_trait(&path_string(path))
        {
            let target = match node.self_ty.as_ref() {
                syn::Type::Path(t) => path_string(&t.path),
                _ => "?".into(),
            };
            let line = path
                .segments
                .first()
                .map_or(0, |s| s.ident.span().start().line);
            self.found.push((line, target, "impl"));
        }
        syn::visit::visit_item_impl(self, node);
    }

    fn visit_item_macro(&mut self, node: &'ast syn::ItemMacro) {
        if node.mac.path.is_ident("macro_rules") {
            scan_macro_body(node.mac.tokens.clone(), self);
        }
        syn::visit::visit_item_macro(self, node);
    }
}

/// `impl Error for …` de un archivo: `(línea, tipo, origen)`.
fn error_impls(file: &Path) -> Vec<(usize, String, &'static str)> {
    let syntax = parse(file);
    let mut impls = ErrorImpls::default();
    // Primero los alias: un `use … as` puede venir después del `impl`.
    for item in &syntax.items {
        if let syn::Item::Use(u) = item {
            impls.visit_item_use(u);
        }
    }
    impls.visit_file(&syntax);
    impls.found
}

#[test]
fn err_1_domain_errors_do_not_implement_std_error() {
    let mut violations = Vec::new();
    let mut files = 0;
    for krate in crates().into_iter().filter(|c| c.layer == Layer::Domain) {
        for file in rust_files(&krate.dir.join("src")) {
            files += 1;
            for (line, target, origin) in error_impls(&file) {
                violations.push(format!(
                    "{}:{line}: `impl Error for {target}` ({origin}) — un error de dominio \
                     implementa solo Debug + Display",
                    relative(&file)
                ));
            }
        }
    }
    assert_no_violations(
        "ERR-1: los errores de dominio no implementan std::error::Error \
         (si lo hicieran, `?` los convertiría en Fault sin que nadie lo decida)",
        &[("archivos de domain", files)],
        violations,
    );
}

const LEAF: &str = "hoja de errors.rs";
const CONTRACT: &str = "contrato {Verb}{Noun}Error";
const PORT: &str = "error declarado de un puerto";

/// Errores del árbol A que declara un crate de `application`, con el motivo.
///
/// - Hojas: todo `struct`/`enum` de `errors.rs` (o `errors/`).
/// - Contratos: los `*Error` de `commands/` y `queries/` (`{Verb}{Noun}Error`).
/// - Puertos: los `*Error` de `ports/` y todo `E` de un `Failure<E>` en `ports/`.
#[derive(Default)]
struct DeclaredErrors {
    names: BTreeMap<String, &'static str>,
}

/// Recoge el `E` de cada `Failure<E>`.
struct FailureArgs<'a>(&'a mut BTreeMap<String, &'static str>);

impl<'ast> Visit<'ast> for FailureArgs<'_> {
    fn visit_path_segment(&mut self, node: &'ast syn::PathSegment) {
        if node.ident == "Failure"
            && let syn::PathArguments::AngleBracketed(args) = &node.arguments
            && let Some(syn::GenericArgument::Type(syn::Type::Path(t))) = args.args.first()
            && let Some(last) = t.path.segments.last()
        {
            self.0.entry(last.ident.to_string()).or_insert(PORT);
        }
        syn::visit::visit_path_segment(self, node);
    }
}

impl DeclaredErrors {
    fn of(src: &Path) -> Self {
        let mut declared = Self::default();
        for file in rust_files(src) {
            let rel = file.strip_prefix(src).expect("invariante: bajo src");
            let first = rel
                .components()
                .next()
                .and_then(|c| c.as_os_str().to_str())
                .unwrap_or_default()
                .trim_end_matches(".rs")
                .to_string();
            let syntax = parse(&file);
            let reason = match first.as_str() {
                "errors" => Some(LEAF),
                "commands" | "queries" => Some(CONTRACT),
                "ports" => {
                    FailureArgs(&mut declared.names).visit_file(&syntax);
                    Some(PORT)
                }
                _ => None,
            };
            let Some(reason) = reason else { continue };
            let mut types = TypeNames::default();
            types.visit_file(&syntax);
            for name in types.0 {
                if first == "errors" || name.ends_with("Error") {
                    declared.names.entry(name).or_insert(reason);
                }
            }
        }
        declared
    }

    fn count(&self, reason: &str) -> usize {
        self.names.values().filter(|r| **r == reason).count()
    }

    fn reason(&self, target: &str) -> Option<&'static str> {
        // Un `impl` generado por macro sobre `$tipo` no se puede resolver: cuenta.
        if target.starts_with('$') {
            return Some("tipo generado por macro, no verificable");
        }
        let last = target.rsplit("::").next().unwrap_or(target);
        self.names.get(last).copied()
    }
}

/// Nombres de los `struct` y `enum` de un archivo.
#[derive(Default)]
struct TypeNames(Vec<String>);

impl<'ast> Visit<'ast> for TypeNames {
    fn visit_item_struct(&mut self, node: &'ast syn::ItemStruct) {
        self.0.push(node.ident.to_string());
    }
    fn visit_item_enum(&mut self, node: &'ast syn::ItemEnum) {
        self.0.push(node.ident.to_string());
    }
}

#[test]
fn err_1_application_errors_do_not_implement_std_error() {
    let mut violations = Vec::new();
    let (mut leaves, mut contracts, mut ports) = (0, 0, 0);
    for krate in crates()
        .into_iter()
        .filter(|c| c.layer == Layer::Application)
    {
        let src = krate.dir.join("src");
        let declared = DeclaredErrors::of(&src);
        leaves += declared.count(LEAF);
        contracts += declared.count(CONTRACT);
        ports += declared.count(PORT);
        // El `impl` puede vivir en cualquier archivo del crate, no solo junto al tipo.
        for file in rust_files(&src) {
            for (line, target, origin) in error_impls(&file) {
                if let Some(reason) = declared.reason(&target) {
                    violations.push(format!(
                        "{}:{line}: `impl Error for {target}` ({origin}, {reason}) — un error \
                         del árbol A implementa solo Debug (y Display si hace falta)",
                        relative(&file)
                    ));
                }
            }
        }
    }
    assert_no_violations(
        "ERR-1: los errores de `application` (hojas, contratos y errores de puertos) no \
         implementan std::error::Error (`impl From<E: Error> for Fault` los absorbería en el \
         árbol B)",
        &[
            ("hojas de errors.rs", leaves),
            ("contratos {Verb}{Noun}Error", contracts),
            ("errores declarados de puertos", ports),
        ],
        violations,
    );
}
