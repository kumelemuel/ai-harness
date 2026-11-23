#!/usr/bin/env python3
"""Hooks de craft-backend (VER-1): lo que no se verifica se erosiona.

session-start  Al abrir la sesión: si `CRAFT_BACKEND_BASELINE_INIT=1` está
           activa, avisa al usuario y al agente que el trinquete está abierto.
pre-edit   Antes de editar `tools/architecture/baseline.txt`: bloquea la
           edición si agrega entradas o sube un conteo (la línea base solo
           se achica).
post-edit  Tras editar un .rs o Cargo.toml en un workspace Rust con
           `tools/architecture`: formatea el archivo y corre los tests de
           arquitectura. Si fallan, devuelve las violaciones al agente.
stop       Antes de que el agente termine, si hay cambios Rust sin commit:
           clippy -D warnings (solo crates que siguen la arquitectura) +
           cargo test --workspace. Si fallan, no lo deja terminar.

Fuera de un workspace con `tools/architecture` no hace nada.

Variables de entorno:
  CRAFT_BACKEND_HOOKS=0       desactiva todos los hooks
  CRAFT_BACKEND_STOP_GATE=0   desactiva solo el hook de Stop
  CRAFT_BACKEND_BASELINE_INIT=1  permite agrandar baseline.txt (llenado inicial o
                                 sincronización); se lee al abrir la sesión
  CRAFT_BACKEND_HOOKS_LOG=…   ruta de un archivo donde registrar cada decisión

Códigos de salida (contrato de hooks de Claude Code):
  0  sin observaciones
  2  bloquea / devuelve stderr al agente
"""

from __future__ import annotations

import json
import os
import re
import subprocess
import sys

if sys.version_info < (3, 11):
    # Entorno incompleto: avisar sin bloquear al agente (como con FileNotFoundError).
    print(
        "craft-backend: los hooks requieren Python 3.11 o superior; hay "
        + ".".join(map(str, sys.version_info[:3]))
        + ". Se omiten.",
        file=sys.stderr,
    )
    sys.exit(0)

import tomllib
from pathlib import Path

ARCH_PACKAGE = "architecture"
DEFAULT_EDITION = "2024"
MAX_FEEDBACK_LINES = 60


def read_input() -> dict:
    try:
        return json.load(sys.stdin)
    except (json.JSONDecodeError, ValueError):
        return {}


def workspace_root(start: Path) -> Path | None:
    """El `Cargo.toml` con `[workspace]` más cercano hacia arriba."""
    for directory in [start, *start.parents]:
        manifest = directory / "Cargo.toml"
        if manifest.is_file() and "[workspace]" in manifest.read_text(errors="ignore"):
            return directory
    return None


def has_architecture_tests(root: Path) -> bool:
    manifest = root / "tools" / ARCH_PACKAGE / "Cargo.toml"
    return manifest.is_file()


LAYER_DIRS = ("domain", "application", "infrastructure", "api")


def package_name(crate_dir: Path) -> str | None:
    manifest = crate_dir / "Cargo.toml"
    if not manifest.is_file():
        return None
    try:
        return tomllib.loads(manifest.read_text())["package"]["name"]
    except (tomllib.TOMLDecodeError, KeyError):
        return None


def rust_edition(root: Path) -> str:
    """La edición del workspace: `[workspace.package].edition` o, si el workspace no
    hereda la edición, `[package].edition` del manifiesto raíz. `DEFAULT_EDITION`
    si no declara ninguna o el manifiesto no parsea."""
    try:
        manifest = tomllib.loads((root / "Cargo.toml").read_text())
    except (OSError, tomllib.TOMLDecodeError):
        return DEFAULT_EDITION
    for section in (manifest.get("workspace", {}).get("package", {}), manifest.get("package", {})):
        edition = section.get("edition")
        if isinstance(edition, str):
            return edition
    return DEFAULT_EDITION


def architected_packages(root: Path) -> list[str]:
    """Crates que siguen la arquitectura (un crate por capa). A diferencia de
    `tools/architecture`, que también clasifica los contextos legacy y `shared/`,
    los contextos legacy (`contexts/{ctx}/Cargo.toml`) y los crates fuera de la
    convención quedan fuera del gate de clippy: su código con warnings no bloquea."""
    dirs: list[Path] = [root / "kernel", root / "app"]
    for group in ("edge", "infrastructure", "apps", "tools"):
        base = root / group
        if base.is_dir():
            dirs.extend(sorted(p for p in base.iterdir() if p.is_dir()))
    contexts = root / "contexts"
    if contexts.is_dir():
        for context in sorted(p for p in contexts.iterdir() if p.is_dir()):
            if (context / "Cargo.toml").is_file():
                continue  # legacy
            dirs.extend(context / layer for layer in LAYER_DIRS)
    return [name for name in map(package_name, dirs) if name]


def run(command: list[str], cwd: Path, timeout: int) -> subprocess.CompletedProcess:
    return subprocess.run(
        command,
        cwd=cwd,
        capture_output=True,
        text=True,
        timeout=timeout,
        env={**os.environ, "CARGO_TERM_COLOR": "never"},
    )


def violations(output: str) -> str:
    """Extrae las líneas de violación (`HEX-…` y `  - …`) de la salida de cargo test."""
    lines = [
        line
        for line in output.splitlines()
        if line.startswith(("HEX-", "ERR-", "TDD-", "VER-", "RS-", "  - "))
    ]
    return "\n".join(lines[:MAX_FEEDBACK_LINES])


def tail(output: str, lines: int = MAX_FEEDBACK_LINES) -> str:
    return "\n".join(output.strip().splitlines()[-lines:])


def log(mode: str, decision: str, detail: str = "") -> None:
    target = os.environ.get("CRAFT_BACKEND_HOOKS_LOG")
    if target:
        with open(target, "a", encoding="utf-8") as handle:
            handle.write(f"{mode}\t{decision}\t{detail}\n")


def feedback(message: str) -> int:
    print(message, file=sys.stderr)
    return 2


def post_edit(payload: dict) -> int:
    file_path = (payload.get("tool_input") or {}).get("file_path")
    if not file_path:
        return 0
    path = Path(file_path)
    if path.suffix != ".rs" and path.name != "Cargo.toml":
        return 0

    root = workspace_root(path.parent)
    if root is None or not has_architecture_tests(root):
        return 0

    if path.suffix == ".rs" and path.is_file():
        # Best effort: si no parsea, el compilador lo dirá con más contexto.
        run(["rustfmt", "--edition", rust_edition(root), str(path)], root, timeout=30)

    result = run(
        ["cargo", "test", "--quiet", "-p", ARCH_PACKAGE, "--no-fail-fast"],
        root,
        timeout=170,
    )
    if result.returncode == 0:
        return 0

    found = violations(result.stdout + result.stderr)
    if found:
        return feedback(
            "Tests de arquitectura (VER-1) en rojo tras editar "
            f"{path.relative_to(root) if path.is_relative_to(root) else path}:\n\n"
            f"{found}\n\n"
            "Corrige el código para cumplir la regla citada; no modifiques "
            "tools/architecture para que pase. La única edición permitida es "
            "borrar o actualizar en baseline.txt las líneas que el test pida."
        )
    return feedback(
        "No se pudieron ejecutar los tests de arquitectura:\n\n"
        + tail(result.stderr or result.stdout)
    )


BASELINE_NAME = "baseline.txt"


def baseline_entries(text: str) -> dict[tuple[str, str], int]:
    """`(test, clave) → conteo` de una línea base (`<test> <clave> [n]`)."""
    entries: dict[tuple[str, str], int] = {}
    for line in text.splitlines():
        parts = line.split()
        if len(parts) < 2 or parts[0].startswith("#"):
            continue
        entries[(parts[0], parts[1])] = baseline_count(parts[2] if len(parts) > 2 else "")
    return entries


def baseline_count(text: str) -> int:
    """El conteo como lo lee el trinquete en Rust (`parse::<usize>()`, 1 si
    falla): dígitos ASCII con un `+` opcional."""
    return int(text) if re.fullmatch(r"\+?[0-9]+", text) else 1


def baseline_growth(old: str, new: str) -> list[str]:
    """Entradas de `new` que no están en `old` o con un conteo mayor."""
    before = baseline_entries(old)
    grown = []
    for (test, key), count in baseline_entries(new).items():
        if count > before.get((test, key), 0):
            grown.append(f"{test} {key}" + (f" {count}" if count != 1 else ""))
    return grown


def edited_text(tool: str, tool_input: dict, current: str) -> str | None:
    """Contenido del archivo después de la edición, o `None` si no se puede saber."""
    if tool == "Write":
        return tool_input.get("content")
    edits = tool_input.get("edits") if tool == "MultiEdit" else [tool_input]
    text = current
    for edit in edits or []:
        old, new = edit.get("old_string", ""), edit.get("new_string", "")
        if old not in text:
            return None
        text = text.replace(old, new) if edit.get("replace_all") else text.replace(old, new, 1)
    return text


def pre_edit(payload: dict) -> int:
    """Bloquea una edición que agrande `tools/architecture/baseline.txt` (VER-1)."""
    tool_input = payload.get("tool_input") or {}
    path = Path(tool_input.get("file_path") or "")
    if not path.is_absolute():
        path = Path(payload.get("cwd") or ".") / path
    path = path.resolve()
    tools = path.parent
    if (path.name != BASELINE_NAME or tools.name != ARCH_PACKAGE
            or tools.parent.name != "tools" or not (tools / "Cargo.toml").is_file()):
        return 0
    if os.environ.get("CRAFT_BACKEND_BASELINE_INIT") == "1":
        return 0
    current = path.read_text(encoding="utf-8") if path.is_file() else ""
    new = edited_text(payload.get("tool_name", ""), tool_input, current)
    if new is None:
        return feedback(
            "craft-backend no pudo calcular el resultado de esta edición de baseline.txt "
            "(el texto a reemplazar no coincide), así que no puede comprobar que la línea "
            "base no crezca. Usa Write con el contenido completo."
        )
    grown = baseline_growth(current, new)
    if not grown:
        return 0
    return feedback(
        "baseline.txt solo se achica (VER-1): esta edición agrega o sube entradas:\n\n"
        + "\n".join(f"  - {entry}" for entry in grown)
        + "\n\nCorrige la violación en el código. Borrar o bajar entradas sí está "
        "permitido. No edites baseline.txt por shell: si crees que la entrada es "
        "legítima, detente y avisa. Para el llenado inicial de una adopción o una "
        "sincronización con el harness, el usuario exporta "
        "CRAFT_BACKEND_BASELINE_INIT=1 antes de abrir la sesión: el hook hereda el "
        "entorno de Claude Code al arrancar, así que exportarla a mitad de sesión no "
        "sirve. Para retomar esta sesión con la variable, el usuario la cierra y "
        "corre `CRAFT_BACKEND_BASELINE_INIT=1 claude --continue`."
    )


def session_start(payload: dict) -> int:
    """Avisa que la sesión tiene el trinquete abierto (`CRAFT_BACKEND_BASELINE_INIT=1`)."""
    if os.environ.get("CRAFT_BACKEND_BASELINE_INIT") != "1":
        return 0
    root = workspace_root(Path(payload.get("cwd") or ".").resolve())
    if root is None or not has_architecture_tests(root):
        return 0
    message = (
        "craft-backend: CRAFT_BACKEND_BASELINE_INIT=1 está activa. El hook deja agrandar "
        "tools/architecture/baseline.txt en esta sesión (trinquete abierto). Úsala solo "
        "para el llenado inicial de una adopción o una sincronización con el harness, y "
        "cierra la sesión al terminar."
    )
    context = (
        message + " Fuera de ese encargo, no agregues ni subas entradas de baseline.txt "
        "aunque el hook no lo bloquee: detente y avisa."
    )
    print(json.dumps({
        "systemMessage": message,
        "hookSpecificOutput": {"hookEventName": "SessionStart", "additionalContext": context},
    }, ensure_ascii=False))
    return 0


def rust_changes(root: Path) -> bool:
    result = run(
        ["git", "status", "--porcelain", "--", "*.rs", "*Cargo.toml"],
        root,
        timeout=30,
    )
    return result.returncode == 0 and bool(result.stdout.strip())


def stop(payload: dict) -> int:
    if os.environ.get("CRAFT_BACKEND_STOP_GATE") == "0":
        return 0
    # Si ya bloqueamos una vez en este turno, dejamos terminar: evita bucles.
    if payload.get("stop_hook_active"):
        return 0

    cwd = Path(payload.get("cwd") or os.getcwd())
    root = workspace_root(cwd)
    if root is None or not has_architecture_tests(root) or not rust_changes(root):
        return 0

    packages = [arg for name in architected_packages(root) for arg in ("-p", name)]
    checks = [
        ("clippy", ["cargo", "clippy", "--quiet", "--no-deps", "--all-targets", *packages, "--", "-D", "warnings"]),
        ("tests", ["cargo", "test", "--quiet", "--workspace", "--no-fail-fast"]),
    ]
    for name, command in checks:
        result = run(command, root, timeout=420)
        if result.returncode != 0:
            output = result.stdout + result.stderr
            detail = violations(output) or tail(output)
            return feedback(
                f"Antes de terminar: `{' '.join(command)}` falla ({name}).\n\n"
                f"{detail}\n\n"
                "Si el fallo lo causó tu cambio, corrígelo antes de terminar (VER-1). "
                "Si el rojo es intencional (fase roja de TDD) o crees que el test está mal, "
                "no lo modifiques: explica el estado y por qué, y termina. Este control "
                "bloquea una sola vez por turno (TDD-1)."
            )
    return 0


def main() -> int:
    if os.environ.get("CRAFT_BACKEND_HOOKS") == "0":
        return 0
    mode = sys.argv[1] if len(sys.argv) > 1 else ""
    payload = read_input()
    try:
        handlers = {
            "session-start": session_start,
            "pre-edit": pre_edit,
            "post-edit": post_edit,
            "stop": stop,
        }
        if mode in handlers:
            code = handlers[mode](payload)
            target = (payload.get("tool_input") or {}).get("file_path") or payload.get("cwd", "")
            log(mode, "block" if code == 2 else "pass", target)
            return code
    except subprocess.TimeoutExpired as error:
        print(f"craft-backend: `{' '.join(error.cmd)}` excedió el tiempo límite", file=sys.stderr)
        return 0
    except FileNotFoundError as error:
        # cargo/rustfmt/git no instalados: no bloquear al agente por el entorno.
        print(f"craft-backend: herramienta no encontrada: {error.filename}", file=sys.stderr)
        return 0
    return 0


if __name__ == "__main__":
    sys.exit(main())
