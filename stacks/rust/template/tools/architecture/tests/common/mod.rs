#![allow(dead_code)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Marcadores de modelo del kernel (HEX-8): abiertos, sin vocabulario de un contexto.
/// La misma lista está en HEX-8 (`core/principles/hexagonal.md`). Sin marcadores de
/// error: HEX-8 los admite solo si el lenguaje los necesita, y en Rust el `enum`
/// del contrato ya cierra la jerarquía.
pub const MODEL_MARKERS: &[&str] = &[
    "ValueObject",
    "Entity",
    "AggregateRoot",
    "Identifier",
    "DomainEvent",
];

/// Capacidades técnicas transversales del kernel: lista cerrada de HEX-8.
/// Agregar una exige cambiar HEX-8 (`core/principles/hexagonal.md`) y este test
/// en el mismo commit.
pub const CAPABILITIES: &[&str] = &["Clock", "IdGenerator", "FaultReporter"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    Kernel,
    Domain,
    Application,
    Infrastructure,
    /// Adaptadores driving: `contexts/{ctx}/api` (por contexto) o `apps/api` (central).
    Api,
    /// Borde compartido por los adaptadores driving: `edge/*`.
    Edge,
    Bootstrap,
}

#[derive(Debug)]
pub struct Crate {
    pub name: String,
    pub layer: Layer,
    pub context: Option<String>,
    /// Directorio del `Cargo.toml`.
    pub dir: PathBuf,
    /// Raíz del código de la capa: `dir/src` en un crate por capa; el módulo de
    /// la capa en un contexto legacy (`contexts/iam/src/domain`).
    pub src: PathBuf,
    /// Tests de integración de la capa: `dir/tests`. En un contexto legacy no
    /// hay: los `tests/` del crate no pertenecen a una sola capa.
    pub tests: Option<PathBuf>,
    /// `true` si es una capa virtual de un contexto legacy.
    pub legacy: bool,
    pub manifest: toml::Table,
    /// `[workspace.dependencies]` de la raíz, para resolver `dep.workspace = true`.
    pub workspace: toml::Table,
}

impl Crate {
    /// Ruta escrita como en un crate por capa (`src/ports/inbound`, `tests`),
    /// resuelta contra la raíz real de la capa. `None` si la capa no tiene ese
    /// directorio (los `tests/` de un contexto legacy).
    pub fn path(&self, sub: &str) -> Option<PathBuf> {
        let (head, rest) = sub.split_once('/').unwrap_or((sub, ""));
        let base = match head {
            "src" => self.src.clone(),
            "tests" => self.tests.clone()?,
            _ => return Some(self.dir.join(sub)),
        };
        Some(if rest.is_empty() {
            base
        } else {
            base.join(rest)
        })
    }

    /// Archivos `.rs` bajo [`Crate::path`]; vacío si la capa no lo tiene.
    pub fn files(&self, sub: &str) -> Vec<PathBuf> {
        self.path(sub).map(|p| rust_files(&p)).unwrap_or_default()
    }

    /// Nombres de paquete de todas las dependencias declaradas (ver [`dependency_names`]).
    pub fn dependencies(&self) -> Vec<String> {
        dependency_names(&self.manifest, &self.workspace)
    }

    /// `(sección, paquete)` de cada dependencia, en el orden de [`Crate::dependencies`].
    pub fn sectioned_dependencies(&self) -> Vec<(String, String)> {
        declared_dependencies(&self.manifest)
            .into_iter()
            .map(|(section, _, _)| section)
            .zip(self.dependencies())
            .collect()
    }
}

/// Secciones de dependencias de un manifiesto.
pub const DEPENDENCY_SECTIONS: &[&str] =
    &["dependencies", "dev-dependencies", "build-dependencies"];

/// Cada dependencia declarada en `manifest`, también las de
/// `[target.'cfg(…)'.*]`: `(sección, clave, valor)`.
pub fn declared_dependencies(manifest: &toml::Table) -> Vec<(String, String, toml::Value)> {
    let mut tables: Vec<(String, &toml::Table)> = DEPENDENCY_SECTIONS
        .iter()
        .filter_map(|section| Some((section.to_string(), manifest.get(*section)?.as_table()?)))
        .collect();
    if let Some(targets) = manifest.get("target").and_then(|t| t.as_table()) {
        for (cfg, target) in targets {
            for section in DEPENDENCY_SECTIONS {
                if let Some(table) = target.get(*section).and_then(|v| v.as_table()) {
                    tables.push((format!("target.'{cfg}'.{section}"), table));
                }
            }
        }
    }
    tables
        .into_iter()
        .flat_map(|(section, table)| {
            table
                .iter()
                .map(move |(k, v)| (section.clone(), k.clone(), v.clone()))
        })
        .collect()
}

fn inherits(value: &toml::Value) -> bool {
    value.get("workspace").and_then(toml::Value::as_bool) == Some(true)
}

fn package_of(value: &toml::Value) -> Option<String> {
    value.get("package")?.as_str().map(str::to_string)
}

/// Nombres de paquete de las dependencias de `manifest`. Resuelve los
/// renombres (`foo = { package = "axum" }`), también cuando el renombre está en
/// `[workspace.dependencies]` y el miembro solo dice `foo.workspace = true`.
pub fn dependency_names(manifest: &toml::Table, workspace: &toml::Table) -> Vec<String> {
    declared_dependencies(manifest)
        .into_iter()
        .map(|(_, key, value)| {
            let inherited = inherits(&value)
                .then(|| workspace.get(&key).and_then(package_of))
                .flatten();
            package_of(&value).or(inherited).unwrap_or(key)
        })
        .collect()
}

/// Dependencias que no heredan del workspace (`workspace = true`): `(sección, clave)`.
pub fn non_inherited(manifest: &toml::Table) -> Vec<(String, String)> {
    declared_dependencies(manifest)
        .into_iter()
        .filter(|(_, _, value)| !inherits(value))
        .map(|(section, key, _)| (section, key))
        .collect()
}

pub fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("invariante: la raíz del workspace existe")
}

fn subdirs(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut dirs: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.join("Cargo.toml").exists() || p.is_dir())
        .collect();
    dirs.sort();
    dirs
}

/// `[workspace.dependencies]` del `Cargo.toml` de `root`.
pub fn workspace_dependencies_at(root: &Path) -> toml::Table {
    let text =
        fs::read_to_string(root.join("Cargo.toml")).expect("invariante: Cargo.toml raíz legible");
    let manifest: toml::Table = text.parse().expect("invariante: Cargo.toml raíz válido");
    manifest
        .get("workspace")
        .and_then(|w| w.get("dependencies"))
        .and_then(|d| d.as_table())
        .cloned()
        .unwrap_or_default()
}

fn load(dir: PathBuf, layer: Layer, context: Option<String>, workspace: &toml::Table) -> Crate {
    let text = fs::read_to_string(dir.join("Cargo.toml")).expect("invariante: Cargo.toml legible");
    let manifest: toml::Table = text.parse().expect("invariante: Cargo.toml válido");
    let name = manifest["package"]["name"]
        .as_str()
        .expect("invariante: package.name")
        .to_string();
    Crate {
        name,
        layer,
        context,
        src: dir.join("src"),
        tests: Some(dir.join("tests")),
        legacy: false,
        dir,
        manifest,
        workspace: workspace.clone(),
    }
}

/// Modo legacy (adopción). Directorio del kernel cuando no hay `kernel/`.
///
/// Todo lo marcado "legacy" en este archivo existe para adoptar el harness en
/// un proyecto con un crate por contexto. Cuando no quede ningún contexto
/// legacy y el kernel esté en `kernel/`, se borran `LEGACY_KERNEL_DIR`,
/// `LEGACY_MODULES`, `legacy_layers` y sus dos llamadas en `crates_at`.
pub const LEGACY_KERNEL_DIR: &str = "shared";

/// Módulos de capa de un crate legacy, en `src/`. `infrastructure/http` es el
/// adaptador driving: cuenta como Api, y sus archivos también como
/// Infrastructure porque viven dentro de ese módulo.
const LEGACY_MODULES: &[(&str, Layer)] = &[
    ("domain", Layer::Domain),
    ("application", Layer::Application),
    ("infrastructure", Layer::Infrastructure),
    ("infrastructure/http", Layer::Api),
];

/// Capas virtuales de un crate legacy (un crate por contexto, con las capas
/// como módulos). Todas comparten el manifiesto: lo que declara está
/// disponible en cada capa, que es justo lo que HEX-1 y HEX-2 tienen que ver.
/// No tienen `tests/`: los tests del crate no pertenecen a una sola capa.
fn legacy_layers(whole: &Crate) -> Vec<Crate> {
    LEGACY_MODULES
        .iter()
        .filter(|(module, _)| whole.src.join(module).is_dir())
        .map(|(module, layer)| Crate {
            name: whole.name.clone(),
            layer: *layer,
            context: whole.context.clone(),
            dir: whole.dir.clone(),
            src: whole.src.join(module),
            tests: None,
            legacy: true,
            manifest: whole.manifest.clone(),
            workspace: whole.workspace.clone(),
        })
        .collect()
}

/// Clasifica cada crate del workspace por su ubicación.
///
/// - `kernel/` → Kernel
/// - `edge/*` → Edge
/// - `infrastructure/*` → Infrastructure compartida, sin contexto: adaptadores de
///   las capacidades del kernel (HEX-8). Por HEX-1 solo ve el kernel.
/// - `contexts/{ctx}/{domain|application|infrastructure|api}/` → capa del contexto
/// - `apps/api/` → Api central · otros `apps/*` y `app/` → Bootstrap
///
/// Legacy (ver [`LEGACY_KERNEL_DIR`]): sin `kernel/`, el kernel es `shared/`, y un
/// contexto con `Cargo.toml` propio (`contexts/{ctx}/Cargo.toml`) se clasifica
/// por sus módulos de capa (ver `legacy_layers`). `tools/` no se clasifica.
pub fn crates() -> Vec<Crate> {
    crates_at(&root())
}

/// [`crates`] sobre un workspace cualquiera (los tests lo usan con fixtures).
pub fn crates_at(root: &Path) -> Vec<Crate> {
    let ws = workspace_dependencies_at(root);
    let load = |dir: PathBuf, layer: Layer, context: Option<String>| load(dir, layer, context, &ws);
    // Legacy: sin `kernel/`, el kernel es `LEGACY_KERNEL_DIR`.
    let kernel = if root.join("kernel/Cargo.toml").exists() {
        root.join("kernel")
    } else {
        root.join(LEGACY_KERNEL_DIR)
    };
    let mut crates = vec![load(kernel, Layer::Kernel, None)];

    for edge_dir in subdirs(&root.join("edge")) {
        if edge_dir.join("Cargo.toml").exists() {
            crates.push(load(edge_dir, Layer::Edge, None));
        }
    }

    for shared_dir in subdirs(&root.join("infrastructure")) {
        if shared_dir.join("Cargo.toml").exists() {
            crates.push(load(shared_dir, Layer::Infrastructure, None));
        }
    }

    for context_dir in subdirs(&root.join("contexts")) {
        let context = context_dir
            .file_name()
            .expect("invariante: directorio con nombre")
            .to_string_lossy()
            .to_string();
        if context_dir.join("Cargo.toml").exists() {
            let whole = load(context_dir.clone(), Layer::Bootstrap, Some(context));
            crates.extend(legacy_layers(&whole));
            continue;
        }
        for layer_dir in subdirs(&context_dir) {
            if !layer_dir.join("Cargo.toml").exists() {
                continue;
            }
            let layer = match layer_dir.file_name().and_then(|n| n.to_str()) {
                Some("domain") => Layer::Domain,
                Some("application") => Layer::Application,
                Some("infrastructure") => Layer::Infrastructure,
                Some("api") => Layer::Api,
                other => panic!("capa desconocida en contexts/{context}: {other:?}"),
            };
            crates.push(load(layer_dir, layer, Some(context.clone())));
        }
    }

    for app_dir in subdirs(&root.join("apps")) {
        if !app_dir.join("Cargo.toml").exists() {
            continue;
        }
        let layer = if app_dir.ends_with("api") {
            Layer::Api
        } else {
            Layer::Bootstrap
        };
        crates.push(load(app_dir, layer, None));
    }

    if root.join("app/Cargo.toml").exists() {
        crates.push(load(root.join("app"), Layer::Bootstrap, None));
    }

    crates
}

/// Directorios de los miembros que declara `[workspace]` en `root`: expande
/// los globs de `members` (solo `*` por segmento) y quita `exclude`. Incluye la
/// raíz si además es un paquete.
pub fn expand_members(root: &Path, workspace_manifest: &toml::Table) -> Vec<PathBuf> {
    fn strings(table: Option<&toml::Value>) -> Vec<String> {
        table
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default()
    }
    let workspace = workspace_manifest.get("workspace");
    let exclude: Vec<PathBuf> = strings(workspace.and_then(|w| w.get("exclude")))
        .iter()
        .map(|e| root.join(e))
        .collect();
    let mut found: Vec<PathBuf> = Vec::new();
    for pattern in strings(workspace.and_then(|w| w.get("members"))) {
        let mut dirs = vec![root.to_path_buf()];
        for segment in pattern.split('/') {
            dirs = dirs
                .into_iter()
                .flat_map(|dir| {
                    if segment == "*" {
                        subdirs(&dir).into_iter().filter(|d| d.is_dir()).collect()
                    } else {
                        vec![dir.join(segment)]
                    }
                })
                .collect();
        }
        found.extend(
            dirs.into_iter()
                .filter(|d| d.join("Cargo.toml").exists() && !exclude.contains(d)),
        );
    }
    if workspace_manifest.contains_key("package") {
        found.push(root.to_path_buf());
    }
    found.sort();
    found.dedup();
    found
}

/// Cada miembro del workspace con su manifiesto, según `[workspace] members`:
/// también los contextos legacy y los crates que [`crates`] no clasifica.
pub fn workspace_members() -> Vec<(PathBuf, toml::Table)> {
    let root = root();
    let text =
        fs::read_to_string(root.join("Cargo.toml")).expect("invariante: Cargo.toml raíz legible");
    let manifest: toml::Table = text.parse().expect("invariante: Cargo.toml raíz válido");
    expand_members(&root, &manifest)
        .into_iter()
        .map(|dir| {
            let text =
                fs::read_to_string(dir.join("Cargo.toml")).expect("invariante: Cargo.toml legible");
            let member = text.parse().expect("invariante: Cargo.toml válido");
            (dir, member)
        })
        .collect()
}

pub fn rust_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return files;
    };
    for path in entries.filter_map(Result::ok).map(|e| e.path()) {
        if path.is_dir() {
            files.extend(rust_files(&path));
        } else if path.extension().is_some_and(|e| e == "rs") {
            files.push(path);
        }
    }
    files.sort();
    files
}

pub fn parse(path: &Path) -> syn::File {
    let text = fs::read_to_string(path).expect("invariante: archivo legible");
    syn::parse_file(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

pub fn relative(path: &Path) -> String {
    path.strip_prefix(root())
        .unwrap_or(path)
        .display()
        .to_string()
}

/// Variable de entorno que convierte un conjunto vacío en un fallo.
///
/// Un test que no encuentra nada sobre qué actuar es indistinguible de uno que
/// verificó todo. En un proyecto recién creado puede ser legítimo que aún no
/// haya, por ejemplo, `queries/`, así que sin la variable un conjunto vacío
/// pasa. El CI del harness la activa sobre la plantilla, que debe ejercitar
/// cada regla.
pub const REQUIRE_MATERIAL: &str = "ARCH_REQUIRE_MATERIAL";

fn material_required() -> bool {
    std::env::var(REQUIRE_MATERIAL).is_ok_and(|v| v == "1")
}

/// Línea base del trinquete: `tools/architecture/baseline.txt`.
pub const BASELINE: &str = "baseline.txt";

/// Clave de una violación en la línea base: lo que va antes del primer `:`.
pub fn location(violation: &str) -> &str {
    violation.split(':').next().unwrap_or(violation).trim()
}

/// Rutas registradas para `id` en el texto de una línea base. Formato de cada
/// línea: `<test> <clave>`; `#` empieza un comentario de línea completa.
/// Entradas de un test en la línea base: clave → violaciones admitidas.
pub type Baseline = BTreeMap<String, usize>;

/// Clave de una violación de dependencias: manifiesto, sección y paquete.
pub fn dependency_key(dir: &Path, section: &str, dep: &str) -> String {
    format!("{}/Cargo.toml#{section}.{dep}", dir.display())
}

pub fn parse_baseline(text: &str, id: &str) -> Baseline {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| l.split_once(char::is_whitespace))
        .filter(|(test, _)| *test == id)
        .map(|(_, rest)| {
            let mut parts = rest.split_whitespace();
            let key = parts.next().unwrap_or_default().to_string();
            let count = parts.next().and_then(|n| n.parse().ok()).unwrap_or(1);
            (key, count)
        })
        .collect()
}

/// Informe del trinquete; vacío si cada violación está en `baseline` y cada
/// entrada de `baseline` sigue violando. Cada línea empieza por `  - `, que es
/// lo que el hook `PostToolUse` devuelve al agente.
pub fn ratchet_report(id: &str, baseline: &Baseline, violations: &[String]) -> String {
    let mut by_key: BTreeMap<&str, Vec<&String>> = BTreeMap::new();
    for v in violations {
        by_key.entry(location(v)).or_default().push(v);
    }

    let mut new = Vec::new();
    let mut over = Vec::new();
    for (key, found) in &by_key {
        match baseline.get(*key) {
            None => new.extend(found.iter().copied()),
            Some(&allowed) if found.len() > allowed => over.push((*key, found, allowed)),
            Some(_) => {}
        }
    }
    let mut fixed = Vec::new();
    let mut lower = Vec::new();
    for (key, &allowed) in baseline {
        match by_key.get(key.as_str()).map(Vec::len) {
            None => fixed.push(key),
            Some(n) if n < allowed => lower.push((key, n)),
            Some(_) => {}
        }
    }

    let mut lines = Vec::new();
    if !new.is_empty() || !over.is_empty() {
        lines.push(format!(
            "  - violaciones nuevas: corregirlas; {BASELINE} no se agranda"
        ));
    }
    for v in &new {
        lines.push(format!("  - {v}"));
    }
    for (key, found, allowed) in &over {
        lines.push(format!(
            "  - {key}: {} violaciones y {BASELINE} admite {allowed}",
            found.len()
        ));
        for v in found.iter() {
            lines.push(format!("  - {v}"));
        }
    }
    for key in &fixed {
        lines.push(format!("  - borrar de {BASELINE}: {id} {key}"));
    }
    for (key, n) in &lower {
        lines.push(format!("  - actualizar en {BASELINE}: {id} {key} {n}"));
    }
    if !new.is_empty() && !fixed.is_empty() {
        lines.push(format!(
            "  - ¿Se movió o renombró un archivo? {BASELINE} no sigue los movimientos: \
             corregir en el lugar, borrar la entrada y después mover."
        ));
    }
    lines.iter().map(|l| format!("{l}\n")).collect()
}

fn baseline_for(id: &str) -> Baseline {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(BASELINE);
    parse_baseline(&fs::read_to_string(path).unwrap_or_default(), id)
}

/// Cierra un test de arquitectura con trinquete sobre la línea base.
///
/// - `material`: cada conjunto que recorrió la regla, `(qué, cuántos)`. Se
///   informa siempre (visible con `--nocapture`). Con `ARCH_REQUIRE_MATERIAL=1`,
///   un conjunto vacío es un fallo que nombra la regla y lo que faltó.
/// - `id`: el nombre de la función del test, primera columna de `baseline.txt`.
/// - `violations`: lo que la regla encontró mal. Cada una empieza por su clave
///   (la ruta, o `ruta/Cargo.toml#dep`) seguida de `:`.
///
/// Trinquete: falla si aparece una violación cuya clave no está en la línea
/// base, o si una entrada de la línea base ya no viola. Así solo se achica.
pub fn assert_no_violations(
    id: &str,
    rule: &str,
    material: &[(&str, usize)],
    violations: Vec<String>,
) {
    let rule_id = rule.split(':').next().unwrap_or(rule);
    for (what, count) in material {
        eprintln!("{rule_id}: revisó {count} {what}");
    }
    let missing: Vec<&str> = material
        .iter()
        .filter(|(_, count)| *count == 0)
        .map(|(what, _)| *what)
        .collect();
    assert!(
        missing.is_empty() || !material_required(),
        "\n{rule}\nSin material ({REQUIRE_MATERIAL}=1): la regla no encontró sobre qué actuar.\n{}\n",
        missing
            .iter()
            .map(|what| format!("  - 0 {what}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
    let baseline = baseline_for(id);
    let tolerated: usize = baseline
        .iter()
        .map(|(key, allowed)| {
            violations
                .iter()
                .filter(|v| location(v) == key)
                .count()
                .min(*allowed)
        })
        .sum();
    if tolerated > 0 {
        eprintln!("{rule_id}: {tolerated} violaciones toleradas por {BASELINE}");
    }
    let report = ratchet_report(id, &baseline, &violations);
    assert!(report.is_empty(), "\n{rule}\n{report}");
}
