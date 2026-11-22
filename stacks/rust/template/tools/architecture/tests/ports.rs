//! El dominio no declara puertos (HEX-3).
//!
//! Un nombre no distingue un puerto de una abstracción de dominio legítima (una
//! política, una especificación). La semántica sí: un puerto implica E/S, y en
//! este workspace la E/S se expresa con primitivas del kernel. `BoxFuture` es
//! asincronía, `Fault` es la falla de infraestructura (árbol B) y `Failure`
//! junta los dos árboles en la firma de un puerto. Un dominio que nombra
//! cualquiera de ellas está declarando una frontera técnica.

//!
//! Dos tests, con detectores distintos:
//! - `hex_3_domain_does_not_use_kernel_io_primitives`: recorre los tokens
//!   (también los de macros) y resuelve los alias del kernel.
//! - `hex_3_domain_declares_no_asynchrony`: el dominio es síncrono (RS-3; la
//!   asincronía vive en los puertos). Recorre el AST y los cuerpos de macros, y
//!   cuenta `async`, `.await`, los módulos `std::future`/`std::task` y los
//!   nombres `Future` y `AsyncFn*`, también en `#[cfg(test)]`.

mod common;

use common::{Crate, Layer, assert_no_violations, crates, parse, relative};
use proc_macro2::{Spacing, TokenStream, TokenTree};
use std::collections::BTreeSet;
use std::fs;
use syn::visit::Visit;

/// Primitivas de E/S del kernel: solo tienen sentido en la firma de un puerto.
const IO_PRIMITIVES: &[&str] = &["BoxFuture", "Fault", "Failure"];

/// Lo que un archivo importa del kernel.
struct KernelImports {
    /// Nombre del crate kernel (`kernel`, o `shared` en legacy).
    kernel: String,
    /// Nombres con los que el archivo se refiere al crate (`kernel`, `use kernel as k`).
    crate_aliases: Vec<String>,
    /// `(nombre local, primitiva)` de cada primitiva importada, con o sin alias.
    names: Vec<(String, &'static str)>,
    /// Línea de cada `use kernel::*` (o de un módulo del kernel).
    globs: Vec<usize>,
}

impl KernelImports {
    /// `kernel`, o el nombre del crate legacy que hace de kernel.
    fn is_kernel(&self, name: &str) -> bool {
        name == "kernel" || name == self.kernel
    }

    fn new(kernel: &str) -> Self {
        Self {
            kernel: kernel.into(),
            crate_aliases: vec!["kernel".into(), kernel.into()],
            names: Vec::new(),
            globs: Vec::new(),
        }
    }
}

fn primitive(name: &str) -> Option<&'static str> {
    IO_PRIMITIVES.iter().copied().find(|p| *p == name)
}

impl KernelImports {
    fn collect(&mut self, tree: &syn::UseTree, prefix: &mut Vec<String>) {
        let from_kernel = prefix.first().is_some_and(|s| self.is_kernel(s));
        match tree {
            syn::UseTree::Path(p) => {
                prefix.push(p.ident.to_string());
                self.collect(&p.tree, prefix);
                prefix.pop();
            }
            syn::UseTree::Group(g) => {
                for item in &g.items {
                    self.collect(item, prefix);
                }
            }
            syn::UseTree::Name(n) if from_kernel => {
                if let Some(p) = primitive(&n.ident.to_string()) {
                    self.names.push((p.into(), p));
                }
            }
            syn::UseTree::Rename(r)
                if prefix.is_empty() && self.is_kernel(&r.ident.to_string()) =>
            {
                self.crate_aliases.push(r.rename.to_string());
            }
            syn::UseTree::Rename(r) if from_kernel => {
                if let Some(p) = primitive(&r.ident.to_string()) {
                    self.names.push((r.rename.to_string(), p));
                }
            }
            syn::UseTree::Glob(g) if from_kernel => {
                self.globs.push(g.star_token.spans[0].start().line);
            }
            _ => {}
        }
    }
}

impl<'ast> Visit<'ast> for KernelImports {
    fn visit_item_use(&mut self, node: &'ast syn::ItemUse) {
        // `use ::kernel::…` da el mismo árbol que `use kernel::…`.
        self.collect(&node.tree, &mut Vec::new());
    }
}

fn is_path_sep(tokens: &[TokenTree], at: usize) -> bool {
    matches!(
        (tokens.get(at), tokens.get(at + 1)),
        (Some(TokenTree::Punct(a)), Some(TokenTree::Punct(b)))
            if a.as_char() == ':' && a.spacing() == Spacing::Joint && b.as_char() == ':'
    )
}

/// Recorre los tokens del archivo (también los cuerpos de macros, que `syn` no
/// parsea como ítems) y registra cada mención de una primitiva del kernel:
/// por ruta (`kernel::Fault`, `k::future::BoxFuture`) o por nombre importado.
fn scan(tokens: TokenStream, imports: &KernelImports, found: &mut BTreeSet<(usize, String)>) {
    let tokens: Vec<TokenTree> = tokens.into_iter().collect();
    for (i, token) in tokens.iter().enumerate() {
        match token {
            TokenTree::Group(group) => scan(group.stream(), imports, found),
            TokenTree::Ident(ident) => {
                let name = ident.to_string();
                let after_path_sep = i >= 2 && is_path_sep(&tokens, i - 2);
                if !after_path_sep && imports.crate_aliases.contains(&name) {
                    // Ruta desde el crate: `kernel :: a :: b …`.
                    let mut path = name;
                    let mut at = i + 1;
                    while is_path_sep(&tokens, at) {
                        let Some(TokenTree::Ident(segment)) = tokens.get(at + 2) else {
                            break;
                        };
                        path = format!("{path}::{segment}");
                        if primitive(&segment.to_string()).is_some() {
                            found.insert((segment.span().start().line, path.clone()));
                        }
                        at += 3;
                    }
                } else if !after_path_sep
                    && let Some((_, p)) = imports.names.iter().find(|(local, _)| *local == name)
                {
                    let shown = if name == *p {
                        name.clone()
                    } else {
                        format!("{name} (alias de kernel::{p})")
                    };
                    found.insert((ident.span().start().line, shown));
                }
            }
            _ => {}
        }
    }
}

/// Violaciones de HEX-3 en los dominios de `all`, y cuántos archivos revisó.
fn io_violations(all: &[Crate]) -> (Vec<String>, usize) {
    let mut violations = Vec::new();
    let mut files = 0;
    let kernel = kernel_alias(all);
    for krate in all.iter().filter(|c| c.layer == Layer::Domain) {
        for file in krate.files("src") {
            files += 1;
            let mut imports = KernelImports::new(&kernel);
            imports.visit_file(&parse(&file));

            let text = fs::read_to_string(&file).expect("invariante: archivo legible");
            let tokens: TokenStream = text.parse().expect("invariante: tokens válidos");
            let mut found = BTreeSet::new();
            scan(tokens, &imports, &mut found);

            for (line, name) in found {
                violations.push(format!(
                    "{}:{line}: nombra `{name}` — una primitiva de E/S del kernel en el dominio \
                     es un puerto; declararlo en `application/ports`",
                    relative(&file)
                ));
            }
            for line in &imports.globs {
                violations.push(format!(
                    "{}:{line}: `use kernel::*` — no se puede verificar qué primitivas usa; \
                     importar por nombre",
                    relative(&file)
                ));
            }
        }
    }
    (violations, files)
}

/// Identificador con el que el código nombra el kernel: `kernel`, o (legacy:)
/// el crate que hace de kernel (`shared`).
fn kernel_alias(all: &[Crate]) -> String {
    all.iter()
        .find(|c| c.layer == Layer::Kernel)
        .map_or_else(|| "kernel".into(), |k| k.name.replace('-', "_"))
}

#[test]
fn hex_3_domain_does_not_use_kernel_io_primitives() {
    let (violations, files) = io_violations(&crates());
    assert_no_violations(
        "hex_3_domain_does_not_use_kernel_io_primitives",
        "HEX-3: el dominio no declara puertos (no usa `BoxFuture`, `Fault` ni `Failure` del \
         kernel: un puerto implica E/S)",
        &[("archivos de domain", files)],
        violations,
    );
}

/// legacy: con `shared/` como kernel, `shared::Fault` en el dominio es una
/// primitiva de E/S igual que `kernel::Fault`.
#[test]
fn hex_3_sees_the_legacy_kernel() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/legacy");

    let (violations, _) = io_violations(&common::crates_at(&fixture));

    assert!(
        violations.iter().any(|v| v.contains("nombra `Fault`")),
        "{violations:#?}"
    );
}

/// Nombres que solo pueden ser asincronía (salvo un tipo propio con ese nombre).
const ASYNC_NAMES: &[&str] = &[
    "Future",
    "IntoFuture",
    "AsyncFn",
    "AsyncFnMut",
    "AsyncFnOnce",
];

/// `std::future`, `core::task`…: los módulos de asincronía del lenguaje, si la
/// ruta pasa por uno.
fn async_module(segments: &[String]) -> Option<String> {
    segments.windows(2).find_map(|w| {
        (matches!(w[0].as_str(), "std" | "core" | "alloc")
            && matches!(w[1].as_str(), "future" | "task"))
        .then(|| format!("`{}::{}`", w[0], w[1]))
    })
}

/// Tipos que el dominio declara con un nombre de [`ASYNC_NAMES`] (un `Future`
/// de un dominio financiero): no son asincronía.
fn declared_async_names(file: &syn::File) -> BTreeSet<String> {
    #[derive(Default)]
    struct Declared(BTreeSet<String>);

    impl<'ast> Visit<'ast> for Declared {
        fn visit_item(&mut self, item: &'ast syn::Item) {
            let ident = match item {
                syn::Item::Struct(i) => Some(&i.ident),
                syn::Item::Enum(i) => Some(&i.ident),
                syn::Item::Type(i) => Some(&i.ident),
                syn::Item::Trait(i) => Some(&i.ident),
                syn::Item::Union(i) => Some(&i.ident),
                _ => None,
            };
            if let Some(ident) = ident.filter(|i| ASYNC_NAMES.iter().any(|n| *i == n)) {
                self.0.insert(ident.to_string());
            }
            syn::visit::visit_item(self, item);
        }
    }

    let mut declared = Declared::default();
    declared.visit_file(file);
    declared.0
}

/// Asincronía propia en un archivo: `(línea, qué)`. Cuenta:
/// - `async fn` (libre, de trait o de impl), bloques y closures `async`, `.await`;
/// - rutas y `use` que pasan por `std`/`core`/`alloc` `::future` o `::task`
///   (`Poll`, `Context`, `Waker`, y un alias `use std::future::Future as F`);
/// - los nombres de [`ASYNC_NAMES`] (`impl Future`, `AsyncFn`), salvo un tipo
///   propio del dominio con ese nombre (`own`);
/// - `async`, `await` y esos nombres dentro del cuerpo de una macro.
///
/// También en `#[cfg(test)]`: un dominio síncrono no necesita tests async.
fn asynchrony(file: &syn::File, own: &BTreeSet<String>) -> Vec<(usize, String)> {
    struct Found<'o> {
        own: &'o BTreeSet<String>,
        found: Vec<(usize, String)>,
    }

    impl Found<'_> {
        fn is_async_name(&self, ident: &str) -> bool {
            ASYNC_NAMES.contains(&ident) && !self.own.contains(ident)
        }

        fn scan_tokens(&mut self, tokens: TokenStream) {
            for token in tokens {
                match token {
                    TokenTree::Group(group) => self.scan_tokens(group.stream()),
                    TokenTree::Ident(ident) => {
                        let text = ident.to_string();
                        let what = match text.as_str() {
                            "async" => Some("`async` en una macro".to_string()),
                            "await" => Some("`.await` en una macro".to_string()),
                            name if self.is_async_name(name) => {
                                Some(format!("`{name}` en una macro"))
                            }
                            _ => None,
                        };
                        if let Some(what) = what {
                            self.found.push((ident.span().start().line, what));
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    impl<'ast> Visit<'ast> for Found<'_> {
        fn visit_signature(&mut self, sig: &'ast syn::Signature) {
            if let Some(token) = &sig.asyncness {
                self.found
                    .push((token.span.start().line, format!("`async fn {}`", sig.ident)));
            }
            syn::visit::visit_signature(self, sig);
        }

        fn visit_expr_async(&mut self, expr: &'ast syn::ExprAsync) {
            self.found
                .push((expr.async_token.span.start().line, "bloque `async`".into()));
            syn::visit::visit_expr_async(self, expr);
        }

        fn visit_expr_closure(&mut self, closure: &'ast syn::ExprClosure) {
            if let Some(token) = &closure.asyncness {
                self.found
                    .push((token.span.start().line, "closure `async`".into()));
            }
            syn::visit::visit_expr_closure(self, closure);
        }

        fn visit_expr_await(&mut self, expr: &'ast syn::ExprAwait) {
            self.found
                .push((expr.await_token.span.start().line, "`.await`".into()));
            syn::visit::visit_expr_await(self, expr);
        }

        fn visit_path(&mut self, path: &'ast syn::Path) {
            let segments: Vec<String> = path.segments.iter().map(|s| s.ident.to_string()).collect();
            let line = path
                .segments
                .first()
                .map_or(0, |s| s.ident.span().start().line);
            if let Some(module) = async_module(&segments) {
                self.found.push((line, module));
            } else if let Some(name) = segments.iter().find(|s| self.is_async_name(s)) {
                self.found.push((line, format!("`{name}`")));
            }
            syn::visit::visit_path(self, path);
        }

        fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
            fn leaves(tree: &syn::UseTree, prefix: &mut Vec<String>, out: &mut Vec<Vec<String>>) {
                match tree {
                    syn::UseTree::Path(p) => {
                        prefix.push(p.ident.to_string());
                        leaves(&p.tree, prefix, out);
                        prefix.pop();
                    }
                    syn::UseTree::Name(n) => {
                        out.push([prefix.clone(), vec![n.ident.to_string()]].concat())
                    }
                    syn::UseTree::Rename(r) => {
                        out.push([prefix.clone(), vec![r.ident.to_string()]].concat())
                    }
                    syn::UseTree::Glob(_) => out.push(prefix.clone()),
                    syn::UseTree::Group(g) => g.items.iter().for_each(|i| leaves(i, prefix, out)),
                }
            }
            let mut paths = Vec::new();
            leaves(&item.tree, &mut Vec::new(), &mut paths);
            let line = item.use_token.span.start().line;
            let mut reported = BTreeSet::new();
            for path in paths {
                if let Some(module) = async_module(&path)
                    && reported.insert(module.clone())
                {
                    self.found.push((line, module));
                }
            }
        }

        fn visit_macro(&mut self, mac: &'ast syn::Macro) {
            self.scan_tokens(mac.tokens.clone());
            syn::visit::visit_macro(self, mac);
        }
    }

    let mut found = Found {
        own,
        found: Vec::new(),
    };
    found.visit_file(file);
    found.found.sort();
    found.found
}

/// El dominio es síncrono (RS-3: la asincronía vive en los puertos de
/// `application`). Una función o un tipo async en el dominio declara una
/// frontera técnica propia, aunque no use las primitivas del kernel.
#[test]
fn hex_3_domain_declares_no_asynchrony() {
    let mut violations = Vec::new();
    let mut files = 0;
    for krate in crates().into_iter().filter(|c| c.layer == Layer::Domain) {
        // Un tipo propio llamado `Future` puede vivir en otro archivo del crate.
        let own: BTreeSet<String> = krate
            .files("src")
            .iter()
            .flat_map(|f| declared_async_names(&parse(f)))
            .collect();
        for file in krate.files("src") {
            files += 1;
            for (line, what) in asynchrony(&parse(&file), &own) {
                violations.push(format!(
                    "{}:{line}: {what} — el dominio no declara asincronía propia; \
                     la E/S va por un puerto en `application/ports`",
                    relative(&file)
                ));
            }
        }
    }
    assert_no_violations(
        "hex_3_domain_declares_no_asynchrony",
        "HEX-3: el dominio no declara asincronía propia (`async fn`, `async`, `.await`, `Future`)",
        &[("archivos de domain", files)],
        violations,
    );
}

mod own_asynchrony {
    use super::{asynchrony, declared_async_names};

    fn found(code: &str) -> Vec<String> {
        let file = syn::parse_file(code).expect("invariante: código de prueba válido");
        asynchrony(&file, &declared_async_names(&file))
            .into_iter()
            .map(|(_, what)| what)
            .collect()
    }

    #[test]
    fn an_async_fn_is_asynchrony() {
        assert_eq!(found("pub async fn load() {}"), ["`async fn load`"]);
    }

    #[test]
    fn an_async_method_in_a_trait_or_impl_is_asynchrony() {
        let code = "trait T { async fn a(&self); } struct S; impl S { async fn b(&self) {} }";
        assert_eq!(found(code), ["`async fn a`", "`async fn b`"]);
    }

    #[test]
    fn a_future_in_a_signature_is_asynchrony() {
        let code = "fn a() -> impl std::future::Future<Output = ()> { async {} }";
        assert_eq!(found(code), ["`std::future`", "bloque `async`"]);
    }

    #[test]
    fn a_bare_future_from_the_prelude_is_asynchrony() {
        assert_eq!(
            found("fn a() -> impl Future<Output = ()> { todo!() }"),
            ["`Future`"]
        );
    }

    #[test]
    fn await_is_asynchrony() {
        assert_eq!(found("fn a(x: X) { x.await; }"), ["`.await`"]);
    }

    #[test]
    fn an_async_closure_is_asynchrony() {
        assert_eq!(
            found("fn a() { let f = async move |x: u8| x; }"),
            ["closure `async`"]
        );
    }

    #[test]
    fn an_async_fn_bound_is_asynchrony() {
        assert_eq!(found("fn a(f: impl AsyncFn() -> u8) {}"), ["`AsyncFn`"]);
    }

    #[test]
    fn the_std_future_and_task_modules_are_asynchrony_also_in_a_use() {
        let code = "use std::future::Future as F;\nfn p(c: &mut core::task::Context) {}";
        assert_eq!(found(code), ["`std::future`", "`core::task`"]);
    }

    #[test]
    fn asynchrony_inside_a_macro_is_seen() {
        assert_eq!(found("m! { async fn x() {} }"), ["`async` en una macro"]);
    }

    #[test]
    fn a_domain_type_named_future_is_not_asynchrony() {
        let code =
            "pub struct Future { strike: u64 }\npub fn price(f: &Future) -> u64 { f.strike }";
        assert!(found(code).is_empty(), "{:?}", found(code));
    }

    /// Los tests del dominio también son síncronos: no necesitan runtime.
    #[test]
    fn asynchrony_in_test_modules_counts() {
        let code = "#[cfg(test)] mod tests { async fn t() {} }";
        assert_eq!(found(code), ["`async fn t`"]);
    }

    #[test]
    fn plain_domain_code_is_not() {
        let code =
            "pub struct Slug(String); impl Slug { pub fn new(s: &str) -> Self { Self(s.into()) } }";
        assert!(found(code).is_empty());
    }
}
