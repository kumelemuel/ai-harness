//! Errores (ERR-1, RS-2).

mod common;

use common::{Layer, assert_no_violations, crates, parse, relative, rust_files};
use proc_macro2::{TokenStream, TokenTree};
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

#[test]
fn err_1_domain_errors_do_not_implement_std_error() {
    let mut violations = Vec::new();
    for krate in crates().into_iter().filter(|c| c.layer == Layer::Domain) {
        for file in rust_files(&krate.dir.join("src")) {
            let syntax = parse(&file);
            let mut impls = ErrorImpls::default();
            // Primero los alias: un `use … as` puede venir después del `impl`.
            for item in &syntax.items {
                if let syn::Item::Use(u) = item {
                    impls.visit_item_use(u);
                }
            }
            impls.visit_file(&syntax);
            for (line, target, origin) in impls.found {
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
        violations,
    );
}
