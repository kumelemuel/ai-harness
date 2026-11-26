//! Errores (ERR-1, RS-2): ni el dominio ni los errores de `application` implementan
//! `std::error::Error`.

mod common;

use common::{Layer, assert_no_violations, crates, parse, relative, rust_files};
use proc_macro2::{TokenStream, TokenTree};
use std::collections::BTreeMap;
use std::path::Path;
use syn::spanned::Spanned;
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
        for file in krate.files("src") {
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
        "err_1_domain_errors_do_not_implement_std_error",
        "ERR-1: los errores de dominio no implementan std::error::Error \
         (si lo hicieran, `?` los convertiría en Fault sin que nadie lo decida)",
        &[("archivos de domain", files)],
        violations,
    );
}

const LEAF: &str = "hoja de errors.rs";
const CONTRACT: &str = "contrato {Verb}{Noun}Error";
const WORKFLOW: &str = "error de un workflow {Verb}{Noun}WorkflowError";
const PORT: &str = "error declarado de un puerto";

/// Errores del árbol A que declara un crate de `application`, con el motivo.
///
/// - Hojas: todo `struct`/`enum` de `errors.rs` (o `errors/`).
/// - Contratos: los `*Error` de `commands/` y `queries/` (`{Verb}{Noun}Error`).
/// - Workflows: los `*Error` de `workflows/` (`{Verb}{Noun}WorkflowError`). Sin
///   material propio: un contexto puede no tener workflows.
/// - Puertos: los `*Error` de `ports/` y todo `E` de un `Failure<E>` en `ports/outbound/`.
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
                "workflows" => Some(WORKFLOW),
                "ports" => {
                    // Solo los salientes: el `Failure<E>` de un puerto entrante es
                    // el contrato del caso de uso, que se clasifica en `commands/`
                    // o `queries/`. Si se registrara aquí, el orden de recorrido
                    // decidiría el motivo (`ports/` va antes que `queries/`).
                    if rel.starts_with("ports/outbound") {
                        FailureArgs(&mut declared.names).visit_file(&syntax);
                    }
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
        let src = krate.src.clone();
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
        "err_1_application_errors_do_not_implement_std_error",
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

/// Enums de application y del kernel: los contratos, sus hojas cerradas
/// (`Field`) y `Failure`.
fn contract_enums(all: &[common::Crate]) -> std::collections::BTreeSet<String> {
    #[derive(Default)]
    struct Enums(std::collections::BTreeSet<String>);
    impl<'ast> Visit<'ast> for Enums {
        fn visit_item_enum(&mut self, e: &'ast syn::ItemEnum) {
            self.0.insert(e.ident.to_string());
        }
    }
    let mut enums = Enums::default();
    for c in all
        .iter()
        .filter(|c| matches!(c.layer, Layer::Application | Layer::Kernel))
    {
        for path in c.files("src") {
            enums.visit_file(&parse(&path));
        }
    }
    enums.0
}

/// El enum del contrato que nombra la ruta de un patrón (`ChargeError::Declined`),
/// y la variante escrita (`ChargeError::Declined`) como paso de la cadena.
fn enum_of<'a>(
    path: &syn::Path,
    enums: &'a std::collections::BTreeSet<String>,
) -> Option<(&'a String, String)> {
    let segments: Vec<String> = path.segments.iter().map(|s| s.ident.to_string()).collect();
    let at = segments.len().checked_sub(2)?;
    let e = enums.get(&segments[at])?;
    Some((e, segments[at..].join("::")))
}

/// Lo que hay en una posición de un patrón. La posición es la cadena de pasos
/// desde la raíz: `["Err", "0", "Failure::Expected", "0"]`.
enum Reach<'a> {
    /// Un enum del contrato, nombrado por una variante.
    Enum(Vec<String>, &'a String),
    /// Un comodín (`_`, una variable) que cubre todo lo que cuelga de su
    /// posición, salvo los campos nombrados al lado de un `..` (`except`).
    CatchAll {
        chain: Vec<String>,
        except: Vec<String>,
        shown: String,
    },
}

fn reach<'a>(pat: &syn::Pat, enums: &'a std::collections::BTreeSet<String>) -> Vec<Reach<'a>> {
    fn walk<'a>(
        pat: &syn::Pat,
        enums: &'a std::collections::BTreeSet<String>,
        chain: &mut Vec<String>,
        out: &mut Vec<Reach<'a>>,
    ) {
        let catch_all = |name: &str, chain: &[String], except: Vec<String>| Reach::CatchAll {
            chain: chain.to_vec(),
            except,
            shown: shown(chain, name),
        };
        match pat {
            syn::Pat::Or(or) => or.cases.iter().for_each(|p| walk(p, enums, chain, out)),
            syn::Pat::Paren(p) => walk(&p.pat, enums, chain, out),
            syn::Pat::Reference(r) => walk(&r.pat, enums, chain, out),
            syn::Pat::Wild(_) => out.push(catch_all("_", chain, Vec::new())),
            syn::Pat::Ident(i) => match &i.subpat {
                Some((_, sub)) => walk(sub, enums, chain, out),
                None if i
                    .ident
                    .to_string()
                    .starts_with(|c: char| c.is_lowercase() || c == '_') =>
                {
                    out.push(catch_all(&i.ident.to_string(), chain, Vec::new()));
                }
                None => {}
            },
            syn::Pat::Path(p) => {
                if let Some((e, _)) = enum_of(&p.path, enums) {
                    out.push(Reach::Enum(chain.clone(), e));
                }
            }
            syn::Pat::TupleStruct(p) => {
                let step = match enum_of(&p.path, enums) {
                    Some((e, variant)) => {
                        out.push(Reach::Enum(chain.clone(), e));
                        variant
                    }
                    None => last_segment(&p.path),
                };
                for (i, elem) in p.elems.iter().enumerate() {
                    under(step.clone(), i.to_string(), elem, enums, chain, out);
                }
            }
            syn::Pat::Struct(p) => {
                let step = match enum_of(&p.path, enums) {
                    Some((e, variant)) => {
                        out.push(Reach::Enum(chain.clone(), e));
                        variant
                    }
                    None => last_segment(&p.path),
                };
                let mut named = Vec::new();
                for field in &p.fields {
                    let name = match &field.member {
                        syn::Member::Named(n) => n.to_string(),
                        syn::Member::Unnamed(i) => i.index.to_string(),
                    };
                    named.push(name.clone());
                    under(step.clone(), name, &field.pat, enums, chain, out);
                }
                if p.rest.is_some() {
                    chain.push(step);
                    out.push(catch_all("..", chain, named));
                    chain.pop();
                }
            }
            syn::Pat::Tuple(t) => {
                for (i, elem) in t.elems.iter().enumerate() {
                    under("()".to_string(), i.to_string(), elem, enums, chain, out);
                }
            }
            _ => {}
        }
    }
    /// `p` en la posición `chain` + `[step, index]`.
    fn under<'a>(
        step: String,
        index: String,
        p: &syn::Pat,
        enums: &'a std::collections::BTreeSet<String>,
        chain: &mut Vec<String>,
        out: &mut Vec<Reach<'a>>,
    ) {
        chain.push(step);
        chain.push(index);
        walk(p, enums, chain, out);
        chain.pop();
        chain.pop();
    }
    let mut out = Vec::new();
    walk(pat, enums, &mut Vec::new(), &mut out);
    out
}

fn last_segment(path: &syn::Path) -> String {
    path.segments
        .last()
        .map(|s| s.ident.to_string())
        .unwrap_or_default()
}

/// El comodín como se escribiría en su posición: `Err(_)`, `Failure::Expected(e)`.
fn shown(chain: &[String], name: &str) -> String {
    chain
        .chunks(2)
        .rev()
        .fold(name.to_string(), |inner, step| match step {
            [w, _] if w == "()" => format!("(…, {inner}, …)"),
            [w, _] => format!("{w}({inner})"),
            _ => inner,
        })
}

/// ¿El comodín cubre la posición de un enum? Su cadena es un prefijo de la del
/// enum, salvo que el enum cuelgue de un campo que el `..` dejó nombrado.
fn covers(chain: &[String], except: &[String], target: &[String]) -> bool {
    target.starts_with(chain)
        && target
            .get(chain.len())
            .is_none_or(|next| !except.contains(next))
}

/// ¿Un ítem es solo de test (`#[cfg(test)]`)?
fn is_test_only(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|a| {
        a.path().is_ident("cfg")
            && a.meta
                .require_list()
                .is_ok_and(|l| l.tokens.to_string() == "test")
    })
}

/// `matches!(expr, patrón)`: el patrón, si la macro es esa.
fn matches_pattern(m: &syn::Macro) -> Option<syn::Pat> {
    if !m.path.is_ident("matches") {
        return None;
    }
    m.parse_body_with(|input: syn::parse::ParseStream| {
        let _: syn::Expr = input.parse()?;
        let _: syn::Token![,] = input.parse()?;
        let pat = syn::Pat::parse_multi(input)?;
        let _: proc_macro2::TokenStream = input.parse()?;
        Ok(pat)
    })
    .ok()
}

/// Folds de los adaptadores driving con un comodín sobre un enum del contrato,
/// y cuántos folds sobre un enum del contrato revisó. Un fold es un `match`,
/// un `if let`, un `let … else` o un `matches!`: los tres últimos dejan sin
/// nombrar las variantes que no escriben.
fn catch_all_violations(all: &[common::Crate]) -> (Vec<String>, usize) {
    struct Folds<'a> {
        enums: &'a std::collections::BTreeSet<String>,
        found: Vec<String>,
        folds: usize,
    }
    impl Folds<'_> {
        /// Un fold de un solo patrón: si llega a un enum, el resto queda sin nombrar.
        fn partial(&mut self, kind: &str, pat: &syn::Pat) {
            let first = reach(pat, self.enums).into_iter().find_map(|r| match r {
                Reach::Enum(_, e) => Some(e.clone()),
                Reach::CatchAll { .. } => None,
            });
            if let Some(e) = first {
                self.folds += 1;
                self.found.push(format!(
                    "un `{kind}` sobre `{e}` (línea {}) deja sin nombrar las demás variantes: \
                     el fold del contrato es un `match` que nombra cada una (ERR-4)",
                    pat.span().start().line
                ));
            }
        }
    }
    impl<'ast> Visit<'ast> for Folds<'_> {
        fn visit_item_mod(&mut self, m: &'ast syn::ItemMod) {
            if !is_test_only(&m.attrs) {
                syn::visit::visit_item_mod(self, m);
            }
        }
        fn visit_item_fn(&mut self, f: &'ast syn::ItemFn) {
            if !is_test_only(&f.attrs) {
                syn::visit::visit_item_fn(self, f);
            }
        }
        fn visit_expr_let(&mut self, l: &'ast syn::ExprLet) {
            self.partial("if let", &l.pat);
            syn::visit::visit_expr_let(self, l);
        }
        fn visit_local(&mut self, l: &'ast syn::Local) {
            if l.init.as_ref().is_some_and(|i| i.diverge.is_some()) {
                self.partial("let … else", &l.pat);
            }
            syn::visit::visit_local(self, l);
        }
        fn visit_macro(&mut self, m: &'ast syn::Macro) {
            if let Some(pat) = matches_pattern(m) {
                self.partial("matches!", &pat);
            }
            syn::visit::visit_macro(self, m);
        }
        fn visit_expr_match(&mut self, m: &'ast syn::ExprMatch) {
            let arms: Vec<(usize, Vec<Reach>)> = m
                .arms
                .iter()
                .map(|a| (a.pat.span().start().line, reach(&a.pat, self.enums)))
                .collect();
            let reached: Vec<(&Vec<String>, &String)> = arms
                .iter()
                .flat_map(|(_, r)| r)
                .filter_map(|r| match r {
                    Reach::Enum(chain, e) => Some((chain, *e)),
                    Reach::CatchAll { .. } => None,
                })
                .collect();
            if !reached.is_empty() {
                self.folds += 1;
                for (line, reaches) in &arms {
                    for r in reaches {
                        let Reach::CatchAll {
                            chain,
                            except,
                            shown,
                        } = r
                        else {
                            continue;
                        };
                        if let Some((_, e)) = reached.iter().find(|(c, _)| covers(chain, except, c))
                        {
                            self.found.push(format!(
                                "el `match` sobre `{e}` tiene un brazo comodín (`{shown}`, línea \
                                 {line}): el fold del contrato nombra cada variante (ERR-4)"
                            ));
                        }
                    }
                }
            }
            syn::visit::visit_expr_match(self, m);
        }
    }

    let enums = contract_enums(all);
    let mut violations = Vec::new();
    let mut folds = 0;
    for api in all.iter().filter(|c| c.layer == Layer::Api) {
        for path in api.files("src") {
            let mut visitor = Folds {
                enums: &enums,
                found: Vec::new(),
                folds: 0,
            };
            visitor.visit_file(&parse(&path));
            folds += visitor.folds;
            let file = relative(&path);
            violations.extend(visitor.found.into_iter().map(|v| format!("{file}: {v}")));
        }
    }
    (violations, folds)
}

/// ERR-4: el adaptador driving traduce el contrato con un `match` que nombra
/// cada variante. Un comodín (`_`, una variable, `Err(_)`, `..`) en la posición
/// de un enum de application o del kernel, o un `if let`, `let … else` o
/// `matches!` sobre uno, manda una variante nueva a una respuesta inventada en
/// vez de no compilar. No mira el código de test.
#[test]
fn err_4_driving_folds_are_exhaustive() {
    let (violations, folds) = catch_all_violations(&crates());

    assert_no_violations(
        "err_4_driving_folds_are_exhaustive",
        "ERR-4: el fold del contrato en un adaptador driving nombra cada variante, sin comodín",
        &[("folds sobre un enum del contrato en api", folds)],
        violations,
    );
}

#[test]
fn err_4_driving_folds_are_checked_in_legacy_contexts() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/legacy");

    let (violations, folds) = catch_all_violations(&common::crates_at(&fixture));

    let shown: Vec<&str> = violations
        .iter()
        .map(|v| v.split_once(": ").map_or(v.as_str(), |(_, rest)| rest))
        .collect();
    let arm = |e: &str, wild: &str, line: usize| {
        format!(
            "el `match` sobre `{e}` tiene un brazo comodín (`{wild}`, línea {line}): \
             el fold del contrato nombra cada variante (ERR-4)"
        )
    };
    let partial = |kind: &str, e: &str, line: usize| {
        format!(
            "un `{kind}` sobre `{e}` (línea {line}) deja sin nombrar las demás variantes: \
             el fold del contrato es un `match` que nombra cada una (ERR-4)"
        )
    };
    // Una por cada `bad_*` del fixture; ninguna de los `ok_*` ni del código de test.
    let expected = [
        arm("ChargeError", "_", 10),
        arm("Field", "_other", 17),
        arm("ChargeError", "Err(e)", 25),
        arm("ChargeError", "Failure::Expected(_)", 35),
        arm("CardRejection", "Field::Card(_)", 44),
        arm("ChargeError", "_", 51),
        arm("Field", "..", 61),
        arm("Field", "(…, _, …)", 68),
        partial("if let", "ChargeError", 73),
        partial("let … else", "ChargeError", 77),
        partial("matches!", "ChargeError", 84),
    ];
    assert_eq!(shown, expected, "{violations:#?}");
    // 8 `match` y 3 folds parciales de los `bad_*`, y 3 `match` de los `ok_*`.
    assert_eq!(folds, 14, "{violations:#?}");
}
