"""Tests del verificador de consistencia del harness (VER-1 aplicado al repo).

Cada fixture de `fixtures/` es un mini-repo: `valid` no tiene problemas y cada
uno de los demás contiene un solo tipo de problema. Se comprueba el tipo, el
archivo y lo esencial del mensaje, y que no aparezca nada más.

    python3 -m unittest discover -s scripts/tests
"""

from __future__ import annotations

import subprocess
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
SCRIPTS = HERE.parent
FIXTURES = HERE / "fixtures"
sys.path.insert(0, str(SCRIPTS))

import check_harness  # noqa: E402


def problems(fixture: str) -> list[check_harness.Problem]:
    return check_harness.check(FIXTURES / fixture)


def summary(fixture: str) -> set[tuple[str, str]]:
    """(código, archivo) de cada problema, para comparar sin depender del texto."""
    return {(p.code, p.path) for p in problems(fixture)}


class ValidRepo(unittest.TestCase):
    def test_reports_nothing(self) -> None:
        self.assertEqual([], [p.render() for p in problems("valid")])


class Ids(unittest.TestCase):
    def test_cited_id_must_be_defined_in_its_source(self) -> None:
        found = problems("undefined_id")
        self.assertEqual({("ID-UNDEFINED", "README.md")}, {(p.code, p.path) for p in found})
        messages = " ".join(p.message for p in found)
        self.assertIn("HEX-9", messages)
        self.assertIn("ERR-1", messages)  # su archivo fuente ni siquiera existe
        self.assertNotIn("HEX-1 ", messages)
        self.assertEqual(2, len(found))

    def test_defined_id_must_not_repeat(self) -> None:
        found = problems("duplicate_id")
        self.assertEqual({("ID-DUPLICATE", "core/principles/hexagonal.md")}, summary("duplicate_id"))
        self.assertEqual(1, len(found))
        self.assertIn("HEX-1", found[0].message)
        self.assertEqual(7, found[0].line)


class Links(unittest.TestCase):
    def test_every_symlink_resolves(self) -> None:
        found = problems("broken_symlink")
        self.assertEqual(
            {("SYMLINK", "plugins/demo/skills/tdd/references/roto.md")},
            summary("broken_symlink"),
        )
        self.assertIn("no-existe.md", found[0].message)

    def test_every_relative_markdown_link_resolves(self) -> None:
        found = problems("broken_md_link")
        self.assertEqual({("MD-LINK", "README.md")}, summary("broken_md_link"))
        self.assertEqual(
            ["docs/falta.md", "docs/tampoco.md"],
            sorted(p.message.split("`")[1] for p in found),
        )
        self.assertEqual([3, 5], sorted(p.line for p in found))


class Frontmatter(unittest.TestCase):
    def test_skills_and_agents_need_name_and_description(self) -> None:
        self.assertEqual(
            {
                ("FRONTMATTER", "plugins/demo/skills/tdd/SKILL.md"),
                ("FRONTMATTER", "plugins/demo/agents/reviewer.md"),
            },
            summary("frontmatter_missing"),
        )
        messages = {p.path: p.message for p in problems("frontmatter_missing")}
        self.assertIn("description", messages["plugins/demo/skills/tdd/SKILL.md"])
        self.assertIn("frontmatter", messages["plugins/demo/agents/reviewer.md"])

    def test_name_matches_folder_or_file(self) -> None:
        self.assertEqual(
            {
                ("FRONTMATTER", "plugins/demo/skills/tdd/SKILL.md"),
                ("FRONTMATTER", "plugins/demo/agents/reviewer.md"),
            },
            summary("frontmatter_name_mismatch"),
        )
        messages = " ".join(p.message for p in problems("frontmatter_name_mismatch"))
        self.assertIn("test-driven", messages)
        self.assertIn("revisor", messages)


class Stacks(unittest.TestCase):
    def test_paths_in_stack_skill_description_exist_in_template(self) -> None:
        found = problems("stack_description_path")
        self.assertEqual(
            {("STACK-PATH", "plugins/demo/skills/rust-stack/SKILL.md")},
            summary("stack_description_path"),
        )
        self.assertEqual(
            ["apps/api", "contexts/*/domain"],
            sorted(p.message.split("`")[1] for p in found),
        )

    def test_stack_status_matches_its_guide(self) -> None:
        found = problems("stack_status_mismatch")
        self.assertEqual({("STACK-STATUS", "stacks/README.md")}, summary("stack_status_mismatch"))
        messages = sorted(p.message for p in found)
        self.assertEqual(2, len(found))
        self.assertIn("v0.1", messages[0] + messages[1])
        self.assertTrue(any("v0.2" in m for m in messages))
        self.assertTrue(any("astro/README.md" in m for m in messages))


class Cli(unittest.TestCase):
    def run_cli(self, fixture: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [sys.executable, "-B", str(SCRIPTS / "check_harness.py"), str(FIXTURES / fixture)],
            capture_output=True,
            text=True,
            check=False,
        )

    def test_valid_repo_exits_zero(self) -> None:
        result = self.run_cli("valid")
        self.assertEqual(0, result.returncode, result.stdout + result.stderr)

    def test_one_line_per_problem_and_nonzero_exit(self) -> None:
        result = self.run_cli("stack_status_mismatch")
        self.assertNotEqual(0, result.returncode)
        lines = [line for line in result.stdout.splitlines() if line.strip()]
        self.assertEqual(2, len(lines))
        for line in lines:
            self.assertRegex(line, r"^stacks/README\.md:\d+: STACK-STATUS: ")


if __name__ == "__main__":
    unittest.main()
