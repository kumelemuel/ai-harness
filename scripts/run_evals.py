#!/usr/bin/env python3
"""Runner de katas: mide corrección y consumo del harness con sesiones reales.

Uso:
  python3 scripts/run_evals.py run --model MODELO --label ETIQUETA [--kata K ...]
                                   [--runs 3] [--parallel N] [--keep]
  python3 scripts/run_evals.py table evals/results/*.json

Cada kata vive en `evals/<kata>/`: `prompt.md` (la tarea), `acceptance/` (tests
ocultos que se copian después de que el agente termina) y `kata.toml`:

  template        ruta de la plantilla, relativa al repo (se copian sus archivos
                  versionados, sin `target/` ni `node_modules/`)
  plugins         plugins del marketplace `ai-harness` que se habilitan
  acceptance      comando de aceptación; imprime `PASS nombre` / `FAIL nombre: …`
  setup           comandos previos al agente (no cuentan en el consumo);
                  `{kata}` es el directorio de la kata
  timeout_minutes límite de la sesión del agente (por defecto 45)
  allowed_tools   herramientas extra, además de las de `ALLOWED_TOOLS`
  agent           corre la sesión como ese subagente (`--agent`). Ojo: así no se
                  precargan las `skills:` de su frontmatter
  subagent        `plugin:agente` que la sesión principal invoca como subagente
                  (con sus skills). Con `--subagent-model`, el plugin se carga
                  desde una copia con ese `model:` (`--plugin-dir`), y el reporte
                  que juzga la aceptación es el `tool_result` del subagente
  [env]           variables para el setup, el agente, sus hooks y la aceptación;
                  `{cache}` es un directorio persistente por kata y ejecución

Por ejecución: copia la plantilla a un directorio nuevo, hace un commit inicial
(los hooks miran `git status`), habilita los plugins con `enabledPlugins` en
`.claude/settings.json` del proyecto (y deshabilita el resto de los plugins
del usuario), corre `claude -p` en modo headless con el modelo fijado, copia
`acceptance/` y la corre. Registra tokens, costo, turnos, skills cargadas,
archivos leídos, bloqueos de hooks y aceptación. El resultado queda en
`evals/results/<versión>-<etiqueta>.json`; la salida cruda de cada ejecución,
en `~/.cache/ai-harness-evals/runs/`.

No corre en CI: cuesta tokens y requiere autenticación.

Solo biblioteca estándar (Python 3.11+), igual que los hooks.
"""

from __future__ import annotations

import argparse
import concurrent.futures
import datetime as dt
import json
import os
import re
import shlex
import shutil
import statistics
import subprocess
import sys
import tomllib
from dataclasses import dataclass, field
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
EVALS = REPO / "evals"
RESULTS = EVALS / "results"
CACHE = Path(os.environ.get("AI_HARNESS_EVALS_CACHE", Path.home() / ".cache" / "ai-harness-evals"))
MARKETPLACE = "ai-harness"

# Lo que puede hacer el agente sin preguntar (`--permission-mode dontAsk`): lo
# demás se deniega y queda en `permission_denials`. Sin `bypassPermissions`.
ALLOWED_TOOLS = [
    "Read", "Glob", "Grep", "Edit", "Write", "MultiEdit", "NotebookEdit",
    "Skill", "Agent", "Task", "TodoWrite",
    "Bash(cargo *)", "Bash(rustfmt *)", "Bash(npm *)", "Bash(npx *)", "Bash(node *)",
    "Bash(git status*)", "Bash(git diff*)", "Bash(git log*)", "Bash(git show*)",
    "Bash(ls *)", "Bash(find *)", "Bash(cat *)", "Bash(head *)", "Bash(tail *)",
    "Bash(grep *)", "Bash(rg *)", "Bash(wc *)", "Bash(mkdir *)", "Bash(sed -n *)",
]

HOOK_LOG_VARS = ("CRAFT_BACKEND_HOOKS_LOG", "CRAFT_WEB_HOOKS_LOG")


class KataError(Exception):
    pass


# --- Kata -------------------------------------------------------------------


@dataclass
class Kata:
    name: str
    path: Path
    prompt: str
    template: str
    plugins: list[str]
    acceptance: str
    setup: list[str] = field(default_factory=list)
    env: dict[str, str] = field(default_factory=dict)
    timeout_minutes: int = 45
    allowed_tools: list[str] = field(default_factory=list)
    agent: str | None = None
    subagent: str | None = None

    def expanded_setup(self) -> list[str]:
        return [command.replace("{kata}", str(self.path)) for command in self.setup]

    def expanded_env(self, cache: Path) -> dict[str, str]:
        return {key: value.replace("{cache}", str(cache)) for key, value in self.env.items()}


def load_kata(path: Path) -> Kata:
    prompt = path / "prompt.md"
    if not prompt.is_file():
        raise KataError(f"{path}: falta prompt.md")
    config_file = path / "kata.toml"
    if not config_file.is_file():
        raise KataError(f"{path}: falta kata.toml")
    config = tomllib.loads(config_file.read_text(encoding="utf-8"))
    for key in ("template", "plugins", "acceptance"):
        if key not in config:
            raise KataError(f"{config_file}: falta `{key}`")
    return Kata(
        name=path.name,
        path=path,
        prompt=prompt.read_text(encoding="utf-8"),
        template=config["template"],
        plugins=list(config["plugins"]),
        acceptance=config["acceptance"],
        setup=list(config.get("setup", [])),
        env={k: str(v) for k, v in config.get("env", {}).items()},
        timeout_minutes=int(config.get("timeout_minutes", 45)),
        allowed_tools=list(config.get("allowed_tools", [])),
        agent=config.get("agent"),
        subagent=config.get("subagent"),
    )


def discover_katas() -> list[Path]:
    return sorted(p for p in EVALS.iterdir() if (p / "kata.toml").is_file())


# --- Interpretación de la salida (determinista, con tests) -------------------


@dataclass
class SessionMetrics:
    tokens: dict[str, int]
    cost_usd: float
    turns: int
    is_error: bool
    models: dict[str, dict]
    skills: list[str]
    subagents: list[str]
    files_read: list[str]
    harness_files_read: int
    tools: dict[str, int]
    permission_denials: int
    result_text: str
    api_error_status: int | None
    requests: int = 0
    tool_result_chars: dict[str, int] = field(default_factory=dict)
    subagent_reports: list[str] = field(default_factory=list)
    subagent_async: bool = False


def parse_stream(lines: list[str], workdir: Path, harness: Path) -> SessionMetrics:
    """Interpreta `claude -p --output-format stream-json --verbose`.

    El consumo sale del evento `result`, que suma la sesión completa (también
    los subagentes). Herramientas, skills y archivos leídos salen de los
    `tool_use` de los mensajes `assistant`, incluidos los de subagentes
    (`parent_tool_use_id`).
    """
    result: dict | None = None
    tools: dict[str, int] = {}
    tool_names: dict[str, str] = {}
    result_chars: dict[str, int] = {}
    responses: set[str] = set()
    reports: list[str] = []
    launched_async = False
    skills: list[str] = []
    subagents: list[str] = []
    files: set[str] = set()
    for line in lines:
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            continue
        if event.get("type") == "result":
            result = event
        if event.get("type") == "user":
            _add_result_chars(event, tool_names, result_chars)
            for report in _subagent_reports(event, tool_names):
                if report.startswith("Async agent launched"):
                    launched_async = True
                else:
                    reports.append(report)
        if event.get("type") != "assistant":
            continue
        message_id = event.get("message", {}).get("id")
        if message_id:
            responses.add(message_id)
        for block in event.get("message", {}).get("content", []):
            if block.get("type") != "tool_use":
                continue
            name = block.get("name", "")
            if block.get("id"):
                tool_names[block["id"]] = name
            tool_input = block.get("input") or {}
            tools[name] = tools.get(name, 0) + 1
            if name == "Skill" and tool_input.get("skill"):
                skills.append(tool_input["skill"])
            elif name in ("Agent", "Task") and tool_input.get("subagent_type"):
                subagents.append(tool_input["subagent_type"])
            elif name == "Read" and tool_input.get("file_path"):
                files.add(_relative(tool_input["file_path"], workdir, harness))

    usage = (result or {}).get("usage", {})
    return SessionMetrics(
        tokens={
            "input": usage.get("input_tokens", 0),
            "output": usage.get("output_tokens", 0),
            "cache_read": usage.get("cache_read_input_tokens", 0),
            "cache_write": usage.get("cache_creation_input_tokens", 0),
        },
        cost_usd=(result or {}).get("total_cost_usd", 0.0),
        turns=(result or {}).get("num_turns", 0),
        is_error=result is None or bool(result.get("is_error")),
        models=(result or {}).get("modelUsage", {}),
        skills=skills,
        subagents=subagents,
        files_read=sorted(files),
        harness_files_read=sum(1 for f in files if f.startswith("harness:")),
        tools=tools,
        permission_denials=len((result or {}).get("permission_denials", [])),
        result_text=(result or {}).get("result", ""),
        api_error_status=(result or {}).get("api_error_status"),
        requests=len(responses),
        tool_result_chars=result_chars,
        subagent_reports=reports,
        subagent_async=launched_async,
    )


def _tool_result_text(body) -> str:
    if isinstance(body, list):
        return "".join(part.get("text", "") for part in body if isinstance(part, dict))
    return body or ""


def _subagent_reports(event: dict, tool_names: dict[str, str]) -> list[str]:
    content = event.get("message", {}).get("content")
    if event.get("parent_tool_use_id") or not isinstance(content, list):
        return []
    return [
        _tool_result_text(block.get("content"))
        for block in content
        if isinstance(block, dict) and block.get("type") == "tool_result"
        and tool_names.get(block.get("tool_use_id", "")) in ("Agent", "Task")
    ]


def _add_result_chars(event: dict, tool_names: dict[str, str], totals: dict[str, int]) -> None:
    """Caracteres que cada herramienta devolvió al contexto: lo que se escribe en caché."""
    content = event.get("message", {}).get("content")
    if not isinstance(content, list):
        return
    for block in content:
        if not isinstance(block, dict) or block.get("type") != "tool_result":
            continue
        size = len(_tool_result_text(block.get("content")))
        name = tool_names.get(block.get("tool_use_id", ""), "?")
        totals[name] = totals.get(name, 0) + size


def _relative(file_path: str, workdir: Path, harness: Path) -> str:
    path = Path(file_path)
    if not path.is_absolute():
        return path.as_posix()
    for base, prefix in ((workdir, ""), (harness, "harness:")):
        try:
            return prefix + path.relative_to(base).as_posix()
        except ValueError:
            pass
    return path.as_posix()


def parse_hook_log(text: str) -> dict[str, int]:
    """Bloqueos por modo del registro de los hooks (`modo\\tdecisión\\tdetalle`)."""
    blocks: dict[str, int] = {}
    for line in text.splitlines():
        parts = line.split("\t")
        if len(parts) >= 2 and parts[1] == "block":
            blocks[parts[0]] = blocks.get(parts[0], 0) + 1
    return blocks


@dataclass
class AcceptanceResult:
    passed: bool
    checks: dict[str, bool]


CHECK_LINE = re.compile(r"^(PASS|FAIL) ([\w.-]+)")


def parse_acceptance(output: str, exit_code: int) -> AcceptanceResult:
    checks: dict[str, bool] = {}
    for line in output.splitlines():
        match = CHECK_LINE.match(line.strip())
        if match:
            checks[match.group(2)] = match.group(1) == "PASS"
    passed = exit_code == 0 and bool(checks) and all(checks.values())
    return AcceptanceResult(passed=passed, checks=checks)


def spread(values: list[float]) -> dict | None:
    if not values:
        return None
    return {"median": statistics.median(values), "min": min(values), "max": max(values)}


def is_valid(run: dict) -> bool:
    """Una ejecución cortada por la API (límite de uso, 5xx), o una kata de
    subagente sin su reporte, no mide al agente."""
    return run.get("api_error_status") is None and not run.get("invalid")


def cache_slot(kata: Kata, index: int, subagent_model: str | None, label: str) -> Path:
    """Un `CARGO_TARGET_DIR` por configuración, variante y corrida. Dos corridas
    en paralelo que compilan en el mismo directorio se pisan (E0463), y una
    puede correr los tests compilados desde el árbol de la otra."""
    variant = f"{kata.name}@{subagent_model}" if subagent_model else kata.name
    return CACHE / label / variant / str(index)


def pending_indices(existing: list[dict], runs: int) -> list[int]:
    done = {r["index"] for r in existing if is_valid(r)}
    return [i for i in range(1, runs + 1) if i not in done]


def summarize(all_runs: list[dict]) -> dict:
    runs = [r for r in all_runs if is_valid(r)]
    checks: dict[str, list[bool]] = {}
    for run in runs:
        for name, ok in run["acceptance"]["checks"].items():
            checks.setdefault(name, []).append(ok)
    total = len(runs)
    return {
        "runs": total,
        "invalid": len(all_runs) - total,
        "cost_usd": spread([r["cost_usd"] for r in runs]),
        "turns": spread([r["turns"] for r in runs]),
        "requests": spread([r["requests"] for r in runs if "requests" in r]),
        "tool_result_chars": spread([sum(r["tool_result_chars"].values()) for r in runs if "tool_result_chars" in r]),
        "tool_result_chars_by_tool": {
            tool: spread([r["tool_result_chars"].get(tool, 0) for r in runs if "tool_result_chars" in r])
            for tool in sorted({t for r in runs for t in r.get("tool_result_chars", {})})
        },
        "tokens": {k: spread([r["tokens"][k] for r in runs]) for k in ("input", "output", "cache_read", "cache_write")},
        "files_read": spread([len(r["files_read"]) for r in runs]),
        "harness_files_read": spread([r["harness_files_read"] for r in runs]),
        "hook_blocks": spread([sum(r["hook_blocks"].values()) for r in runs]),
        "acceptance": f"{sum(1 for r in runs if r['acceptance']['passed'])}/{total}",
        "checks": {name: f"{sum(oks)}/{total}" for name, oks in checks.items()},
    }


def _fmt(value: dict | None, money: bool = False, thousands: bool = False) -> str:
    if value is None:
        return "—"

    def one(x: float) -> str:
        if money:
            return f"${x:.2f}"
        if thousands:
            return f"{x / 1000:.0f}k"
        return f"{x:g}"

    if value["min"] == value["max"]:
        return one(value["median"])
    return f"{one(value['median'])} ({one(value['min'])}–{one(value['max'])})"


def render_table(results: list[dict]) -> str:
    """Tabla Markdown: mediana y, entre paréntesis, el rango."""
    header = (
        "| Kata | Configuración | Modelo | Aceptación | Costo | Turnos | Peticiones | Entrada | Salida "
        "| Caché leída | Caché escrita | Resultados de herramientas (caracteres) | Archivos leídos | Del harness "
        "| Bloqueos de hooks |\n"
        "|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|"
    )
    rows = [header]
    for result in results:
        config = f"{result['version']}-{result['label']}"
        for kata, data in sorted(result["katas"].items()):
            s = data["summary"]
            tokens = s["tokens"]
            invalid = f" (+{s['invalid']} inválida)" if s.get("invalid") else ""
            rows.append(
                f"| {kata} | {config} | {result['model']} | {s['acceptance']}{invalid} | {_fmt(s['cost_usd'], money=True)} "
                f"| {_fmt(s['turns'])} | {_fmt(s.get('requests'))} | {_fmt(tokens['input'])} "
                f"| {_fmt(tokens['output'], thousands=True)} "
                f"| {_fmt(tokens['cache_read'], thousands=True)} | {_fmt(tokens['cache_write'], thousands=True)} "
                f"| {_fmt(s.get('tool_result_chars'), thousands=True)} | {_fmt(s['files_read'])} | {_fmt(s['harness_files_read'])} | {_fmt(s['hook_blocks'])} |"
            )
    return "\n".join(rows)


# --- Ejecución (sesiones reales) --------------------------------------------


FINDING = re.compile(r"^[-*\s]*`?([A-Z]+-\d+)`?\s*\|\s*([^|]+?)\s*\|\s*(.*)$")


def parse_findings(report: str) -> list[dict]:
    """Hallazgos del formato de los revisores: `ID | ruta:línea | descripción`,
    con las líneas indentadas que lo siguen como detalle."""
    found: list[dict] = []
    current: dict | None = None
    for line in report.splitlines():
        match = FINDING.match(line)
        if match:
            current = {"id": match.group(1), "path": match.group(2), "text": match.group(3)}
            found.append(current)
        elif current is not None and line.startswith((" ", "\t")) and line.strip():
            current["text"] += "\n" + line.strip()
        elif line.strip():
            current = None
    return found


REVIEW_AGENT = "craft-backend:architecture-reviewer"
REVIEW_PROMPT = (
    "Pide al subagente craft-backend:architecture-reviewer que revise el diff actual "
    "(cambios sin commit) antes de que lo suba. Invócalo en primer plano, nunca en "
    "segundo plano. No revises nada tú: tu único paso es invocarlo. Después responde "
    "solo «listo».\n"
)


def review_kata(source: Kata, patch: Path) -> Kata:
    """La revisión del diff de una corrida: la plantilla de la kata con el parche
    aplicado sin commit. Una sesión mínima invoca a `architecture-reviewer` como
    subagente, para que cargue sus skills (con `--agent` no las carga)."""
    return Kata(
        name=f"{source.name}-revision",
        path=source.path,
        prompt=REVIEW_PROMPT,
        template=source.template,
        plugins=["craft-backend"],
        acceptance="python3 -c \"print('PASS reporte')\"",
        setup=[f"git apply {patch}", *source.setup],
        env=dict(source.env),
        timeout_minutes=20,
        subagent=REVIEW_AGENT,
    )


def harness_version() -> str:
    for line in (REPO / "CHANGELOG.md").read_text(encoding="utf-8").splitlines():
        match = re.match(r"^## (\d+\.\d+\.\d+)", line)
        if match:
            return match.group(1)
    return "sin-version"


def plugin_versions() -> dict[str, str]:
    versions = {}
    for manifest in sorted(REPO.glob("plugins/*/.claude-plugin/plugin.json")):
        data = json.loads(manifest.read_text(encoding="utf-8"))
        versions[data["name"]] = data["version"]
    return versions


def copy_template(template: str, target: Path) -> None:
    listed = subprocess.run(
        ["git", "-C", str(REPO), "ls-files", "-z", "--", template],
        capture_output=True, check=True,
    ).stdout.decode().split("\0")
    prefix = Path(template)
    for entry in filter(None, listed):
        source = REPO / entry
        destination = target / Path(entry).relative_to(prefix)
        destination.parent.mkdir(parents=True, exist_ok=True)
        # Sin copy2: mtimes nuevos para que cargo recompile en este directorio.
        shutil.copy(source, destination, follow_symlinks=False)


def project_settings(plugins: list[str], overridden: list[str] | None = None) -> dict:
    """Solo los plugins de la kata: los demás del usuario quedan deshabilitados.
    Un plugin `overridden` se carga desde una copia (`--plugin-dir`), así que
    el del marketplace también se deshabilita."""
    enabled: dict[str, bool] = {}
    user_settings = Path.home() / ".claude" / "settings.json"
    if user_settings.is_file():
        try:
            for key in json.loads(user_settings.read_text(encoding="utf-8")).get("enabledPlugins", {}):
                enabled[key] = False
        except json.JSONDecodeError:
            pass
    for plugin in plugins:
        enabled[f"{plugin}@{MARKETPLACE}"] = plugin not in (overridden or [])
    return {"enabledPlugins": enabled}


def set_frontmatter_model(text: str, model: str) -> str:
    head, sep, body = text.partition("\n---\n")
    lines = [line for line in head.splitlines() if not line.startswith("model:")]
    lines.insert(1 + next(i for i, l in enumerate(lines[1:]) if l.startswith("name:")), f"model: {model}")
    return "\n".join(lines) + sep + body


def plugin_with_model(plugin: str, agent: str, model: str, target: Path) -> Path:
    """Copia del plugin con `model:` fijado en un agente. Los symlinks quedan
    absolutos hacia el repo: así no se copia la plantilla (ni su `target/`)."""
    source = REPO / "plugins" / plugin
    copy = target / plugin
    shutil.copytree(source, copy, symlinks=True, ignore=shutil.ignore_patterns("__pycache__"))
    for link in [p for p in copy.rglob("*") if p.is_symlink()]:
        resolved = (source / link.relative_to(copy)).resolve()
        link.unlink()
        link.symlink_to(resolved)
    agent_file = copy / "agents" / f"{agent}.md"
    agent_file.write_text(set_frontmatter_model(agent_file.read_text(encoding="utf-8"), model), encoding="utf-8")
    return copy


def interleave(katas: list, models: list[str], indices: list[int]) -> list[tuple]:
    """Las configuraciones alternadas en la misma ventana: A1, B1, A2, B2…"""
    return [(kata, model, index) for kata in katas for index in indices for model in models]


def save_diff(workdir: Path, target: Path) -> None:
    """El cambio de la corrida contra su commit inicial, archivos nuevos incluidos."""
    git(workdir, "add", "-A")
    patch = subprocess.run(["git", "diff", "--cached", "--binary"], cwd=workdir,
                           capture_output=True, check=True).stdout
    target.write_bytes(patch)
    git(workdir, "reset", "-q")


def git(workdir: Path, *args: str) -> None:
    subprocess.run(
        ["git", "-c", "user.email=evals@ai-harness", "-c", "user.name=evals", *args],
        cwd=workdir, check=True, capture_output=True,
    )


def claude_command(kata: Kata, model: str, budget: float, plugin_dir: Path | None = None) -> list[str]:
    command = [
        "claude", "-p", kata.prompt,
        "--output-format", "stream-json", "--verbose",
        "--model", model,
        "--permission-mode", "dontAsk",
        "--allowedTools", " ".join(ALLOWED_TOOLS + kata.allowed_tools),
        "--no-session-persistence",
        "--max-budget-usd", str(budget),
    ]
    if kata.agent:
        command += ["--agent", kata.agent]
    if plugin_dir:
        command += ["--plugin-dir", str(plugin_dir)]
    return command


def run_once(kata: Kata, index: int, model: str, label: str, budget: float, keep: bool,
             subagent_model: str | None = None) -> dict:
    stamp = dt.datetime.now().strftime("%Y%m%d-%H%M%S")
    variant = f"{kata.name}@{subagent_model}" if subagent_model else kata.name
    run_dir = CACHE / "runs" / f"{harness_version()}-{label}" / f"{variant}-{index}-{stamp}"
    workdir = CACHE / "work" / f"{variant}-{label}-{index}-{stamp}"
    run_dir.mkdir(parents=True, exist_ok=True)
    workdir.mkdir(parents=True)

    copy_template(kata.template, workdir)
    plugin_dir = None
    overridden: list[str] = []
    if kata.subagent and subagent_model:
        plugin, agent = kata.subagent.split(":")
        plugin_dir = plugin_with_model(plugin, agent, subagent_model, run_dir / "plugin")
        overridden = [plugin]
    (workdir / ".claude").mkdir(exist_ok=True)
    (workdir / ".claude" / "settings.json").write_text(
        json.dumps(project_settings(kata.plugins, overridden), indent=2) + "\n", encoding="utf-8"
    )
    git(workdir, "init", "-q")
    git(workdir, "add", "-A")
    git(workdir, "commit", "-qm", "plantilla")

    cache = cache_slot(kata, index, subagent_model, label)
    cache.mkdir(parents=True, exist_ok=True)
    hook_log = run_dir / "hooks.log"
    env = {k: v for k, v in os.environ.items() if k != "ANTHROPIC_API_KEY"}
    env.update(kata.expanded_env(cache))
    for var in HOOK_LOG_VARS:
        env[var] = str(hook_log)

    for command in kata.expanded_setup():
        log(kata, index, f"setup: {command}")
        subprocess.run(command, shell=True, cwd=workdir, env=env, check=True,
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    hook_log.write_text("", encoding="utf-8")

    command = claude_command(kata, model, budget, plugin_dir)
    log(kata, index, f"agente ({model})")
    started = dt.datetime.now()
    stream_file = run_dir / "stream.jsonl"
    timed_out = False
    with stream_file.open("w", encoding="utf-8") as out, (run_dir / "stderr.txt").open("w") as err:
        try:
            subprocess.run(command, cwd=workdir, env=env, stdout=out, stderr=err,
                           timeout=kata.timeout_minutes * 60)
        except subprocess.TimeoutExpired:
            timed_out = True
    duration = (dt.datetime.now() - started).total_seconds()
    metrics = parse_stream(stream_file.read_text(encoding="utf-8").splitlines(), workdir, REPO)

    diff = subprocess.run(["git", "status", "--porcelain"], cwd=workdir, capture_output=True, text=True).stdout
    (run_dir / "changes.txt").write_text(diff, encoding="utf-8")
    save_diff(workdir, run_dir / "diff.patch")

    shutil.copytree(kata.path / "acceptance", workdir / "acceptance", dirs_exist_ok=True)
    log(kata, index, "aceptación")
    env["EVAL_RESULT_TEXT"] = str(run_dir / "result.txt")
    report = "\n\n".join(metrics.subagent_reports) if kata.subagent else metrics.result_text
    (run_dir / "result.txt").write_text(report, encoding="utf-8")
    acceptance = subprocess.run(shlex.split(kata.acceptance), cwd=workdir, env=env,
                                capture_output=True, text=True, timeout=1800)
    (run_dir / "acceptance.txt").write_text(acceptance.stdout + acceptance.stderr, encoding="utf-8")
    verdict = parse_acceptance(acceptance.stdout, acceptance.returncode)

    if not keep:
        shutil.rmtree(workdir, ignore_errors=True)
    run = {
        "index": index,
        "duration_s": round(duration),
        "timed_out": timed_out,
        "is_error": metrics.is_error,
        "cost_usd": metrics.cost_usd,
        "turns": metrics.turns,
        "tokens": metrics.tokens,
        "models": {m: {k: u.get(k) for k in ("inputTokens", "outputTokens", "cacheReadInputTokens",
                                             "cacheCreationInputTokens", "costUSD")}
                   for m, u in metrics.models.items()},
        "skills": metrics.skills,
        "subagents": metrics.subagents,
        "files_read": metrics.files_read,
        "harness_files_read": metrics.harness_files_read,
        "tools": metrics.tools,
        "permission_denials": metrics.permission_denials,
        "api_error_status": metrics.api_error_status,
        "requests": metrics.requests,
        "tool_result_chars": metrics.tool_result_chars,
        "hook_blocks": parse_hook_log(hook_log.read_text(encoding="utf-8")),
        "acceptance": {"passed": verdict.passed, "checks": verdict.checks},
        "findings": parse_findings(report),
        "subagent_model": subagent_model,
        "invalid": _invalid_reason(kata, metrics),
        "raw": str(run_dir),
    }
    log(kata, index, f"listo: aceptación={'sí' if verdict.passed else 'no'} costo=${metrics.cost_usd:.2f} "
                     f"turnos={metrics.turns}")
    return run


def _invalid_reason(kata: Kata, metrics: SessionMetrics) -> str | None:
    if not kata.subagent:
        return None
    if metrics.subagent_async:
        return "subagente en segundo plano: su reporte no llegó como tool_result"
    if not metrics.subagent_reports:
        return "sin reporte del subagente"
    return None


def log(kata: Kata, index: int, message: str) -> None:
    print(f"[{dt.datetime.now():%H:%M:%S}] {kata.name}#{index}: {message}", file=sys.stderr, flush=True)


def claude_version() -> str:
    try:
        return subprocess.run(["claude", "--version"], capture_output=True, text=True).stdout.strip()
    except OSError:
        return "desconocida"


def command_run(args: argparse.Namespace) -> int:
    paths = [EVALS / k for k in args.kata] if args.kata else discover_katas()
    katas = [load_kata(p) for p in paths]
    RESULTS.mkdir(parents=True, exist_ok=True)
    output = RESULTS / f"{harness_version()}-{args.label}.json"
    data = json.loads(output.read_text(encoding="utf-8")) if output.is_file() else {"katas": {}}
    if data.get("model", args.model) != args.model:
        raise SystemExit(f"{output.name} es de {data['model']}: usa otra etiqueta")
    # Retoma: conserva las ejecuciones válidas y corre solo las que faltan o
    # quedaron inválidas (salvo --fresh).
    runs: dict[str, list[dict]] = {}
    pending: dict[str, list[int]] = {}
    jobs = []
    models = args.subagent_model or [None]
    for kata, submodel, index in interleave(katas, models, list(range(1, args.runs + 1))):
        key = f"{kata.name}@{submodel}" if submodel else kata.name
        if key not in runs:
            existing = [] if args.fresh else data["katas"].get(key, {}).get("runs", [])
            pending[key] = pending_indices(existing, args.runs)
            runs[key] = [r for r in existing if r["index"] not in pending[key]]
        if index in pending[key]:
            jobs.append((key, kata, index, submodel))
    execute(jobs, runs, args)
    save_results(output, data, runs, args)
    return 0


def execute(jobs: list[tuple], runs: dict[str, list[dict]], args: argparse.Namespace) -> None:
    """Corre en el orden de `jobs` (intercalado si hay varias configuraciones)."""
    with concurrent.futures.ThreadPoolExecutor(max_workers=args.parallel) as pool:
        futures = {pool.submit(run_once, kata, index, args.model, args.label, args.budget, args.keep, submodel): key
                   for key, kata, index, submodel in jobs}
        for future in concurrent.futures.as_completed(futures):
            runs[futures[future]].append(future.result())


def save_results(output: Path, data: dict, runs: dict[str, list[dict]], args: argparse.Namespace) -> None:
    data.update({
        "version": harness_version(),
        "label": args.label,
        "model": args.model,
        "plugins": plugin_versions(),
        "claude": claude_version(),
        "date": dt.date.today().isoformat(),
    })
    for name, kata_runs in runs.items():
        kata_runs.sort(key=lambda r: r["index"])
        data["katas"][name] = {"runs": kata_runs, "summary": summarize(kata_runs)}
    output.write_text(json.dumps(data, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(render_table([data]))
    print(f"\n{output.relative_to(REPO)}")


def command_review(args: argparse.Namespace) -> int:
    """Revisa con `architecture-reviewer` el diff final de cada corrida de un resultado."""
    source = json.loads(Path(args.results).read_text(encoding="utf-8"))
    RESULTS.mkdir(parents=True, exist_ok=True)
    output = RESULTS / f"{harness_version()}-{args.label}.json"
    data = json.loads(output.read_text(encoding="utf-8")) if output.is_file() else {"katas": {}}
    runs: dict[str, list[dict]] = {}
    jobs: list[tuple[Kata, int]] = []
    for kata_name in args.kata:
        kata = load_kata(EVALS / kata_name)
        name = f"{kata_name}-revision"
        existing = [] if args.fresh else data["katas"].get(name, {}).get("runs", [])
        done = {r["index"] for r in existing if is_valid(r)}
        runs[name] = [r for r in existing if r["index"] in done]
        for run in source["katas"][kata_name]["runs"]:
            patch = Path(run["raw"]) / "diff.patch"
            if not is_valid(run) or run["index"] in done:
                continue
            if not patch.is_file():
                raise SystemExit(f"{kata_name}#{run['index']}: no hay {patch}; vuelve a correr la kata")
            jobs.append((name, review_kata(kata, patch), run["index"], args.subagent_model))
    execute(jobs, runs, args)
    for name, kata_runs in runs.items():
        for run in kata_runs:
            run["source"] = f"{Path(args.results).name}#{run['index']}"
    save_results(output, data, runs, args)
    return 0


def command_table(args: argparse.Namespace) -> int:
    results = [json.loads(Path(p).read_text(encoding="utf-8")) for p in args.results]
    print(render_table(results))
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="command", required=True)
    run = sub.add_parser("run", help="corre katas con sesiones reales")
    run.add_argument("--model", required=True, help="modelo fijado de forma explícita (p. ej. claude-opus-5-5)")
    run.add_argument("--label", required=True, help="etiqueta de la configuración (p. ej. base)")
    run.add_argument("--kata", action="append", help="kata a correr (por defecto, todas)")
    run.add_argument("--runs", type=int, default=3)
    run.add_argument("--parallel", type=int, default=1)
    run.add_argument("--budget", type=float, default=15.0, help="tope en USD por ejecución")
    run.add_argument("--fresh", action="store_true", help="repite también las ejecuciones válidas")
    run.add_argument("--subagent-model", action="append",
                     help="modelo del subagente de la kata (repetible: se intercalan en la misma ventana)")
    run.add_argument("--keep", action="store_true", help="conserva el directorio de trabajo")
    run.set_defaults(func=command_run)
    review = sub.add_parser("review", help="revisa con architecture-reviewer el diff de cada corrida")
    review.add_argument("results", help="resultado cuyas corridas se revisan (evals/results/….json)")
    review.add_argument("--kata", action="append", required=True)
    review.add_argument("--model", required=True, help="modelo de la sesión mínima que delega")
    review.add_argument("--subagent-model", required=True, help="modelo del revisor")
    review.add_argument("--label", required=True)
    review.add_argument("--parallel", type=int, default=1)
    review.add_argument("--budget", type=float, default=5.0)
    review.add_argument("--fresh", action="store_true")
    review.add_argument("--keep", action="store_true")
    review.set_defaults(func=command_review)
    table = sub.add_parser("table", help="tabla Markdown de uno o más resultados")
    table.add_argument("results", nargs="+")
    table.set_defaults(func=command_table)
    args = parser.parse_args(argv)
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())
