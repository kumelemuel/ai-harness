"""Tests de la parte determinista del runner de katas (`scripts/run_evals.py`).

La ejecución real de una kata cuesta tokens y requiere autenticación: no se
prueba aquí. Se prueba lo que interpreta su salida, con fixtures grabados en
`fixtures/evals/` (`stream.jsonl` es la salida real de `claude -p
--output-format stream-json --verbose`, sin el mensaje `init`).

    python3 -m unittest discover -s scripts/tests
"""

from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
SCRIPTS = HERE.parent
FIXTURES = HERE / "fixtures" / "evals"
sys.path.insert(0, str(SCRIPTS))

import run_evals  # noqa: E402

# Directorio donde corrió la sesión grabada y raíz del harness en esa máquina.
RECORDED_WORKDIR = Path("/tmp/evalprobe2")
RECORDED_HARNESS = Path("/home/kumelemuel/Development/ai-harness")


def recorded_stream() -> run_evals.SessionMetrics:
    lines = (FIXTURES / "stream.jsonl").read_text(encoding="utf-8").splitlines()
    return run_evals.parse_stream(lines, RECORDED_WORKDIR, RECORDED_HARNESS)


class Stream(unittest.TestCase):
    def test_usage_and_cost_come_from_the_result_event(self) -> None:
        metrics = recorded_stream()
        self.assertEqual(
            {"input": 4, "output": 120, "cache_read": 31107, "cache_write": 12746},
            metrics.tokens,
        )
        self.assertAlmostEqual(0.00292067, metrics.cost_usd)
        self.assertEqual(4, metrics.turns)
        self.assertFalse(metrics.is_error)

    def test_model_usage_is_kept_per_model(self) -> None:
        self.assertEqual(["claude-haiku-5-5"], sorted(recorded_stream().models))

    def test_skills_loaded_come_from_skill_tool_uses(self) -> None:
        self.assertEqual(["craft-backend:tdd"], recorded_stream().skills)

    def test_files_read_are_relative_to_the_workdir(self) -> None:
        self.assertEqual(["a.txt"], recorded_stream().files_read)

    def test_tool_uses_are_counted_by_name(self) -> None:
        self.assertEqual({"Skill": 1, "Read": 1}, recorded_stream().tools)

    def test_reads_inside_the_harness_are_marked_and_others_stay_absolute(self) -> None:
        lines = [
            _assistant_tool("Read", {"file_path": str(RECORDED_HARNESS / "stacks/rust/README.md")}),
            _assistant_tool("Read", {"file_path": "/etc/hosts"}),
            _assistant_tool("Read", {"file_path": str(RECORDED_WORKDIR / "src/a.rs")}),
            _assistant_tool("Read", {"file_path": str(RECORDED_WORKDIR / "src/a.rs")}),
            _assistant_tool("Read", {"file_path": "src/b.rs"}),
        ]
        metrics = run_evals.parse_stream(lines, RECORDED_WORKDIR, RECORDED_HARNESS)
        self.assertEqual(
            ["/etc/hosts", "harness:stacks/rust/README.md", "src/a.rs", "src/b.rs"],
            metrics.files_read,
        )
        self.assertEqual(5, metrics.tools["Read"])
        self.assertEqual(1, metrics.harness_files_read)

    def test_subagent_tool_uses_count_too(self) -> None:
        lines = [
            _assistant_tool("Agent", {"subagent_type": "craft-backend:architecture-reviewer"}),
            _assistant_tool("Read", {"file_path": "x.rs"}, parent="toolu_1"),
        ]
        metrics = run_evals.parse_stream(lines, RECORDED_WORKDIR, RECORDED_HARNESS)
        self.assertEqual(["x.rs"], metrics.files_read)
        self.assertEqual(["craft-backend:architecture-reviewer"], metrics.subagents)

    def test_a_stream_without_result_is_an_error(self) -> None:
        metrics = run_evals.parse_stream([], RECORDED_WORKDIR, RECORDED_HARNESS)
        self.assertTrue(metrics.is_error)
        self.assertEqual(0, metrics.turns)

    def test_api_error_status_is_kept(self) -> None:
        lines = [json.dumps({"type": "result", "num_turns": 1, "is_error": True, "api_error_status": 429})]
        metrics = run_evals.parse_stream(lines, RECORDED_WORKDIR, RECORDED_HARNESS)
        self.assertEqual(429, metrics.api_error_status)
        self.assertIsNone(recorded_stream().api_error_status)

    def test_non_json_lines_are_ignored(self) -> None:
        lines = ["no es json", *(FIXTURES / "stream.jsonl").read_text(encoding="utf-8").splitlines()]
        metrics = run_evals.parse_stream(lines, RECORDED_WORKDIR, RECORDED_HARNESS)
        self.assertEqual(4, metrics.turns)


class Kata(unittest.TestCase):
    def test_reads_kata_toml_with_defaults(self) -> None:
        kata = run_evals.load_kata(FIXTURES / "kata_valid")
        self.assertEqual("kata_valid", kata.name)
        self.assertEqual("stacks/rust/template", kata.template)
        self.assertEqual(["craft-backend"], kata.plugins)
        self.assertEqual(["cargo build --workspace --tests"], kata.setup)
        self.assertEqual("python3 acceptance/check.py", kata.acceptance)
        self.assertEqual(40, kata.timeout_minutes)
        self.assertEqual({"CARGO_TARGET_DIR": "{cache}/target"}, kata.env)
        self.assertEqual("Agrega algo.", kata.prompt.strip())

    def test_env_placeholders_are_expanded(self) -> None:
        kata = run_evals.load_kata(FIXTURES / "kata_valid")
        self.assertEqual(
            {"CARGO_TARGET_DIR": "/cache/kata_valid/target"},
            kata.expanded_env(Path("/cache/kata_valid")),
        )

    def test_missing_acceptance_is_rejected(self) -> None:
        with self.assertRaisesRegex(run_evals.KataError, "acceptance"):
            run_evals.load_kata(FIXTURES / "kata_missing_acceptance")

    def test_missing_prompt_is_rejected(self) -> None:
        with self.assertRaisesRegex(run_evals.KataError, "prompt.md"):
            run_evals.load_kata(FIXTURES)


class HookLog(unittest.TestCase):
    def test_counts_blocks_by_mode(self) -> None:
        text = (FIXTURES / "hooks.log").read_text(encoding="utf-8")
        self.assertEqual({"post-edit": 1, "stop": 1}, run_evals.parse_hook_log(text))

    def test_empty_log_has_no_blocks(self) -> None:
        self.assertEqual({}, run_evals.parse_hook_log(""))


class Acceptance(unittest.TestCase):
    def test_checks_come_from_pass_and_fail_lines(self) -> None:
        text = (FIXTURES / "acceptance_output.txt").read_text(encoding="utf-8")
        result = run_evals.parse_acceptance(text, exit_code=1)
        self.assertEqual(
            {
                "rename_by_owner": True,
                "rename_by_non_owner_is_forbidden": False,
                "architecture_tests": True,
            },
            result.checks,
        )
        self.assertFalse(result.passed)

    def test_passes_only_with_exit_zero_and_no_failed_check(self) -> None:
        self.assertTrue(run_evals.parse_acceptance("PASS a\n", exit_code=0).passed)
        self.assertFalse(run_evals.parse_acceptance("PASS a\n", exit_code=1).passed)
        self.assertFalse(run_evals.parse_acceptance("FAIL a\n", exit_code=0).passed)

    def test_no_checks_is_not_a_pass(self) -> None:
        self.assertFalse(run_evals.parse_acceptance("", exit_code=0).passed)


class Summary(unittest.TestCase):
    def test_median_and_range(self) -> None:
        self.assertEqual({"median": 2, "min": 1, "max": 9}, run_evals.spread([9, 1, 2]))
        self.assertEqual({"median": 2.5, "min": 1, "max": 4}, run_evals.spread([4, 1, 2, 3]))

    def test_spread_of_nothing_is_none(self) -> None:
        self.assertIsNone(run_evals.spread([]))

    def test_kata_summary_aggregates_runs(self) -> None:
        runs = [
            _run(cost=1.0, turns=10, output=100, files=3, passed=True),
            _run(cost=3.0, turns=30, output=300, files=5, passed=False),
            _run(cost=2.0, turns=20, output=200, files=4, passed=True),
        ]
        summary = run_evals.summarize(runs)
        self.assertEqual({"median": 2.0, "min": 1.0, "max": 3.0}, summary["cost_usd"])
        self.assertEqual({"median": 20, "min": 10, "max": 30}, summary["turns"])
        self.assertEqual({"median": 200, "min": 100, "max": 300}, summary["tokens"]["output"])
        self.assertEqual({"median": 4, "min": 3, "max": 5}, summary["files_read"])
        self.assertEqual("2/3", summary["acceptance"])
        self.assertEqual({"check": "2/3"}, summary["checks"])

    def test_runs_with_api_errors_are_invalid_and_left_out(self) -> None:
        broken = _run(cost=0.0, turns=1, output=0, files=0, passed=False)
        broken["api_error_status"] = 429
        runs = [_run(1.0, 10, 100, 3, True), broken, _run(3.0, 30, 300, 5, True)]
        summary = run_evals.summarize(runs)
        self.assertEqual(1, summary["invalid"])
        self.assertEqual("2/2", summary["acceptance"])
        self.assertEqual({"median": 2.0, "min": 1.0, "max": 3.0}, summary["cost_usd"])

    def test_pending_indices_skip_valid_runs(self) -> None:
        broken = _run(0.0, 1, 0, 0, False)
        broken["api_error_status"] = 429
        existing = [dict(_run(1.0, 10, 100, 3, True), index=1), dict(broken, index=2)]
        self.assertEqual([2, 3], run_evals.pending_indices(existing, runs=3))
        self.assertEqual([1, 2, 3], run_evals.pending_indices([], runs=3))

    def test_table_has_one_row_per_kata(self) -> None:
        results = {
            "version": "0.11.0",
            "label": "base",
            "model": "claude-opus-5-5",
            "katas": {"rust-rename": {"summary": run_evals.summarize([_run(1.5, 12, 900, 7, True)])}},
        }
        table = run_evals.render_table([results])
        self.assertIn("| rust-rename | 0.11.0-base | claude-opus-5-5 |", table)
        self.assertIn("1/1", table)
        self.assertIn("$1.50", table)


def _assistant_tool(name: str, tool_input: dict, parent: str | None = None) -> str:
    import json

    return json.dumps(
        {
            "type": "assistant",
            "parent_tool_use_id": parent,
            "message": {"content": [{"type": "tool_use", "name": name, "input": tool_input}]},
        }
    )


def _run(cost: float, turns: int, output: int, files: int, passed: bool) -> dict:
    return {
        "cost_usd": cost,
        "turns": turns,
        "tokens": {"input": 1, "output": output, "cache_read": 10, "cache_write": 5},
        "files_read": [f"f{i}" for i in range(files)],
        "harness_files_read": 0,
        "skills": [],
        "hook_blocks": {},
        "acceptance": {"passed": passed, "checks": {"check": passed}},
    }


if __name__ == "__main__":
    unittest.main()
