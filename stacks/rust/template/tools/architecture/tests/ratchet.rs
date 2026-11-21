//! Trinquete de la línea base (VER-1): `baseline.txt` solo se achica.

mod common;

use common::{parse_baseline, ratchet_report};
use std::collections::BTreeSet;

fn baseline(entries: &[&str]) -> BTreeSet<String> {
    entries.iter().map(|e| e.to_string()).collect()
}

fn violations(lines: &[&str]) -> Vec<String> {
    lines.iter().map(|l| l.to_string()).collect()
}

#[test]
fn ver_1_baseline_reads_only_the_entries_of_its_test() {
    let text = "# comentario\n\nhex_1_x a/Cargo.toml#b\nhex_2_y c/src/d.rs\nhex_1_x e/src/f.rs\n";

    let entries = parse_baseline(text, "hex_1_x");

    assert_eq!(entries, baseline(&["a/Cargo.toml#b", "e/src/f.rs"]));
}

#[test]
fn ver_1_baseline_tolerates_the_listed_violations() {
    let report = ratchet_report(
        "hex_2_y",
        &baseline(&["c/src/d.rs"]),
        &violations(&["c/src/d.rs:3: `Foo` expone `Bar`", "c/src/d.rs:9: otra"]),
    );

    assert_eq!(report, "");
}

#[test]
fn ver_1_baseline_rejects_a_new_violation() {
    let report = ratchet_report(
        "hex_2_y",
        &baseline(&["c/src/d.rs"]),
        &violations(&["c/src/d.rs:3: vieja", "g/src/h.rs:1: nueva"]),
    );

    assert!(report.contains("  - g/src/h.rs:1: nueva"), "{report}");
    assert!(!report.contains("vieja"), "{report}");
}

#[test]
fn ver_1_baseline_asks_to_delete_what_no_longer_violates() {
    let report = ratchet_report("hex_2_y", &baseline(&["c/src/d.rs"]), &[]);

    assert!(
        report.contains("  - borrar de baseline.txt: hex_2_y c/src/d.rs"),
        "{report}"
    );
}

/// Una entrada por dependencia: tolerar un manifiesto no tolera cualquier
/// dependencia nueva en él.
#[test]
fn ver_1_baseline_keys_dependencies_one_by_one() {
    let report = ratchet_report(
        "hex_2_core",
        &baseline(&["contexts/x/Cargo.toml#uuid"]),
        &violations(&[
            "contexts/x/Cargo.toml#uuid: x (Domain) usa el crate externo `uuid`",
            "contexts/x/Cargo.toml#chrono: x (Domain) usa el crate externo `chrono`",
        ]),
    );

    assert!(report.contains("Cargo.toml#chrono"), "{report}");
    assert!(!report.contains("borrar"), "{report}");
}

/// Mover un archivo con violaciones da a la vez una nueva y una para borrar.
/// El método es corregir en el lugar y después mover.
#[test]
fn ver_1_baseline_hints_at_a_move() {
    let report = ratchet_report(
        "hex_2_y",
        &baseline(&["c/src/old.rs"]),
        &violations(&["c/src/new.rs:3: la misma"]),
    );

    assert!(report.contains("corregir en el lugar"), "{report}");
}
