"""Tests del verificador de consistencia del harness (VER-1 aplicado al repo).

Cada fixture de `fixtures/` es un mini-repo: `valid` no tiene problemas y cada
uno de los demás contiene un solo tipo de problema. Se comprueba el tipo, el
archivo y lo esencial del mensaje, y que no aparezca nada más.

    python3 -m unittest discover -s scripts/tests
"""

from __future__ import annotations

import json
import shutil
import subprocess
import sys
import tempfile
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


class VolatileNumbers(unittest.TestCase):
    def test_docs_do_not_count_tests_except_the_changelog(self) -> None:
        found = problems("volatile_count")
        self.assertEqual(
            {("VOLATILE-COUNT", "README.md"), ("VOLATILE-COUNT", "docs/guia.md")},
            summary("volatile_count"),
        )
        self.assertEqual(
            [("README.md", 6, "36 tests"), ("docs/guia.md", 3, "1 test"),
             ("docs/guia.md", 3, "16 e2e"), ("docs/guia.md", 3, "22 tests")],
            sorted((p.path, p.line, p.message.split("`")[1]) for p in found),
        )


class Release(unittest.TestCase):
    """`--base`: lo que entra a `main` publica una versión nueva.

    Cada test copia `fixtures/version_bump` a un repo git temporal, hace un
    commit base, aplica un cambio en un segundo commit y verifica contra la base.
    En el fixture, `demo` alcanza `core/principles/tdd.md` y `stacks/demo/template`
    por symlinks, y `other` alcanza `core/principles/web.md`.
    """

    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name) / "repo"
        shutil.copytree(FIXTURES / "version_bump", self.root, symlinks=True)
        self.git("init", "-q", "-b", "main")
        self.commit("base")
        self.base = self.git("rev-parse", "HEAD").strip()

    def tearDown(self) -> None:
        self.tmp.cleanup()

    def git(self, *args: str) -> str:
        return subprocess.run(
            ["git", "-c", "user.name=t", "-c", "user.email=t@t", "-C", str(self.root), *args],
            capture_output=True, text=True, check=True,
        ).stdout

    def commit(self, message: str) -> None:
        self.git("add", "-A")
        self.git("commit", "-q", "--allow-empty", "-m", message)

    def append(self, path: str, text: str = "cambio\n") -> None:
        with open(self.root / path, "a", encoding="utf-8") as handle:
            handle.write(text)

    def bump(self, plugin: str, version: str) -> None:
        manifest = self.root / "plugins" / plugin / ".claude-plugin" / "plugin.json"
        data = json.loads(manifest.read_text(encoding="utf-8"))
        data["version"] = version
        manifest.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")

    def release_problems(self) -> list[check_harness.Problem]:
        self.commit("cambio")
        return check_harness.check_release(self.root, self.base)

    def test_file_reached_by_symlink_without_bump_fails_naming_the_plugin(self) -> None:
        self.append("core/principles/tdd.md")
        found = self.release_problems()
        self.assertEqual(
            {("VERSION-NOT-BUMPED", "plugins/demo/.claude-plugin/plugin.json")},
            {(p.code, p.path) for p in found},
        )
        self.assertIn("demo", found[0].message)
        self.assertIn("1.0.0", found[0].message)
        self.assertIn("core/principles/tdd.md", found[0].message)

    def test_bumped_version_passes(self) -> None:
        self.append("core/principles/tdd.md")
        self.bump("demo", "1.0.1")
        self.assertEqual([], [p.render() for p in self.release_problems()])

    def test_file_inside_a_symlinked_directory_counts(self) -> None:
        self.append("stacks/demo/template/src/main.rs", "// cambio\n")
        self.assertEqual(
            {("VERSION-NOT-BUMPED", "plugins/demo/.claude-plugin/plugin.json")},
            {(p.code, p.path) for p in self.release_problems()},
        )

    def test_new_and_deleted_files_count(self) -> None:
        (self.root / "stacks/demo/template/src/lib.rs").write_text("", encoding="utf-8")
        (self.root / "plugins/other/skills/web/SKILL.md").unlink()
        self.assertEqual(
            {
                ("VERSION-NOT-BUMPED", "plugins/demo/.claude-plugin/plugin.json"),
                ("VERSION-NOT-BUMPED", "plugins/other/.claude-plugin/plugin.json"),
            },
            {(p.code, p.path) for p in self.release_problems()},
        )

    def test_file_inside_the_plugin_directory_counts(self) -> None:
        self.append("plugins/other/skills/web/SKILL.md")
        found = self.release_problems()
        self.assertEqual(
            {("VERSION-NOT-BUMPED", "plugins/other/.claude-plugin/plugin.json")},
            {(p.code, p.path) for p in found},
        )
        self.assertIn("0.1.0", found[0].message)

    def test_only_affected_plugins_need_a_bump(self) -> None:
        self.append("core/principles/web.md")
        self.bump("other", "0.2.0")
        self.append("README.md")
        self.append("core/principles/suelto.md")
        self.assertEqual([], [p.render() for p in self.release_problems()])

    def test_version_must_increase(self) -> None:
        self.append("core/principles/tdd.md")
        self.bump("demo", "0.9.0")
        found = self.release_problems()
        self.assertEqual(["VERSION-NOT-BUMPED"], [p.code for p in found])
        self.assertIn("0.9.0", found[0].message)

    def test_new_plugin_needs_no_bump(self) -> None:
        plugin = self.root / "plugins" / "nuevo" / ".claude-plugin"
        plugin.mkdir(parents=True)
        (plugin / "plugin.json").write_text('{"name": "nuevo", "version": "0.1.0"}\n', encoding="utf-8")
        marketplace = self.root / ".claude-plugin" / "marketplace.json"
        data = json.loads(marketplace.read_text(encoding="utf-8"))
        data["plugins"].append({"name": "nuevo", "source": "./plugins/nuevo"})
        marketplace.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
        self.assertEqual([], [p.render() for p in self.release_problems()])

    def test_unreleased_section_fails(self) -> None:
        changelog = self.root / "CHANGELOG.md"
        changelog.write_text(
            "# Changelog\n\n## Sin publicar\n\n- Algo.\n\n## 1.0.0 — 2026-11-12\n",
            encoding="utf-8",
        )
        found = self.release_problems()
        self.assertEqual([("CHANGELOG-UNRELEASED", "CHANGELOG.md", 3)],
                         [(p.code, p.path, p.line) for p in found])

    def test_unreleased_section_is_fine_without_base(self) -> None:
        (self.root / "CHANGELOG.md").write_text("# Changelog\n\n## Sin publicar\n", encoding="utf-8")
        self.assertEqual([], [p.render() for p in check_harness.check(self.root)])

    def test_cli_with_base(self) -> None:
        self.append("core/principles/tdd.md")
        self.commit("cambio")
        result = subprocess.run(
            [sys.executable, "-B", str(SCRIPTS / "check_harness.py"), str(self.root),
             "--base", self.base],
            capture_output=True, text=True, check=False,
        )
        self.assertNotEqual(0, result.returncode)
        self.assertRegex(
            result.stdout,
            r"^plugins/demo/\.claude-plugin/plugin\.json:\d+: VERSION-NOT-BUMPED: ",
        )


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
