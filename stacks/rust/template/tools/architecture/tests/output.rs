//! El núcleo no escribe en la salida (ERR-4).
//!
//! HEX-2 impide los crates de logging en `domain` y `application`; este test
//! cubre lo que el lenguaje trae sin dependencias: las macros de `std` que
//! escriben en stdout o stderr. El árbol B se registra en el borde, a través
//! de `FaultReporter`.

mod common;

use common::{Layer, assert_no_violations, crates, parse, relative, rust_files};
use proc_macro2::{TokenStream, TokenTree};
use std::fs;
use std::path::{Path, PathBuf};
use syn::visit::Visit;

const OUTPUT_MACROS: &[&str] = &["println", "eprintln", "print", "eprint", "dbg"];

/// ¿El atributo es un `cfg` que exige `test`? `cfg(test)` y `cfg(all(…, test, …))`
/// sí; `cfg(any(test, feature = "testing"))` no, porque compila sin `test`.
fn requires_test(attr: &syn::Attribute) -> bool {
    fn predicate(meta: &syn::Meta) -> bool {
        match meta {
            syn::Meta::Path(p) => p.is_ident("test"),
            syn::Meta::List(l) if l.path.is_ident("all") => l
                .parse_args_with(
                    syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
                )
                .is_ok_and(|args| args.iter().any(predicate)),
            _ => false,
        }
    }
    attr.path().is_ident("cfg")
        && attr
            .parse_args::<syn::Meta>()
            .is_ok_and(|meta| predicate(&meta))
}

/// Rangos de líneas bajo `#[cfg(test)]` y archivos de módulos `#[cfg(test)] mod x;`.
struct TestOnly<'a> {
    file: &'a Path,
    lines: Vec<(usize, usize)>,
    files: Vec<PathBuf>,
}

impl TestOnly<'_> {
    fn mark(&mut self, attrs: &[syn::Attribute], span: proc_macro2::Span) {
        if attrs.iter().any(requires_test) {
            self.lines.push((span.start().line, span.end().line));
        }
    }

    /// Directorio donde viven los submódulos de este archivo.
    fn module_dir(&self) -> PathBuf {
        let dir = self
            .file
            .parent()
            .expect("invariante: archivo en un directorio");
        match self.file.file_stem().and_then(|s| s.to_str()) {
            Some("lib" | "main" | "mod") => dir.to_path_buf(),
            Some(stem) => dir.join(stem),
            None => dir.to_path_buf(),
        }
    }
}

impl<'ast> Visit<'ast> for TestOnly<'_> {
    fn visit_item(&mut self, node: &'ast syn::Item) {
        let attrs = match node {
            syn::Item::Const(i) => &i.attrs,
            syn::Item::Enum(i) => &i.attrs,
            syn::Item::Fn(i) => &i.attrs,
            syn::Item::Impl(i) => &i.attrs,
            syn::Item::Macro(i) => &i.attrs,
            syn::Item::Mod(i) => &i.attrs,
            syn::Item::Static(i) => &i.attrs,
            syn::Item::Struct(i) => &i.attrs,
            syn::Item::Trait(i) => &i.attrs,
            syn::Item::Use(i) => &i.attrs,
            _ => {
                syn::visit::visit_item(self, node);
                return;
            }
        };
        self.mark(attrs, syn::spanned::Spanned::span(node));
        if let syn::Item::Mod(m) = node
            && m.content.is_none()
            && attrs.iter().any(requires_test)
        {
            let dir = self.module_dir();
            let name = m.ident.to_string();
            self.files.push(dir.join(format!("{name}.rs")));
            self.files.push(dir.join(&name)); // `x/mod.rs` y los submódulos de `x.rs`
        }
        syn::visit::visit_item(self, node);
    }

    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        self.mark(&node.attrs, syn::spanned::Spanned::span(node));
        syn::visit::visit_impl_item_fn(self, node);
    }

    fn visit_trait_item_fn(&mut self, node: &'ast syn::TraitItemFn) {
        self.mark(&node.attrs, syn::spanned::Spanned::span(node));
        syn::visit::visit_trait_item_fn(self, node);
    }

    fn visit_stmt(&mut self, node: &'ast syn::Stmt) {
        if let syn::Stmt::Local(l) = node {
            self.mark(&l.attrs, syn::spanned::Spanned::span(node));
        } else if let syn::Stmt::Macro(m) = node {
            self.mark(&m.attrs, syn::spanned::Spanned::span(node));
        }
        syn::visit::visit_stmt(self, node);
    }
}

/// Invocaciones `nombre!` (también `std::println!`) en todos los tokens del
/// archivo, incluidos los argumentos de otras macros y los cuerpos de `macro_rules!`.
fn invocations(tokens: TokenStream, found: &mut Vec<(usize, String)>) {
    let tokens: Vec<TokenTree> = tokens.into_iter().collect();
    for (i, token) in tokens.iter().enumerate() {
        match token {
            TokenTree::Group(group) => invocations(group.stream(), found),
            TokenTree::Ident(ident)
                if OUTPUT_MACROS.contains(&ident.to_string().as_str())
                    && matches!(tokens.get(i + 1), Some(TokenTree::Punct(p)) if p.as_char() == '!') =>
            {
                found.push((ident.span().start().line, format!("{ident}!")));
            }
            _ => {}
        }
    }
}

#[test]
fn err_4_core_does_not_write_to_output() {
    let mut violations = Vec::new();
    for krate in crates()
        .into_iter()
        .filter(|c| matches!(c.layer, Layer::Domain | Layer::Application))
    {
        let files = rust_files(&krate.dir.join("src"));
        let mut test_files: Vec<PathBuf> = Vec::new();
        let mut per_file = Vec::new();
        for file in &files {
            let syntax = parse(file);
            let mut test_only = TestOnly {
                file,
                lines: Vec::new(),
                files: Vec::new(),
            };
            let whole_file = syntax.attrs.iter().any(requires_test); // `#![cfg(test)]`
            test_only.visit_file(&syntax);
            test_files.append(&mut test_only.files);
            per_file.push((file, whole_file, test_only.lines));
        }

        for (file, whole_file, lines) in per_file {
            if whole_file || test_files.iter().any(|t| file.starts_with(t)) {
                continue;
            }
            let text = fs::read_to_string(file).expect("invariante: archivo legible");
            let tokens: TokenStream = text.parse().expect("invariante: tokens válidos");
            let mut found = Vec::new();
            invocations(tokens, &mut found);
            for (line, name) in found {
                if !lines.iter().any(|(a, b)| (*a..=*b).contains(&line)) {
                    violations.push(format!(
                        "{}:{line}: `{name}` — el núcleo no escribe en la salida; devolver el \
                         error (árbol A) o propagarlo como `Fault` (árbol B)",
                        relative(file)
                    ));
                }
            }
        }
    }
    assert_no_violations(
        "ERR-4: `domain` y `application` no invocan `println!`, `eprintln!`, `print!`, \
         `eprint!` ni `dbg!` fuera de `#[cfg(test)]` (el registro ocurre en el borde)",
        violations,
    );
}
