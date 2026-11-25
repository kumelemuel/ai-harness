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

/// `CreateCommunity` → `create_community`.
///
/// Agrupa las siglas: `VerifyHTTPToken` → `verify_http_token`. Corta antes de
/// una mayúscula que sigue a una minúscula, y antes de la última de una sigla
/// si la sigue una minúscula.
fn snake_case(name: &str) -> String {
    let chars: Vec<char> = name.chars().collect();
    let mut out = String::new();
    for (i, &c) in chars.iter().enumerate() {
        if c.is_uppercase() && i > 0 {
            let prev = chars[i - 1];
            let next_lower = chars.get(i + 1).is_some_and(|n| n.is_lowercase());
            if prev.is_lowercase() || (prev.is_uppercase() && next_lower) {
                out.push('_');
            }
        }
        out.extend(c.to_lowercase());
    }
    out
}

/// Violaciones de la firma de `execute` de un `{Verb}{Noun}UseCase`:
/// `(línea, mensaje)`. Un solo argumento además de `&self`,
/// `{Verb}{Noun}Command` o `{Verb}{Noun}Query`, y
/// `Result<{Verb}{Noun}Response, Failure<{Verb}{Noun}Error>>` (o `Fault`, si el
/// caso de uso no tiene árbol A), directo o dentro de un `BoxFuture`.
///
/// Límites: compara el último segmento de cada tipo, así que un alias de tipo
/// (`type CreateCommunityResult = Result<…>`) es una violación (se escribe la
/// firma explícita), y un homónimo definido en otro módulo pasa si existe el
/// del caso de uso.
fn use_case_signature_violations(t: &syn::ItemTrait) -> Vec<(usize, String)> {
    let name = t.ident.to_string();
    let Some(stem) = name.strip_suffix("UseCase").filter(|s| !s.is_empty()) else {
        return Vec::new();
    };
    let mut violations = Vec::new();
    for item in &t.items {
        let syn::TraitItem::Fn(f) = item else {
            continue;
        };
        if f.sig.ident != "execute" {
            continue;
        }
        let line = f.sig.ident.span().start().line;
        let args: Vec<&syn::PatType> = f
            .sig
            .inputs
            .iter()
            .filter_map(|a| match a {
                syn::FnArg::Typed(t) => Some(t),
                syn::FnArg::Receiver(_) => None,
            })
            .collect();
        let inputs = [format!("{stem}Command"), format!("{stem}Query")];
        match args.as_slice() {
            [arg] => {
                let ty = type_name(&arg.ty);
                if !inputs.contains(&ty) {
                    violations.push((
                        line,
                        format!(
                            "`{name}::execute` recibe `{ty}`: recibe `{}` o `{}`, por valor",
                            inputs[0], inputs[1]
                        ),
                    ));
                }
            }
            other => violations.push((
                line,
                format!(
                    "`{name}::execute` recibe {} argumentos además de `&self`: uno solo, `{}` o `{}`",
                    other.len(),
                    inputs[0],
                    inputs[1]
                ),
            )),
        }
        let response = format!("{stem}Response");
        let error = format!("{stem}Error");
        let syn::ReturnType::Type(_, output) = &f.sig.output else {
            violations.push((line, format!("`{name}::execute` no devuelve nada: devuelve `Result<{response}, Failure<{error}>>`")));
            continue;
        };
        let Some((ok, err)) = result_args(output) else {
            violations.push((line, format!("`{name}::execute` no devuelve un `Result`: devuelve `Result<{response}, Failure<{error}>>`")));
            continue;
        };
        let ok = type_name(ok);
        if ok != response {
            violations.push((
                line,
                format!("`{name}::execute` responde `{ok}`: responde `{response}`"),
            ));
        }
        let err_name = type_name(err);
        let failure_of = generic_arg(err).map(type_name);
        let valid = err_name == "Fault"
            || (err_name == "Failure" && failure_of.as_deref() == Some(error.as_str()));
        if !valid {
            let shown = failure_of.map_or(err_name.clone(), |inner| format!("{err_name}<{inner}>"));
            violations.push((
                line,
                format!(
                    "`{name}::execute` falla con `{shown}`: con `Failure<{error}>`, o con `Fault` si \
                     el caso de uso no tiene errores de negocio (ERR-1, ERR-2)"
                ),
            ));
        }
    }
    violations
}

/// Último segmento de un tipo de ruta (`a::b::C<…>` → `C`), con `&` si es una
/// referencia.
fn type_name(ty: &syn::Type) -> String {
    match ty {
        syn::Type::Path(p) => p
            .path
            .segments
            .last()
            .map(|s| s.ident.to_string())
            .unwrap_or_default(),
        syn::Type::Reference(r) => format!("&{}", type_name(&r.elem)),
        _ => "un tipo que no es una ruta".into(),
    }
}

/// Primer argumento genérico de tipo de `C<T, …>`.
fn generic_arg(ty: &syn::Type) -> Option<&syn::Type> {
    generic_args(ty).into_iter().next()
}

fn generic_args(ty: &syn::Type) -> Vec<&syn::Type> {
    let syn::Type::Path(p) = ty else {
        return Vec::new();
    };
    let Some(syn::PathArguments::AngleBracketed(args)) =
        p.path.segments.last().map(|s| &s.arguments)
    else {
        return Vec::new();
    };
    args.args
        .iter()
        .filter_map(|a| match a {
            syn::GenericArgument::Type(t) => Some(t),
            _ => None,
        })
        .collect()
}

/// `(T, E)` del primer `Result<T, E>`, directo o dentro de un genérico
/// (`BoxFuture<'_, Result<…>>`).
fn result_args(ty: &syn::Type) -> Option<(&syn::Type, &syn::Type)> {
    if type_name(ty) == "Result"
        && let [ok, err] = generic_args(ty).as_slice()
    {
        return Some((ok, err));
    }
    generic_args(ty).into_iter().find_map(result_args)
}

mod signatures {
    use super::*;

    fn violations(code: &str) -> Vec<String> {
        let t: syn::ItemTrait = syn::parse_str(code).expect("invariante: trait de prueba válido");
        use_case_signature_violations(&t)
            .into_iter()
            .map(|(_, v)| v)
            .collect()
    }

    #[test]
    fn the_canonical_signature_passes() {
        let found = violations(
            "pub trait CreateCommunityUseCase: Send + Sync {
                fn execute(&self, command: CreateCommunityCommand)
                    -> BoxFuture<'_, Result<CreateCommunityResponse, Failure<CreateCommunityError>>>;
            }",
        );

        assert_eq!(found, Vec::<String>::new());
    }

    #[test]
    fn a_query_and_a_synchronous_result_pass() {
        let found = violations(
            "pub trait VerifyAccessTokenUseCase {
                fn execute(&self, query: VerifyAccessTokenQuery)
                    -> Result<VerifyAccessTokenResponse, Failure<VerifyAccessTokenError>>;
            }",
        );

        assert_eq!(found, Vec::<String>::new());
    }

    /// Sin árbol A no hay tipo de error (ERR-1): `Result<…, Fault>`.
    #[test]
    fn a_use_case_without_business_errors_returns_fault() {
        let found = violations(
            "pub trait ListCommunitiesUseCase {
                fn execute(&self, query: ListCommunitiesQuery)
                    -> BoxFuture<'_, Result<ListCommunitiesResponse, Fault>>;
            }",
        );

        assert_eq!(found, Vec::<String>::new());
    }

    #[test]
    fn a_second_argument_is_a_violation() {
        let found = violations(
            "pub trait CreateCommunityUseCase {
                fn execute(&self, command: CreateCommunityCommand, identity: VerifiedIdentity)
                    -> BoxFuture<'_, Result<CreateCommunityResponse, Failure<CreateCommunityError>>>;
            }",
        );

        assert!(
            found.iter().any(|v| v.contains("2 argumentos")),
            "{found:?}"
        );
    }

    #[test]
    fn an_argument_of_another_type_is_a_violation() {
        let found = violations(
            "pub trait CreateCommunityUseCase {
                fn execute(&self, request: CreateCommunityRequest)
                    -> BoxFuture<'_, Result<CreateCommunityResponse, Failure<CreateCommunityError>>>;
            }",
        );

        assert!(
            found.iter().any(|v| v.contains("`CreateCommunityRequest`")),
            "{found:?}"
        );
    }

    #[test]
    fn a_shared_error_type_is_a_violation() {
        let found = violations(
            "pub trait RegisterAccountUseCase {
                fn execute(&self, command: RegisterAccountCommand)
                    -> Result<RegisterAccountResponse, SystemError>;
            }",
        );

        assert!(
            found.iter().any(|v| v.contains("`SystemError`")),
            "{found:?}"
        );
    }

    #[test]
    fn another_response_is_a_violation() {
        let found = violations(
            "pub trait CreateCommunityUseCase {
                fn execute(&self, command: CreateCommunityCommand)
                    -> Result<Community, Failure<CreateCommunityError>>;
            }",
        );

        assert!(found.iter().any(|v| v.contains("`Community`")), "{found:?}");
    }

    #[test]
    fn a_return_without_result_is_a_violation() {
        let found = violations(
            "pub trait CreateCommunityUseCase {
                fn execute(&self, command: CreateCommunityCommand) -> CreateCommunityResponse;
            }",
        );

        assert!(found.iter().any(|v| v.contains("Result")), "{found:?}");
    }
}

/// ¿Alguna firma de `t` nombra el tipo `name`?
fn uses_type(t: &syn::ItemTrait, name: &str) -> bool {
    struct Uses<'a> {
        name: &'a str,
        found: bool,
    }
    impl<'ast> Visit<'ast> for Uses<'_> {
        fn visit_path_segment(&mut self, segment: &'ast syn::PathSegment) {
            self.found |= segment.ident == self.name;
            syn::visit::visit_path_segment(self, segment);
        }
    }
    let mut uses = Uses { name, found: false };
    uses.visit_item_trait(t);
    uses.found
}

/// Tipos `struct`/`enum` definidos en un directorio de `krate` (sin módulos de test).
fn types_defined_under(dir: &Path) -> Vec<String> {
    rust_files(dir)
        .iter()
        .flat_map(|path| {
            let syntax = parse(path);
            items_of(&syntax.items)
                .into_iter()
                .filter_map(|item| match item {
                    syn::Item::Struct(s) => Some(s.ident.to_string()),
                    syn::Item::Enum(e) => Some(e.ident.to_string()),
                    _ => None,
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

/// HEX-4, ERR-2: la firma de un caso de uso es su contrato propio. `execute`
/// recibe un solo `{Verb}{Noun}Command` o `{Verb}{Noun}Query` y devuelve
/// `Result<{Verb}{Noun}Response, Failure<{Verb}{Noun}Error>>` (o `Fault` sin
/// árbol A). Esos tipos viven junto al handler, en
/// `commands/{verb_noun}/` o `queries/{verb_noun}/`: ni un segundo argumento
/// (la identidad va en el comando) ni un error compartido entre casos.
#[test]
fn hex_4_use_case_signatures_are_their_own_contract() {
    let (violations, use_cases) = signature_violations(&crates());

    assert_no_violations(
        "hex_4_use_case_signatures_are_their_own_contract",
        "HEX-4: `execute` recibe un solo `{Verb}{Noun}Command|Query` y devuelve \
         `Result<{Verb}{Noun}Response, Failure<{Verb}{Noun}Error>>` (o `Fault`), definidos \
         junto al handler",
        &[("UseCase de ports/inbound", use_cases)],
        violations,
    );
}

/// En legacy, `commands/` es `src/application/commands`.
#[test]
fn hex_4_use_case_signatures_are_checked_in_legacy_contexts() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/legacy");

    let (violations, use_cases) = signature_violations(&common::crates_at(&fixture));

    assert_eq!(use_cases, 1, "{violations:#?}");
    let found = |needle: &str| violations.iter().any(|v| v.contains(needle));
    assert!(found("recibe 0 argumentos"), "{violations:#?}");
    // El tipo existe, bajo src/application/commands/register/: el mensaje dice
    // dónde está y dónde se espera.
    assert!(
        found(
            "`RegisterAccountResponse` está en commands/register/: se espera en commands/register_account/"
        ),
        "{violations:#?}"
    );
}

#[test]
fn hex_4_snake_case_groups_acronyms() {
    assert_eq!(snake_case("CreateCommunity"), "create_community");
    assert_eq!(snake_case("VerifyHTTPToken"), "verify_http_token");
    assert_eq!(snake_case("ImportCSV"), "import_csv");
}

/// Tipos de los casos de uso de `app`: nombre → `commands/{caso}` o
/// `queries/{caso}`, para decir dónde está un tipo que no está en su lugar.
fn case_types(app: &Crate) -> std::collections::BTreeMap<String, String> {
    let mut found = std::collections::BTreeMap::new();
    for kind in ["commands", "queries"] {
        let Some(root) = app.path(&format!("src/{kind}")) else {
            continue;
        };
        let Ok(entries) = std::fs::read_dir(&root) else {
            continue;
        };
        for entry in entries.flatten().filter(|e| e.path().is_dir()) {
            let case = entry.file_name().to_string_lossy().to_string();
            for name in types_defined_under(&entry.path()) {
                found
                    .entry(name)
                    .or_insert_with(|| format!("{kind}/{case}"));
            }
        }
    }
    found
}

/// HEX-4: violaciones de las firmas de los `UseCase` de `all`, y cuántos revisó.
fn signature_violations(all: &[Crate]) -> (Vec<String>, usize) {
    let mut violations = Vec::new();
    let mut use_cases = 0;
    for app in all.iter().filter(|c| c.layer == Layer::Application) {
        let Some(inbound) = app.path("src/ports/inbound") else {
            continue;
        };
        for path in rust_files(&inbound) {
            let syntax = parse(&path);
            let file = relative(&path);
            for item in items_of(&syntax.items) {
                let syn::Item::Trait(t) = item else {
                    continue;
                };
                let name = t.ident.to_string();
                let Some(stem) = name.strip_suffix("UseCase").filter(|s| !s.is_empty()) else {
                    continue;
                };
                use_cases += 1;
                for (line, v) in use_case_signature_violations(t) {
                    violations.push(format!("{file}:{line}: {v}"));
                }
                let snake = snake_case(stem);
                let dirs: Vec<PathBuf> = ["src/commands", "src/queries"]
                    .iter()
                    .filter_map(|d| app.path(d))
                    .map(|d| d.join(&snake))
                    .collect();
                let defined: Vec<String> =
                    dirs.iter().flat_map(|d| types_defined_under(d)).collect();
                let elsewhere = case_types(app);
                let line = t.ident.span().start().line;
                let expected = format!("commands/{snake}/ o queries/{snake}/");
                let misplaced = |names: &[String]| -> Option<String> {
                    if names.iter().any(|n| defined.contains(n)) {
                        return None;
                    }
                    let shown = names
                        .iter()
                        .map(|n| format!("`{n}`"))
                        .collect::<Vec<_>>()
                        .join(" o ");
                    Some(
                        match names.iter().find_map(|n| elsewhere.get(n).map(|d| (n, d))) {
                            Some((n, dir)) => {
                                format!("`{n}` está en {dir}/: se espera en {expected}")
                            }
                            None => format!("{shown} no está definido en {expected}"),
                        },
                    )
                };
                let input = [format!("{stem}Command"), format!("{stem}Query")];
                let error = format!("{stem}Error");
                let response = [format!("{stem}Response")];
                for v in [misplaced(&input), misplaced(&response)]
                    .into_iter()
                    .flatten()
                {
                    violations.push(format!("{file}:{line}: {v}"));
                }
                if uses_type(t, &error) {
                    if let Some(v) = misplaced(std::slice::from_ref(&error)) {
                        violations.push(format!(
                            "{file}:{line}: {v} (un error por caso de uso, ERR-2)"
                        ));
                    }
                } else if defined.contains(&error) {
                    violations.push(format!(
                        "{file}:{line}: `{name}` falla solo con `Fault` y `{error}` sigue definido: \
                         sin árbol A no hay tipo de error (ERR-1)"
                    ));
                }
            }
        }
    }

    (violations, use_cases)
}

/// Nombres con los que un archivo de `application` alcanza el crate de dominio.
struct DomainNames {
    /// Ruta raíz del dominio: `["communities_domain"]`, o `["crate", "domain"]`
    /// en un contexto legacy.
    root: Vec<String>,
    /// El dominio del kernel, también dominio para los contratos:
    /// `["kernel", "domain"]`, o `["shared", "domain"]` con el kernel legacy.
    kernel_domain: Vec<String>,
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

    fn of(root: &[String], kernel: &str, module: Option<Vec<String>>, file: &syn::File) -> Self {
        let mut names = Self {
            root: root.to_vec(),
            kernel_domain: vec![kernel.to_string(), "domain".to_string()],
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
    let absolute = names.absolute(path);
    absolute.starts_with(&names.root)
        || absolute.starts_with(&names.kernel_domain)
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
    let kernel = all
        .iter()
        .find(|c| c.layer == Layer::Kernel)
        .map_or_else(|| "kernel".into(), |k| k.name.replace('-', "_"));

    for app in all.iter().filter(|c| c.layer == Layer::Application) {
        let Some(krate) = domain_crate_of(app, all) else {
            continue;
        };
        for path in app.files("src") {
            let syntax = parse(&path);
            let module = app.legacy.then(|| module_path(&app.dir.join("src"), &path));
            let names = DomainNames::of(&krate, &kernel, module, &syntax);
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

/// Alias de un archivo: `use kernel::IdGenerator as Ids;` → `Ids` → `IdGenerator`.
fn renames_of(syntax: &syn::File) -> std::collections::BTreeMap<String, String> {
    fn walk(tree: &syn::UseTree, out: &mut std::collections::BTreeMap<String, String>) {
        match tree {
            syn::UseTree::Path(p) => walk(&p.tree, out),
            syn::UseTree::Rename(r) => {
                out.insert(r.rename.to_string(), r.ident.to_string());
            }
            syn::UseTree::Group(g) => g.items.iter().for_each(|i| walk(i, out)),
            _ => {}
        }
    }
    let mut out = std::collections::BTreeMap::new();
    for item in &syntax.items {
        if let syn::Item::Use(u) = item {
            walk(&u.tree, &mut out);
        }
    }
    out
}

/// HEX-8: violaciones de ubicación de los adaptadores de capacidades en `all`,
/// y cuántas implementaciones de producción revisó.
fn capability_violations(all: &[Crate]) -> (Vec<String>, usize) {
    let mut violations = Vec::new();
    let mut implementations = 0;
    for krate in all {
        let mut renames = std::collections::BTreeMap::new();
        for (file, imp, test) in trait_impls(krate) {
            let Some((_, path, _)) = &imp.trait_ else {
                continue;
            };
            let renamed = renames
                .entry(file.clone())
                .or_insert_with(|| renames_of(&parse(&file)));
            let written = last_segment(path);
            let capability = renamed.get(&written).cloned().unwrap_or(written);
            if test || !CAPABILITIES.contains(&capability.as_str()) {
                continue;
            }
            implementations += 1;
            let shared = krate.layer == Layer::Infrastructure && krate.context.is_none();
            if !shared {
                violations.push(format!(
                    "{}: implementa `{capability}`, una capacidad del kernel, en `{}`: \
                     su adaptador va en un crate de infrastructure/*",
                    relative(&file),
                    krate.name
                ));
            }
        }
    }
    (violations, implementations)
}

/// HEX-8: el adaptador de una capacidad del kernel es infraestructura
/// compartida (`infrastructure/*`), no de un contexto: lo usan todos, y vive
/// una sola vez. Los dobles (`Fixed…`, `Spy…`) de los tests no cuentan.
#[test]
fn hex_8_capability_adapters_are_shared_infrastructure() {
    let (violations, implementations) = capability_violations(&crates());

    assert_no_violations(
        "hex_8_capability_adapters_are_shared_infrastructure",
        "HEX-8: los adaptadores de las capacidades del kernel viven en infrastructure/*, \
         fuera de los contextos",
        &[(
            "implementaciones de producción de capacidades del kernel",
            implementations,
        )],
        violations,
    );
}

/// En legacy, la capa virtual `infrastructure` es de un contexto, y un alias
/// del import no esconde la capacidad.
#[test]
fn hex_8_capability_adapters_are_checked_in_legacy_contexts() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/legacy");

    let (violations, implementations) = capability_violations(&common::crates_at(&fixture));

    assert_eq!(implementations, 1, "{violations:#?}");
    assert!(
        violations
            .iter()
            .any(|v| v.contains("implementa `IdGenerator`") && v.contains("en `iam`")),
        "{violations:#?}"
    );
}

#[test]
fn hex_8_renames_resolve_to_the_original_name() {
    let syntax: syn::File = syn::parse_str(
        "use kernel::{Clock, IdGenerator as Ids};\nuse shared::{FaultReporter as Faults};",
    )
    .expect("invariante: archivo de prueba válido");

    let renames = renames_of(&syntax);

    assert_eq!(renames.get("Ids").map(String::as_str), Some("IdGenerator"));
    assert_eq!(
        renames.get("Faults").map(String::as_str),
        Some("FaultReporter")
    );
    assert_eq!(renames.get("Clock"), None);
}

/// HEX-1: violaciones de los adaptadores driving (`api`, también la capa
/// virtual `infrastructure/http` de un contexto legacy) que nombran el dominio
/// o un puerto saliente, y cuántos archivos revisó.
fn driving_import_violations(all: &[Crate]) -> (Vec<String>, usize) {
    /// Rutas de un ítem (en `use` y en el código).
    #[derive(Default)]
    struct Paths(Vec<(usize, Vec<String>)>);
    impl<'ast> Visit<'ast> for Paths {
        fn visit_path(&mut self, path: &'ast syn::Path) {
            let segments = path.segments.iter().map(|s| s.ident.to_string()).collect();
            let line = path
                .segments
                .first()
                .map_or(0, |s| s.ident.span().start().line);
            self.0.push((line, segments));
            syn::visit::visit_path(self, path);
        }
        fn visit_item_use(&mut self, u: &'ast syn::ItemUse) {
            let line = u.use_token.span.start().line;
            for (leaf, _) in use_leaves(&u.tree) {
                self.0.push((line, leaf));
            }
        }
    }

    let is_outbound = |path: &[String]| path.windows(2).any(|w| w == ["ports", "outbound"]);
    let mut violations = Vec::new();
    let mut files = 0;
    for api in all.iter().filter(|c| c.layer == Layer::Api) {
        // Raíces del dominio y de application que ve este `api`: las de su
        // contexto, `crate::…` en legacy, o todas para un `api` central.
        let roots = |layer: Layer| -> Vec<Vec<String>> {
            if api.legacy {
                let name = if layer == Layer::Domain {
                    "domain"
                } else {
                    "application"
                };
                return vec![vec!["crate".into(), name.into()]];
            }
            all.iter()
                .filter(|c| c.layer == layer && !c.legacy)
                .filter(|c| api.context.is_none() || c.context == api.context)
                .map(|c| vec![c.name.replace('-', "_")])
                .collect()
        };
        let (domains, applications) = (roots(Layer::Domain), roots(Layer::Application));
        let mut test_files: Vec<PathBuf> = Vec::new();
        let mut parsed = Vec::new();
        for path in api.files("src") {
            let syntax = parse(&path);
            let dir = module_dir(&path);
            for item in &syntax.items {
                if let syn::Item::Mod(m) = item
                    && m.content.is_none()
                    && is_test_module(m)
                {
                    test_files.push(dir.join(format!("{}.rs", m.ident)));
                    test_files.push(dir.join(m.ident.to_string()));
                }
            }
            parsed.push((path, syntax));
        }
        for (path, syntax) in parsed {
            if test_files.iter().any(|t| path.starts_with(t)) {
                continue;
            }
            files += 1;
            let module = api.legacy.then(|| module_path(&api.dir.join("src"), &path));
            let resolver = DomainNames {
                root: Vec::new(),
                kernel_domain: Vec::new(),
                local: Vec::new(),
                glob: false,
                module,
            };
            // Nombres importados (`use x::ports;` → `ports` → `x::ports`) y
            // globs, para expandir las rutas que empiezan por ellos.
            let mut imported: std::collections::BTreeMap<String, Vec<String>> =
                std::collections::BTreeMap::new();
            let mut globs: Vec<Vec<String>> = Vec::new();
            let items: Vec<&syn::Item> = items_of(&syntax.items)
                .into_iter()
                .filter(|i| !matches!(i, syn::Item::Mod(m) if is_test_module(m)))
                .collect();
            for item in &items {
                if let syn::Item::Use(u) = item {
                    for (leaf, local) in use_leaves(&u.tree) {
                        let absolute = resolver.absolute(&leaf);
                        match local {
                            Some(name) => {
                                imported.insert(name, absolute);
                            }
                            None => globs.push(absolute),
                        }
                    }
                }
            }
            let mut paths = Paths::default();
            for item in &items {
                paths.visit_item(item);
            }
            let file = relative(&path);
            let mut seen = std::collections::BTreeSet::new();
            for (line, segments) in paths.0 {
                let shown = segments.join("::");
                let mut candidates = vec![resolver.absolute(&segments)];
                // Un nombre suelto importado ya lo reporta su `use`.
                if segments.len() > 1 {
                    if let Some(prefix) = imported.get(&segments[0]) {
                        candidates.push([prefix.clone(), segments[1..].to_vec()].concat());
                    }
                    for glob in &globs {
                        candidates.push([glob.clone(), segments.clone()].concat());
                    }
                }
                let under = |roots: &[Vec<String>], c: &Vec<String>| {
                    roots.iter().any(|r| !r.is_empty() && c.starts_with(r))
                };
                let message = if candidates.iter().any(|c| under(&domains, c)) {
                    format!(
                        "nombra el dominio (`{shown}`): un adaptador driving solo ve el UseCase"
                    )
                } else if candidates
                    .iter()
                    .any(|c| under(&applications, c) && is_outbound(c))
                {
                    format!(
                        "nombra un puerto saliente (`{shown}`): un adaptador driving llama al \
                         UseCase, nunca a los puertos de los que depende el handler"
                    )
                } else {
                    continue;
                };
                if seen.insert((line, shown)) {
                    violations.push(format!("{file}:{line}: {message}"));
                }
            }
        }
    }
    (violations, files)
}

/// HEX-6: violaciones de los tipos de `application` que aparecen en la firma
/// de un puerto saliente sin estar definidos en `ports/outbound/dtos/`, y
/// cuántos puertos revisó.
fn outbound_type_violations(all: &[Crate]) -> (Vec<String>, usize) {
    /// Rutas de una firma, completas.
    struct Paths(Vec<Vec<String>>);
    impl<'ast> Visit<'ast> for Paths {
        fn visit_path(&mut self, path: &'ast syn::Path) {
            self.0
                .push(path.segments.iter().map(|s| s.ident.to_string()).collect());
            syn::visit::visit_path(self, path);
        }
    }

    let mut violations = Vec::new();
    let mut ports = 0;
    for app in all.iter().filter(|c| c.layer == Layer::Application) {
        let Some(outbound) = app.path("src/ports/outbound") else {
            continue;
        };
        // `dtos/` o `dtos.rs`.
        let in_dtos =
            |f: &Path| f.starts_with(outbound.join("dtos")) || f == outbound.join("dtos.rs");
        // Los dobles, por su ruta dentro de `src` (no por la del proyecto).
        let is_double = |f: &Path| {
            f.strip_prefix(&app.src)
                .unwrap_or(f)
                .components()
                .any(|c| matches!(c.as_os_str().to_str(), Some("testing" | "testing.rs")))
        };
        // Raíz de application tal como la escribe un archivo del crate.
        let own_root: Vec<String> = if app.legacy {
            vec!["crate".into(), "application".into()]
        } else {
            vec!["crate".into()]
        };
        let external_name = app.name.replace('-', "_");
        let elsewhere: std::collections::BTreeSet<String> = app
            .files("src")
            .into_iter()
            .filter(|f| !in_dtos(f) && !is_double(f))
            .flat_map(|f| {
                let syntax = parse(&f);
                items_of(&syntax.items)
                    .into_iter()
                    .filter_map(|item| match item {
                        syn::Item::Struct(s) => Some(s.ident.to_string()),
                        syn::Item::Enum(e) => Some(e.ident.to_string()),
                        syn::Item::Type(t) => Some(t.ident.to_string()),
                        syn::Item::Union(u) => Some(u.ident.to_string()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
            })
            .collect();
        for path in rust_files(&outbound).into_iter().filter(|f| !in_dtos(f)) {
            let syntax = parse(&path);
            let file = relative(&path);
            let module = Some(module_path(&app.src, &path));
            let resolver = DomainNames {
                root: Vec::new(),
                kernel_domain: Vec::new(),
                local: Vec::new(),
                glob: false,
                module: module.map(|m| {
                    if app.legacy {
                        [vec!["application".to_string()], m].concat()
                    } else {
                        m
                    }
                }),
            };
            // ¿La ruta es de application? Sin `crate`/`super`/el crate propio
            // delante, es de otro crate (dominio, kernel, `std`).
            let is_application = |p: &[String]| {
                let absolute = resolver.absolute(p);
                absolute.starts_with(&own_root)
                    && !absolute.starts_with(&["crate".into(), "domain".into()])
                    || absolute.first() == Some(&external_name)
            };
            // Nombres sueltos importados de otro crate: no son de application.
            let mut foreign = std::collections::BTreeSet::new();
            for item in items_of(&syntax.items) {
                if let syn::Item::Use(u) = item {
                    for (leaf, local) in use_leaves(&u.tree) {
                        if let Some(name) = local
                            && !is_application(&leaf)
                        {
                            foreign.insert(name);
                        }
                    }
                }
            }
            for item in items_of(&syntax.items) {
                let syn::Item::Trait(t) = item else {
                    continue;
                };
                ports += 1;
                let mut found = std::collections::BTreeSet::new();
                for trait_item in &t.items {
                    let syn::TraitItem::Fn(f) = trait_item else {
                        continue;
                    };
                    let mut paths = Paths(Vec::new());
                    paths.visit_signature(&f.sig);
                    for p in paths.0 {
                        let Some(name) = p.last() else {
                            continue;
                        };
                        let ours = if p.len() > 1 {
                            is_application(&p)
                        } else {
                            !foreign.contains(name)
                        };
                        if ours && elsewhere.contains(name) {
                            found.insert((f.sig.ident.span().start().line, name.clone()));
                        }
                    }
                }
                for (line, name) in found {
                    violations.push(format!(
                        "{file}:{line}: `{name}`, en la firma de `{}`, está definido en \
                         application fuera de ports/outbound/dtos/: los tipos de un puerto \
                         saliente viven en dtos/",
                        t.ident
                    ));
                }
            }
        }
    }
    (violations, ports)
}

/// HEX-1: un adaptador driving llama al UseCase. No nombra el dominio (en
/// legacy comparte crate con él, así que Cargo no lo impide) ni los puertos
/// salientes, que son del handler.
#[test]
fn hex_1_driving_adapters_see_only_use_cases() {
    let (violations, files) = driving_import_violations(&crates());

    assert_no_violations(
        "hex_1_driving_adapters_see_only_use_cases",
        "HEX-1: un adaptador driving no nombra el dominio ni un puerto saliente",
        &[("archivos de api", files)],
        violations,
    );
}

/// HEX-6: los tipos propios de un puerto saliente (DTOs y errores declarados)
/// viven en `ports/outbound/dtos/`. El puerto puede usar el dominio, el kernel
/// y `std`.
#[test]
fn hex_6_outbound_port_types_live_in_dtos() {
    let (violations, ports) = outbound_type_violations(&crates());

    assert_no_violations(
        "hex_6_outbound_port_types_live_in_dtos",
        "HEX-6: los tipos de application en la firma de un puerto saliente están en \
         ports/outbound/dtos/",
        &[("puertos salientes", ports)],
        violations,
    );
}

#[test]
fn hex_1_driving_adapters_are_checked_in_legacy_contexts() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/legacy");

    let (violations, _) = driving_import_violations(&common::crates_at(&fixture));

    let shown: Vec<&str> = violations
        .iter()
        .map(|v| v.split(": ").nth(1).unwrap_or(v))
        .collect();
    assert_eq!(
        shown,
        [
            "nombra un puerto saliente (`crate::application::ports::outbound::PasswordHashingService`)",
            "nombra el dominio (`crate::domain::Account`)",
            "nombra un puerto saliente (`ports::outbound::PasswordHashingService`)",
            "nombra el dominio (`super::super::domain::Account`)",
        ],
        "{violations:#?}"
    );
}

#[test]
fn hex_6_outbound_port_types_are_checked_in_legacy_contexts() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/legacy");

    let (violations, ports) = outbound_type_violations(&common::crates_at(&fixture));

    assert_eq!(ports, 1, "{violations:#?}");
    assert_eq!(violations.len(), 2, "{violations:#?}");
    assert!(
        violations[0].contains("`HashedPassword`"),
        "{violations:#?}"
    );
    assert!(violations[1].contains("`Salt`"), "{violations:#?}");
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
    assert!(
        found("expone `shared::domain::VerifiedIdentity`"),
        "{violations:#?}"
    );
}
