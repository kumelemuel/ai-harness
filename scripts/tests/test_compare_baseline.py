"""Tests de `compare_baseline.py` (skill `adoption`, modo `sincronizar`).

Una sincronización con el harness no cambia código: toda entrada vieja tiene
que seguir en la nueva, igual o con otra clave. Una que desaparece o baja puede
ser un cierre en falso (la versión nueva dejó de detectar algo) y detiene el PR.
"""

from __future__ import annotations

import io
import sys
import tempfile
import unittest
import unittest.mock
from contextlib import redirect_stdout
from pathlib import Path

SCRIPTS = (
    Path(__file__).resolve().parents[2]
    / "plugins" / "craft-backend" / "skills" / "adoption" / "scripts"
)
sys.path.insert(0, str(SCRIPTS))

import compare_baseline  # noqa: E402

HEADER = "# Línea base\n#\n"
# Formato anterior a 0.17.0: una entrada por manifiesto, sin sección ni paquete.
OLD = HEADER + (
    "hex_1_domain_deps contexts/iam/Cargo.toml\n"
    "err_4_no_println contexts/iam/src/api/handler.rs 2\n"
    "hex_2_no_frameworks contexts/feed/Cargo.toml\n"
)
# Formato actual: sección y paquete, y `n` = capas virtuales legacy que la ven.
NEW = HEADER + (
    "hex_1_domain_deps contexts/iam/Cargo.toml#dependencies.axum 4\n"
    "hex_1_domain_deps contexts/iam/Cargo.toml#dependencies.sqlx 4\n"
    "err_4_no_println contexts/iam/src/api/handler.rs 2\n"
    "hex_2_no_frameworks contexts/feed/Cargo.toml#dependencies.axum 2\n"
)


class Compare(unittest.TestCase):
    def test_the_real_case_has_no_closures(self) -> None:
        result = compare_baseline.compare(OLD, NEW)
        self.assertEqual(result.kept, [("err_4_no_println", "contexts/iam/src/api/handler.rs", 2, 2)])
        self.assertEqual(
            result.rekeyed,
            {
                ("hex_1_domain_deps", "contexts/iam/Cargo.toml"): [
                    ("contexts/iam/Cargo.toml#dependencies.axum", 4),
                    ("contexts/iam/Cargo.toml#dependencies.sqlx", 4),
                ],
                ("hex_2_no_frameworks", "contexts/feed/Cargo.toml"): [
                    ("contexts/feed/Cargo.toml#dependencies.axum", 2),
                ],
            },
        )
        self.assertEqual(result.gone, [])
        self.assertEqual(result.lowered, [])
        self.assertEqual(result.added, [])
        self.assertTrue(result.ok)

    def test_an_entry_that_disappears_is_a_possible_false_closure(self) -> None:
        new = NEW.replace("hex_2_no_frameworks contexts/feed/Cargo.toml#dependencies.axum 2\n", "")
        result = compare_baseline.compare(OLD, new)
        self.assertEqual(result.gone, [("hex_2_no_frameworks", "contexts/feed/Cargo.toml", 1)])
        self.assertFalse(result.ok)

    def test_a_lower_count_on_the_same_key_stops_too(self) -> None:
        new = NEW.replace("handler.rs 2", "handler.rs 1")
        result = compare_baseline.compare(OLD, new)
        self.assertEqual(result.lowered, [("err_4_no_println", "contexts/iam/src/api/handler.rs", 2, 1)])
        self.assertFalse(result.ok)

    def test_a_new_entry_needs_an_inventory_id_but_does_not_stop(self) -> None:
        new = NEW + "hex_4_workflows contexts/iam/src/application/flow.rs\n"
        result = compare_baseline.compare(OLD, new)
        self.assertEqual(result.added, [("hex_4_workflows", "contexts/iam/src/application/flow.rs", 1)])
        self.assertTrue(result.ok)

    def test_a_higher_count_on_the_same_key_is_listed_with_the_new_ones(self) -> None:
        new = NEW.replace("handler.rs 2", "handler.rs 3")
        result = compare_baseline.compare(OLD, new)
        self.assertEqual(result.raised, [("err_4_no_println", "contexts/iam/src/api/handler.rs", 2, 3)])
        self.assertTrue(result.ok)

    def test_a_key_under_another_test_is_not_a_rekey(self) -> None:
        # Un test renombrado no se adivina: desaparece y aparece, y lo decide una persona.
        new = NEW.replace("hex_2_no_frameworks", "hex_2_server_frameworks")
        result = compare_baseline.compare(OLD, new)
        self.assertEqual(result.gone, [("hex_2_no_frameworks", "contexts/feed/Cargo.toml", 1)])
        self.assertEqual(
            result.added, [("hex_2_server_frameworks", "contexts/feed/Cargo.toml#dependencies.axum", 2)]
        )
        self.assertFalse(result.ok)

    def test_comments_and_order_do_not_matter(self) -> None:
        lines = OLD.splitlines(keepends=True)
        reordered = "# otra cabecera\n" + "".join(lines[2:][::-1])
        self.assertEqual(compare_baseline.compare(OLD, reordered).kept[0][0], "err_4_no_println")
        self.assertTrue(compare_baseline.compare(OLD, reordered).ok)


class Main(unittest.TestCase):
    def run_main(self, old: str, new: str) -> tuple[int, str]:
        with tempfile.TemporaryDirectory() as tmp:
            a, b = Path(tmp) / "old.txt", Path(tmp) / "new.txt"
            a.write_text(old, encoding="utf-8")
            b.write_text(new, encoding="utf-8")
            out = io.StringIO()
            with redirect_stdout(out):
                code = compare_baseline.main([str(a), str(b)])
        return code, out.getvalue()

    def test_the_report_has_the_four_groups_and_the_mapping(self) -> None:
        code, report = self.run_main(OLD, NEW)
        self.assertEqual(code, 0)
        for heading in ("Se mantienen (1)", "Cambian de clave (2 → 3)", "Desaparecen (0)", "Nuevas (0)"):
            self.assertIn(heading, report)
        self.assertIn(
            "`hex_1_domain_deps contexts/iam/Cargo.toml` → "
            "`contexts/iam/Cargo.toml#dependencies.axum 4`, "
            "`contexts/iam/Cargo.toml#dependencies.sqlx 4`",
            report,
        )

    def test_a_disappearance_exits_1_and_says_why(self) -> None:
        code, report = self.run_main(OLD, NEW.replace("handler.rs 2", "handler.rs 1"))
        self.assertEqual(code, 1)
        self.assertIn("cierre en falso", report)

    def test_wrong_arguments_exit_2(self) -> None:
        with redirect_stdout(io.StringIO()), unittest.mock.patch("sys.stderr", io.StringIO()):
            self.assertEqual(compare_baseline.main(["solo-uno"]), 2)


if __name__ == "__main__":
    unittest.main()
