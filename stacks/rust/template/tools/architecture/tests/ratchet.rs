//! Trinquete de la línea base (VER-1): `baseline.txt` solo se achica.

mod common;

use common::{
    BASELINE, Baseline, MATERIAL_EXEMPT, baseline_lines, exemption_problems, material_report,
    parse, parse_baseline, parse_exemptions, ratchet_report, root, rust_files, verdict,
};
use std::collections::BTreeSet;
use std::fs;
use syn::visit::Visit;

fn baseline(entries: &[(&str, usize)]) -> Baseline {
    entries.iter().map(|(k, n)| (k.to_string(), *n)).collect()
}

fn violations(lines: &[&str]) -> Vec<String> {
    lines.iter().map(|l| l.to_string()).collect()
}

#[test]
fn ver_1_baseline_reads_only_the_entries_of_its_test() {
    let text = "# comentario\n\nhex_1_x a/Cargo.toml#dependencies.b\nhex_2_y c/src/d.rs\nhex_1_x e/src/f.rs 3\n";

    let entries = parse_baseline(text, "hex_1_x");

    assert_eq!(
        entries,
        baseline(&[("a/Cargo.toml#dependencies.b", 1), ("e/src/f.rs", 3)])
    );
}

#[test]
fn ver_1_baseline_tolerates_the_listed_violations() {
    let report = ratchet_report(
        "hex_2_y",
        &baseline(&[("c/src/d.rs", 2)]),
        &violations(&["c/src/d.rs:3: `Foo` expone `Bar`", "c/src/d.rs:9: otra"]),
    );

    assert_eq!(report, "");
}

#[test]
fn ver_1_baseline_rejects_a_new_violation() {
    let report = ratchet_report(
        "hex_2_y",
        &baseline(&[("c/src/d.rs", 1)]),
        &violations(&["c/src/d.rs:3: vieja", "g/src/h.rs:1: nueva"]),
    );

    assert!(report.contains("  - g/src/h.rs:1: nueva"), "{report}");
    assert!(!report.contains("vieja"), "{report}");
}

/// Una entrada por archivo no tolera una violación más en ese archivo.
#[test]
fn ver_1_baseline_rejects_one_more_violation_in_a_listed_file() {
    let report = ratchet_report(
        "err_4_x",
        &baseline(&[("c/src/d.rs", 1)]),
        &violations(&["c/src/d.rs:3: `println!`", "c/src/d.rs:9: `println!`"]),
    );

    assert!(report.contains("c/src/d.rs: 2 violaciones"), "{report}");
    assert!(report.contains("admite 1"), "{report}");
}

#[test]
fn ver_1_baseline_asks_to_delete_what_no_longer_violates() {
    let report = ratchet_report("hex_2_y", &baseline(&[("c/src/d.rs", 1)]), &[]);

    assert!(
        report.contains("  - borrar de baseline.txt: hex_2_y c/src/d.rs"),
        "{report}"
    );
}

#[test]
fn ver_1_baseline_asks_to_lower_the_count_when_one_is_fixed() {
    let report = ratchet_report(
        "err_4_x",
        &baseline(&[("c/src/d.rs", 3)]),
        &violations(&["c/src/d.rs:3: a", "c/src/d.rs:9: b"]),
    );

    assert!(
        report.contains("  - actualizar en baseline.txt: err_4_x c/src/d.rs 2"),
        "{report}"
    );
}

/// Una entrada por dependencia y por sección: tolerar `tokio` en
/// `[dev-dependencies]` no tolera `chrono`, ni `tokio` en `[dependencies]`.
#[test]
fn ver_1_baseline_keys_dependencies_one_by_one() {
    let report = ratchet_report(
        "hex_2_core",
        &baseline(&[("contexts/x/Cargo.toml#dev-dependencies.tokio", 1)]),
        &violations(&[
            "contexts/x/Cargo.toml#dev-dependencies.tokio: x usa `tokio`",
            "contexts/x/Cargo.toml#dependencies.tokio: x usa `tokio`",
            "contexts/x/Cargo.toml#dependencies.chrono: x usa `chrono`",
        ]),
    );

    assert!(report.contains("#dependencies.tokio:"), "{report}");
    assert!(report.contains("#dependencies.chrono:"), "{report}");
    assert!(!report.contains("borrar"), "{report}");
}

/// Mover un archivo con violaciones da a la vez una nueva y una para borrar.
/// El método es corregir en el lugar y después mover.
#[test]
fn ver_1_baseline_hints_at_a_move() {
    let report = ratchet_report(
        "hex_2_y",
        &baseline(&[("c/src/old.rs", 1)]),
        &violations(&["c/src/new.rs:3: la misma"]),
    );

    assert!(report.contains("corregir en el lugar"), "{report}");
}

/// Cada línea del informe llega al agente: el hook `PostToolUse` solo
/// reenvía las que empiezan por `  - ` o por un ID.
#[test]
fn ver_1_baseline_report_lines_reach_the_hook() {
    let report = ratchet_report(
        "hex_2_y",
        &baseline(&[("c/src/old.rs", 1)]),
        &violations(&["c/src/new.rs:3: la misma"]),
    );

    for line in report.lines().filter(|l| !l.is_empty()) {
        assert!(line.starts_with("  - "), "el hook descarta: {line:?}");
    }
}

/// Llamadas a `assert_no_violations` y nombres de tests de un archivo.
#[derive(Default)]
struct Calls {
    current: Option<String>,
    tests: BTreeSet<String>,
    /// Funciones que llaman a `assert_no_violations`: las únicas que leen
    /// `material-exempt.txt`.
    asserting: BTreeSet<String>,
    mismatches: Vec<String>,
}

impl<'ast> Visit<'ast> for Calls {
    fn visit_item_fn(&mut self, f: &'ast syn::ItemFn) {
        let previous = self.current.replace(f.sig.ident.to_string());
        if f.attrs.iter().any(|a| a.path().is_ident("test")) {
            self.tests.insert(f.sig.ident.to_string());
        }
        syn::visit::visit_item_fn(self, f);
        self.current = previous;
    }

    fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
        let is_assert =
            matches!(&*call.func, syn::Expr::Path(p) if p.path.is_ident("assert_no_violations"));
        if is_assert {
            self.asserting.extend(self.current.clone());
            let id = match call.args.first() {
                Some(syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(s),
                    ..
                })) => Some(s.value()),
                _ => None,
            };
            if id != self.current {
                self.mismatches.push(format!(
                    "`{}` llama a assert_no_violations con id {id:?}",
                    self.current.as_deref().unwrap_or("?")
                ));
            }
        }
        syn::visit::visit_expr_call(self, call);
    }
}

/// El `id` del trinquete es el nombre de la función del test, y cada entrada
/// de la línea base nombra un test que existe. Si no, dos tests leen las
/// mismas entradas, o una entrada huérfana nunca se pide borrar.
#[test]
fn ver_1_baseline_ids_match_tests() {
    let dir = root().join("tools/architecture/tests");
    let mut calls = Calls::default();
    let mut files = 0;
    for path in rust_files(&dir)
        .into_iter()
        .filter(|p| !p.starts_with(dir.join("fixtures")))
    {
        files += 1;
        calls.visit_file(&parse(&path));
    }

    let mut problems = calls.mismatches.clone();
    let text =
        fs::read_to_string(root().join("tools/architecture").join(BASELINE)).unwrap_or_default();
    for (number, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let test = line.split_whitespace().next().unwrap_or_default();
        if !calls.tests.contains(test) {
            problems.push(format!(
                "{BASELINE}:{}: `{test}` no es un test de tools/architecture",
                number + 1
            ));
        }
    }

    assert!(files > 0, "sin archivos de test en {}", dir.display());
    assert!(
        problems.is_empty(),
        "\nVER-1: ids del trinquete\n{}\n",
        problems
            .iter()
            .map(|p| format!("  - {p}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// Exenciones de material (VER-1): `<test> | <conjunto> | <motivo>`.
#[test]
fn ver_1_material_exemptions_read_only_their_test() {
    let text = "# comentario\n\
                err_1_x | errores declarados de puertos | los puertos devuelven Fault\n\
                hex_4_y | implementaciones | sin eventos (D4)\n";

    let exempt = parse_exemptions(text, "err_1_x");

    assert_eq!(
        exempt.into_iter().collect::<Vec<_>>(),
        [(
            "errores declarados de puertos".to_string(),
            "los puertos devuelven Fault".to_string()
        )]
    );
}

/// Un conjunto vacío y exento no falla aunque se exija material.
#[test]
fn ver_1_material_exempt_empty_set_passes() {
    let exempt = parse_exemptions("err_1_x | puertos | sin errores declarados", "err_1_x");

    let report = material_report(&[("hojas", 3), ("puertos", 0)], &exempt, true);

    assert_eq!(report, "");
}

/// Sin exención, el conjunto vacío sigue fallando con la variable.
#[test]
fn ver_1_material_empty_set_without_exemption_fails() {
    let report = material_report(&[("hojas", 3), ("puertos", 0)], &Default::default(), true);

    assert!(report.contains("  - 0 puertos"), "{report}");
    assert!(!report.contains("hojas"), "{report}");
}

/// Una exención que ya no hace falta se borra, como una entrada de la línea
/// base: falla aunque no se exija material.
#[test]
fn ver_1_material_exemption_of_a_non_empty_set_asks_to_delete_it() {
    let exempt = parse_exemptions("err_1_x | puertos | sin errores declarados", "err_1_x");

    let report = material_report(&[("puertos", 2)], &exempt, false);

    assert!(
        report.contains("  - borrar de material-exempt.txt: err_1_x | puertos"),
        "{report}"
    );
}

/// Una exención de un conjunto que el test no recorre no exime nada.
#[test]
fn ver_1_material_exemption_of_an_unknown_set_fails() {
    let exempt = parse_exemptions("err_1_x | puerto | errata", "err_1_x");

    let report = material_report(&[("puertos", 0)], &exempt, false);

    assert!(
        report.contains("`puerto` no es un conjunto de este test"),
        "{report}"
    );
}

/// Formato: tres campos no vacíos, un test existente, sin repetidos.
#[test]
fn ver_1_material_exemption_problems_name_the_line() {
    let tests = ["err_1_x".to_string()].into_iter().collect();
    let text = "err_1_x | puertos\n\
                err_1_x | puertos |   \n\
                otro_test | a | motivo\n\
                err_1_x | b | uno\n\
                err_1_x | b | dos\n";

    let problems = exemption_problems(text, &tests);

    assert!(
        problems
            .iter()
            .any(|p| p.starts_with(&format!("{MATERIAL_EXEMPT}:1: "))),
        "{problems:?}"
    );
    assert!(
        problems
            .iter()
            .any(|p| p.starts_with(&format!("{MATERIAL_EXEMPT}:2: "))),
        "{problems:?}"
    );
    assert!(
        problems
            .iter()
            .any(|p| p.starts_with(&format!("{MATERIAL_EXEMPT}:3: `otro_test`"))),
        "{problems:?}"
    );
    assert!(
        problems
            .iter()
            .any(|p| p.starts_with(&format!("{MATERIAL_EXEMPT}:5: "))),
        "{problems:?}"
    );
    assert_eq!(problems.len(), 4, "{problems:?}");
}

/// Las exenciones del proyecto tienen el formato y nombran tests que existen
/// y que revisan material (llaman a `assert_no_violations`): si no, nadie la
/// contrasta con un conjunto y nunca se pide borrarla.
#[test]
fn ver_1_material_exemptions_name_tests() {
    let dir = root().join("tools/architecture/tests");
    let mut calls = Calls::default();
    for path in rust_files(&dir)
        .into_iter()
        .filter(|p| !p.starts_with(dir.join("fixtures")))
    {
        calls.visit_file(&parse(&path));
    }
    let text = fs::read_to_string(root().join("tools/architecture").join(MATERIAL_EXEMPT))
        .unwrap_or_default();

    let problems = exemption_problems(&text, &calls.asserting);

    assert!(
        problems.is_empty(),
        "\nVER-1: exenciones de material\n{}\n",
        problems
            .iter()
            .map(|p| format!("  - {p}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

fn fixture(name: &str) -> std::path::PathBuf {
    root().join("tools/architecture/tests/fixtures").join(name)
}

/// `assert_no_violations` lee las exenciones y la línea base del directorio:
/// el conjunto exento pasa con la variable, el no exento falla.
#[test]
fn ver_1_verdict_reads_the_exemptions_of_the_directory() {
    let dir = fixture("exemptions");

    let exempt = verdict(&dir, "err_1_x", "ERR-1: x", &[("puertos", 0)], &[], true);
    let not_exempt = verdict(&dir, "err_1_x", "ERR-1: x", &[("hojas", 0)], &[], true);

    assert_eq!(exempt, "");
    assert!(not_exempt.contains("  - 0 hojas"), "{not_exempt}");
}

/// Y la línea base del mismo directorio.
#[test]
fn ver_1_verdict_reads_the_baseline_of_the_directory() {
    let dir = fixture("exemptions");

    let report = verdict(
        &dir,
        "err_2_x",
        "ERR-1: x",
        &[("puertos", 0)],
        &violations(&["c/src/d.rs:3: tolerada"]),
        true,
    );

    assert_eq!(report, "");
}

/// Con `ARCH_BASELINE_PRINT=1`, cada test imprime su parte de la línea base:
/// una entrada por clave, con el conteo si pasa de 1, en el formato de
/// `baseline.txt` y con el prefijo que filtra el procedimiento.
#[test]
fn ver_1_baseline_lines_count_violations_by_key() {
    let lines = baseline_lines(
        "hex_2_y",
        &violations(&[
            "c/src/d.rs:3: una",
            "a/Cargo.toml#dependencies.b: x usa `b`",
            "c/src/d.rs:9: otra",
        ]),
    );

    assert_eq!(
        lines,
        [
            "baseline: hex_2_y a/Cargo.toml#dependencies.b",
            "baseline: hex_2_y c/src/d.rs 2",
        ]
    );
}

/// Lo que imprime se lee igual que la línea base: el test que lo generó la
/// tolera completa.
#[test]
fn ver_1_baseline_lines_round_trip_through_the_ratchet() {
    let found = violations(&["c/src/d.rs:3: una", "c/src/d.rs:9: otra", "e/src/f.rs:1: x"]);
    let text: String = baseline_lines("hex_2_y", &found)
        .iter()
        .map(|l| format!("{}\n", l.trim_start_matches("baseline: ")))
        .collect();

    let report = ratchet_report("hex_2_y", &parse_baseline(&text, "hex_2_y"), &found);

    assert_eq!(report, "");
}
