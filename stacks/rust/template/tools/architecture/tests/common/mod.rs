#![allow(dead_code)]

use std::collections::BTreeSet;
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
    pub dir: PathBuf,
    pub manifest: toml::Table,
    /// `[workspace.dependencies]` de la raíz, para resolver `dep.workspace = true`.
    pub workspace: toml::Table,
}

impl Crate {
    /// Nombres de paquete de todas las dependencias declaradas (ver [`dependency_names`]).
    pub fn dependencies(&self) -> Vec<String> {
        dependency_names(&self.manifest, &self.workspace)
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

/// `[workspace.dependencies]` del `Cargo.toml` de la raíz.
pub fn workspace_dependencies() -> toml::Table {
    let text =
        fs::read_to_string(root().join("Cargo.toml")).expect("invariante: Cargo.toml raíz legible");
    let manifest: toml::Table = text.parse().expect("invariante: Cargo.toml raíz válido");
    manifest
        .get("workspace")
        .and_then(|w| w.get("dependencies"))
        .and_then(|d| d.as_table())
        .cloned()
        .unwrap_or_default()
}

fn load(dir: PathBuf, layer: Layer, context: Option<String>) -> Crate {
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
        dir,
        manifest,
        workspace: workspace_dependencies(),
    }
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
/// Un contexto con `Cargo.toml` propio (`contexts/{ctx}/Cargo.toml`) es *legacy*:
/// aún no está separado por capas y no se clasifica. `tools/` tampoco.
pub fn crates() -> Vec<Crate> {
    let root = root();
    let mut crates = vec![load(root.join("kernel"), Layer::Kernel, None)];

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
        if context_dir.join("Cargo.toml").exists() {
            continue; // legacy: un solo crate por contexto
        }
        let context = context_dir
            .file_name()
            .expect("invariante: directorio con nombre")
            .to_string_lossy()
            .to_string();
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
pub fn parse_baseline(text: &str, id: &str) -> BTreeSet<String> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| l.split_once(char::is_whitespace))
        .filter(|(test, _)| *test == id)
        .map(|(_, key)| key.trim().to_string())
        .collect()
}

/// Informe del trinquete; vacío si cada violación está en `baseline` y cada
/// entrada de `baseline` sigue violando. Cada línea empieza por `  - `, que es
/// lo que el hook `PostToolUse` devuelve al agente.
pub fn ratchet_report(id: &str, baseline: &BTreeSet<String>, violations: &[String]) -> String {
    let current: BTreeSet<&str> = violations.iter().map(|v| location(v)).collect();
    let new: Vec<&String> = violations
        .iter()
        .filter(|v| !baseline.contains(location(v)))
        .collect();
    let fixed: Vec<&String> = baseline
        .iter()
        .filter(|key| !current.contains(key.as_str()))
        .collect();

    let mut report = String::new();
    if !new.is_empty() {
        report.push_str(&format!(
            "\nViolaciones nuevas (corregirlas; {BASELINE} no se agranda):\n"
        ));
        for v in &new {
            report.push_str(&format!("  - {v}\n"));
        }
    }
    if !fixed.is_empty() {
        report.push_str("\nYa no violan:\n");
        for key in &fixed {
            report.push_str(&format!("  - borrar de {BASELINE}: {id} {key}\n"));
        }
    }
    if !new.is_empty() && !fixed.is_empty() {
        report.push_str(&format!(
            "  - ¿Se movió o renombró un archivo? {BASELINE} no sigue los movimientos: \
             corregir en el lugar, borrar la entrada y después mover.\n"
        ));
    }
    report
}

fn baseline_for(id: &str) -> BTreeSet<String> {
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
    let tolerated = violations
        .iter()
        .filter(|v| baseline.contains(location(v)))
        .count();
    if tolerated > 0 {
        eprintln!("{rule_id}: {tolerated} violaciones toleradas por {BASELINE}");
    }
    let report = ratchet_report(id, &baseline, &violations);
    assert!(report.is_empty(), "\n{rule}\n{report}");
}
