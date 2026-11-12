#!/usr/bin/env python3
"""Hooks de craft-web: lo que no se verifica se erosiona (VER-1, WEB-*).

post-edit  Tras editar código o contenido de un sitio craft-web (Astro con
           src/routes.ts): formatea el archivo con Prettier y corre las pruebas
           unitarias (rutas, seguridad, formulario). Si fallan, devuelve el detalle
           al agente.
stop       Antes de que el agente termine, si hay cambios sin commit: lint,
           astro check y pruebas unitarias. Si fallan, no lo deja terminar (una
           vez por turno). Los e2e y Lighthouse quedan para CI: necesitan un build.

Fuera de un proyecto craft-web no hace nada. Usa el Node de `.nvmrc` si está
instalado con nvm.

Variables de entorno:
  CRAFT_WEB_HOOKS=0        desactiva ambos hooks
  CRAFT_WEB_STOP_GATE=0    desactiva solo el hook de Stop
  CRAFT_WEB_HOOKS_LOG=…    archivo donde registrar cada decisión

Códigos de salida: 0 sin observaciones · 2 bloquea / devuelve stderr al agente.
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
        "craft-web: los hooks requieren Python 3.11 o superior; hay "
        + ".".join(map(str, sys.version_info[:3]))
        + ". Se omiten.",
        file=sys.stderr,
    )
    sys.exit(0)

from pathlib import Path

MAX_FEEDBACK_LINES = 60
EDITABLE_SUFFIXES = {".astro", ".ts", ".tsx", ".js", ".mjs", ".cjs", ".css", ".md", ".mdx", ".json"}
WATCHED = ("src", "tests", "public", "astro.config.mjs", "astro.config.ts", "package.json")


def read_input() -> dict:
    try:
        return json.load(sys.stdin)
    except (json.JSONDecodeError, ValueError):
        return {}


def project_root(start: Path) -> Path | None:
    """El directorio más cercano hacia arriba con astro.config.* y src/routes.ts."""
    for directory in [start, *start.parents]:
        has_config = any((directory / f"astro.config.{ext}").is_file() for ext in ("mjs", "ts", "js"))
        if has_config and (directory / "src" / "routes.ts").is_file():
            return directory
    return None


def node_env(root: Path) -> dict[str, str]:
    """PATH con el Node de `.nvmrc` primero, si nvm lo tiene instalado."""
    env = {**os.environ, "CI": "1", "FORCE_COLOR": "0", "NO_COLOR": "1"}
    nvmrc = root / ".nvmrc"
    versions = Path(os.environ.get("NVM_DIR", Path.home() / ".nvm")) / "versions" / "node"
    if nvmrc.is_file() and versions.is_dir():
        wanted = nvmrc.read_text().strip().lstrip("v")
        candidates = sorted(
            (p for p in versions.iterdir() if p.name.lstrip("v").startswith(f"{wanted}.") or p.name.lstrip("v") == wanted),
            key=lambda p: [int(n) for n in re.findall(r"\d+", p.name)],
        )
        if candidates:
            env["PATH"] = f"{candidates[-1] / 'bin'}{os.pathsep}{env.get('PATH', '')}"
    return env


def bin_path(root: Path, name: str) -> Path:
    return root / "node_modules" / ".bin" / name


def run(command: list[str], root: Path, timeout: int) -> subprocess.CompletedProcess:
    return subprocess.run(command, cwd=root, capture_output=True, text=True, timeout=timeout, env=node_env(root))


def failures(output: str) -> str:
    """Líneas útiles de la salida de vitest, eslint o astro check."""
    keep = re.compile(r"(×|FAIL|AssertionError|Error:|error|✗|^\s*[+-] |^\s+\d+:\d+\s)")
    lines = [line.rstrip() for line in output.splitlines() if keep.search(line)]
    return "\n".join(lines[:MAX_FEEDBACK_LINES]) or "\n".join(output.strip().splitlines()[-MAX_FEEDBACK_LINES:])


def log(mode: str, decision: str, detail: str = "") -> None:
    target = os.environ.get("CRAFT_WEB_HOOKS_LOG")
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
    root = project_root(path.parent)
    if root is None or not bin_path(root, "vitest").exists():
        return 0
    relative = path.relative_to(root) if path.is_relative_to(root) else path
    if path.suffix not in EDITABLE_SUFFIXES and path.name != "_headers":
        return 0
    if not str(relative).startswith(WATCHED):
        return 0

    prettier = bin_path(root, "prettier")
    if prettier.exists() and path.is_file() and path.name != "_headers":
        run([str(prettier), "--write", "--log-level", "silent", str(path)], root, timeout=30)

    result = run([str(bin_path(root, "vitest")), "run"], root, timeout=110)
    if result.returncode == 0:
        return 0
    return feedback(
        f"Pruebas unitarias en rojo tras editar {relative}:\n\n"
        f"{failures(result.stdout + result.stderr)}\n\n"
        "Corrige el código para cumplir la regla (WEB-*) que nombra el test; no modifiques el test."
    )


def has_changes(root: Path) -> bool:
    result = subprocess.run(
        ["git", "status", "--porcelain", "--", *WATCHED], cwd=root, capture_output=True, text=True, timeout=30
    )
    return result.returncode == 0 and bool(result.stdout.strip())


def stop(payload: dict) -> int:
    if os.environ.get("CRAFT_WEB_STOP_GATE") == "0" or payload.get("stop_hook_active"):
        return 0
    root = project_root(Path(payload.get("cwd") or os.getcwd()))
    if root is None or not bin_path(root, "vitest").exists() or not has_changes(root):
        return 0

    checks = [
        ("lint", [str(bin_path(root, "eslint")), "."]),
        ("astro check", [str(bin_path(root, "astro")), "check"]),
        ("unit", [str(bin_path(root, "vitest")), "run"]),
    ]
    for name, command in checks:
        if not Path(command[0]).exists():
            continue
        result = run(command, root, timeout=280)
        if result.returncode != 0:
            return feedback(
                f"Antes de terminar: {name} falla.\n\n{failures(result.stdout + result.stderr)}\n\n"
                "Si el fallo lo causó tu cambio, corrígelo antes de terminar (VER-1). "
                "Si el rojo es intencional (fase roja de TDD) o crees que el test está mal, "
                "no lo modifiques: explica el estado y por qué, y termina. Este control "
                "bloquea una sola vez por turno (TDD-1)."
            )
    return 0


def main() -> int:
    if os.environ.get("CRAFT_WEB_HOOKS") == "0":
        return 0
    mode = sys.argv[1] if len(sys.argv) > 1 else ""
    payload = read_input()
    handlers = {"post-edit": post_edit, "stop": stop}
    if mode not in handlers:
        return 0
    try:
        code = handlers[mode](payload)
    except subprocess.TimeoutExpired as error:
        print(f"craft-web: `{' '.join(map(str, error.cmd))}` excedió el tiempo límite", file=sys.stderr)
        code = 0
    except FileNotFoundError as error:
        print(f"craft-web: herramienta no encontrada: {error.filename}", file=sys.stderr)
        code = 0
    target = (payload.get("tool_input") or {}).get("file_path") or payload.get("cwd", "")
    log(mode, "block" if code == 2 else "pass", target)
    return code


if __name__ == "__main__":
    sys.exit(main())
