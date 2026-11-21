//! El dominio no declara puertos (HEX-3).
//!
//! Un nombre no distingue un puerto de una abstracción de dominio legítima (una
//! política, una especificación). La semántica sí: un puerto implica E/S, y en
//! este workspace la E/S se expresa con primitivas del kernel. `BoxFuture` es
//! asincronía, `Fault` es la falla de infraestructura (árbol B) y `Failure`
//! junta los dos árboles en la firma de un puerto. Un dominio que nombra
//! cualquiera de ellas está declarando una frontera técnica.

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
