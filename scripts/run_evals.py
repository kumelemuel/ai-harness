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
  agent           corre la sesión como ese subagente (`--agent`), p. ej. para
                  medir un revisor con un modelo dado
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


def parse_stream(lines: list[str], workdir: Path, harness: Path) -> SessionMetrics:
    """Interpreta `claude -p --output-format stream-json --verbose`.

    El consumo sale del evento `result`, que suma la sesión completa (también
    los subagentes). Herramientas, skills y archivos leídos salen de los
    `tool_use` de los mensajes `assistant`, incluidos los de subagentes
    (`parent_tool_use_id`).
    """
    result: dict | None = None
    tools: dict[str, int] = {}
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
        if event.get("type") != "assistant":
            continue
        for block in event.get("message", {}).get("content", []):
            if block.get("type") != "tool_use":
                continue
            name = block.get("name", "")
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
    )


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
    """Una ejecución cortada por la API (límite de uso, 5xx) no mide al agente."""
    return run.get("api_error_status") is None


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
        "| Kata | Configuración | Modelo | Aceptación | Costo | Turnos | Entrada | Salida "
        "| Caché leída | Caché escrita | Archivos leídos | Del harness | Bloqueos de hooks |\n"
        "|---|---|---|---|---|---|---|---|---|---|---|---|---|"
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
                f"| {_fmt(s['turns'])} | {_fmt(tokens['input'])} | {_fmt(tokens['output'], thousands=True)} "
                f"| {_fmt(tokens['cache_read'], thousands=True)} | {_fmt(tokens['cache_write'], thousands=True)} "
                f"| {_fmt(s['files_read'])} | {_fmt(s['harness_files_read'])} | {_fmt(s['hook_blocks'])} |"
            )
    return "\n".join(rows)


# --- Ejecución (sesiones reales) --------------------------------------------


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


def project_settings(plugins: list[str]) -> dict:
    """Solo los plugins de la kata: los demás del usuario quedan deshabilitados."""
    enabled: dict[str, bool] = {}
    user_settings = Path.home() / ".claude" / "settings.json"
    if user_settings.is_file():
        try:
            for key in json.loads(user_settings.read_text(encoding="utf-8")).get("enabledPlugins", {}):
                enabled[key] = False
        except json.JSONDecodeError:
            pass
    for plugin in plugins:
        enabled[f"{plugin}@{MARKETPLACE}"] = True
    return {"enabledPlugins": enabled}


def git(workdir: Path, *args: str) -> None:
    subprocess.run(
        ["git", "-c", "user.email=evals@ai-harness", "-c", "user.name=evals", *args],
        cwd=workdir, check=True, capture_output=True,
    )


def claude_command(kata: Kata, model: str, budget: float) -> list[str]:
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
    return command


def run_once(kata: Kata, index: int, model: str, label: str, budget: float, keep: bool) -> dict:
    stamp = dt.datetime.now().strftime("%Y%m%d-%H%M%S")
    run_dir = CACHE / "runs" / f"{harness_version()}-{label}" / f"{kata.name}-{index}-{stamp}"
    workdir = CACHE / "work" / f"{kata.name}-{label}-{index}-{stamp}"
    run_dir.mkdir(parents=True, exist_ok=True)
    workdir.mkdir(parents=True)

    copy_template(kata.template, workdir)
    (workdir / ".claude").mkdir(exist_ok=True)
    (workdir / ".claude" / "settings.json").write_text(
        json.dumps(project_settings(kata.plugins), indent=2) + "\n", encoding="utf-8"
    )
    git(workdir, "init", "-q")
    git(workdir, "add", "-A")
    git(workdir, "commit", "-qm", "plantilla")

    cache = CACHE / kata.name / str(index)
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

    command = claude_command(kata, model, budget)
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

    shutil.copytree(kata.path / "acceptance", workdir / "acceptance", dirs_exist_ok=True)
    log(kata, index, "aceptación")
    env["EVAL_RESULT_TEXT"] = str(run_dir / "result.txt")
    (run_dir / "result.txt").write_text(metrics.result_text, encoding="utf-8")
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
        "hook_blocks": parse_hook_log(hook_log.read_text(encoding="utf-8")),
        "acceptance": {"passed": verdict.passed, "checks": verdict.checks},
        "raw": str(run_dir),
    }
    log(kata, index, f"listo: aceptación={'sí' if verdict.passed else 'no'} costo=${metrics.cost_usd:.2f} "
                     f"turnos={metrics.turns}")
    return run


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
    jobs = []
    for kata in katas:
        existing = [] if args.fresh else data["katas"].get(kata.name, {}).get("runs", [])
        indices = pending_indices(existing, args.runs)
        runs[kata.name] = [r for r in existing if r["index"] not in indices]
        jobs += [(kata, i) for i in indices]
    with concurrent.futures.ThreadPoolExecutor(max_workers=args.parallel) as pool:
        futures = {pool.submit(run_once, k, i, args.model, args.label, args.budget, args.keep): k
                   for k, i in jobs}
        for future in concurrent.futures.as_completed(futures):
            runs[futures[future].name].append(future.result())

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
    run.add_argument("--keep", action="store_true", help="conserva el directorio de trabajo")
    run.set_defaults(func=command_run)
    table = sub.add_parser("table", help="tabla Markdown de uno o más resultados")
    table.add_argument("results", nargs="+")
    table.set_defaults(func=command_table)
    args = parser.parse_args(argv)
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())
