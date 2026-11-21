"""Tests del hook `pre-edit` de craft-backend: la línea base solo se achica."""

from __future__ import annotations

import io
import os
import sys
import tempfile
import unittest
from contextlib import redirect_stderr
from pathlib import Path
from unittest import mock

HOOKS = Path(__file__).resolve().parents[2] / "plugins" / "craft-backend" / "hooks"
sys.path.insert(0, str(HOOKS))

import craft_hooks  # noqa: E402

HEADER = "# Línea base\n#\n"
BASE = HEADER + "hex_1_x a/Cargo.toml#dependencies.b\nerr_4_y c/src/d.rs 3\n"


class BaselineGrowth(unittest.TestCase):
    def test_a_new_entry_is_growth(self) -> None:
        grown = craft_hooks.baseline_growth(BASE, BASE + "hex_2_z e/src/f.rs\n")
        self.assertEqual(grown, ["hex_2_z e/src/f.rs"])

    def test_a_higher_count_is_growth(self) -> None:
        new = BASE.replace("c/src/d.rs 3", "c/src/d.rs 4")
        self.assertEqual(craft_hooks.baseline_growth(BASE, new), ["err_4_y c/src/d.rs 4"])

    def test_an_implicit_count_is_one(self) -> None:
        new = BASE.replace("a/Cargo.toml#dependencies.b", "a/Cargo.toml#dependencies.b 2")
        self.assertEqual(
            craft_hooks.baseline_growth(BASE, new), ["hex_1_x a/Cargo.toml#dependencies.b 2"]
        )

    def test_a_signed_count_reads_like_the_rust_ratchet(self) -> None:
        # `parse::<usize>()` acepta `+5`: el hook tiene que leer lo mismo.
        new = BASE.replace("c/src/d.rs 3", "c/src/d.rs +5")
        self.assertEqual(craft_hooks.baseline_growth(BASE, new), ["err_4_y c/src/d.rs 5"])

    def test_a_non_ascii_digit_does_not_crash(self) -> None:
        new = BASE.replace("c/src/d.rs 3", "c/src/d.rs ²")
        self.assertEqual(craft_hooks.baseline_growth(BASE, new), [])

    def test_reordering_is_not_growth(self) -> None:
        lines = BASE.splitlines(keepends=True)
        new = "".join(lines[:2] + lines[2:][::-1])
        self.assertEqual(craft_hooks.baseline_growth(BASE, new), [])

    def test_renaming_a_key_is_growth(self) -> None:
        new = BASE.replace("e/src", "x/src").replace("c/src/d.rs", "c/src/moved.rs")
        self.assertEqual(craft_hooks.baseline_growth(BASE, new), ["err_4_y c/src/moved.rs 3"])

    def test_deleting_lowering_and_comments_are_not_growth(self) -> None:
        new = HEADER + "# nota nueva\nerr_4_y c/src/d.rs 2\n"
        self.assertEqual(craft_hooks.baseline_growth(BASE, new), [])


class PreEdit(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        self.baseline = Path(self.tmp.name) / "tools" / "architecture" / "baseline.txt"
        self.baseline.parent.mkdir(parents=True)
        (self.baseline.parent / "Cargo.toml").write_text('[package]\nname = "architecture"\n')
        self.baseline.write_text(BASE, encoding="utf-8")
        self.env = mock.patch.dict(os.environ, {}, clear=False)
        self.env.start()
        os.environ.pop("CRAFT_BACKEND_BASELINE_INIT", None)

    def tearDown(self) -> None:
        self.env.stop()
        self.tmp.cleanup()

    def run_hook(self, tool: str, tool_input: dict, cwd: str | None = None) -> tuple[int, str]:
        stderr = io.StringIO()
        payload = {"tool_name": tool, "tool_input": tool_input, "cwd": cwd or self.tmp.name}
        with redirect_stderr(stderr):
            code = craft_hooks.pre_edit(payload)
        return code, stderr.getvalue()

    def test_write_that_adds_an_entry_is_blocked(self) -> None:
        code, message = self.run_hook(
            "Write", {"file_path": str(self.baseline), "content": BASE + "hex_2_z e/src/f.rs\n"}
        )
        self.assertEqual(code, 2)
        self.assertIn("hex_2_z e/src/f.rs", message)
        self.assertIn("solo se achica", message)

    def test_edit_that_raises_a_count_is_blocked(self) -> None:
        code, _ = self.run_hook(
            "Edit",
            {
                "file_path": str(self.baseline),
                "old_string": "c/src/d.rs 3",
                "new_string": "c/src/d.rs 5",
            },
        )
        self.assertEqual(code, 2)

    def test_multiedit_that_adds_an_entry_is_blocked(self) -> None:
        code, _ = self.run_hook(
            "MultiEdit",
            {
                "file_path": str(self.baseline),
                "edits": [
                    {"old_string": "c/src/d.rs 3", "new_string": "c/src/d.rs 2"},
                    {"old_string": "# Línea base\n", "new_string": "# Línea base\nnew_test g.rs\n"},
                ],
            },
        )
        self.assertEqual(code, 2)

    def test_edit_that_deletes_an_entry_passes(self) -> None:
        code, _ = self.run_hook(
            "Edit",
            {
                "file_path": str(self.baseline),
                "old_string": "hex_1_x a/Cargo.toml#dependencies.b\n",
                "new_string": "",
            },
        )
        self.assertEqual(code, 0)

    def test_initial_fill_is_allowed_with_the_variable(self) -> None:
        os.environ["CRAFT_BACKEND_BASELINE_INIT"] = "1"
        code, _ = self.run_hook(
            "Write", {"file_path": str(self.baseline), "content": BASE + "hex_2_z e/src/f.rs\n"}
        )
        self.assertEqual(code, 0)

    def test_an_edit_whose_result_cannot_be_computed_is_blocked(self) -> None:
        code, message = self.run_hook(
            "Edit",
            {"file_path": str(self.baseline), "old_string": "no existe", "new_string": "x y\n"},
        )
        self.assertEqual(code, 2)
        self.assertIn("Write", message)

    def test_write_on_a_missing_baseline_with_entries_is_blocked(self) -> None:
        self.baseline.unlink()
        code, _ = self.run_hook("Write", {"file_path": str(self.baseline), "content": "a b\n"})
        self.assertEqual(code, 2)

    def test_a_relative_path_is_resolved_against_the_session_cwd(self) -> None:
        code, _ = self.run_hook(
            "Write",
            {"file_path": "tools/architecture/baseline.txt", "content": BASE + "z w\n"},
            cwd=self.tmp.name,
        )
        self.assertEqual(code, 2)

    def test_the_message_forbids_editing_by_shell(self) -> None:
        _, message = self.run_hook(
            "Write", {"file_path": str(self.baseline), "content": BASE + "z w\n"}
        )
        self.assertIn("shell", message)

    def test_an_architecture_dir_outside_tools_is_ignored(self) -> None:
        other = Path(self.tmp.name) / "docs" / "architecture" / "baseline.txt"
        code, _ = self.run_hook("Write", {"file_path": str(other), "content": "a b\n"})
        self.assertEqual(code, 0)

    def test_other_files_are_ignored(self) -> None:
        other = Path(self.tmp.name) / "notes" / "baseline.txt"
        code, _ = self.run_hook("Write", {"file_path": str(other), "content": "a b\n"})
        self.assertEqual(code, 0)


if __name__ == "__main__":
    unittest.main()
