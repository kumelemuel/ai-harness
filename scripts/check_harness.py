#!/usr/bin/env python3
"""Verificador de consistencia del harness: VER-1 aplicado al propio repo.

Uso:
  python3 scripts/check_harness.py [raíz]               (por defecto, la raíz del repo)
  python3 scripts/check_harness.py [raíz] --base REF    (además, lo que entra a main)

Sale con 0 si no hay problemas; si no, con 1 y una línea por problema:
  ruta:línea: CÓDIGO: mensaje

Comprobaciones:
  ID-UNDEFINED   un ID citado (HEX-n, ERR-n, TDD-n, VER-n, WEB-n, RS-n, AS-n)
                 no está definido en su archivo fuente
  ID-DUPLICATE   un ID se define más de una vez en su archivo fuente
  SYMLINK        un symlink no resuelve
  MD-LINK        un enlace relativo de Markdown no resuelve
  FRONTMATTER    un SKILL.md o un subagente no tiene `name` y `description`, o
                 `name` no coincide con su carpeta (skill) o su archivo (subagente)
  STACK-PATH     una ruta entre comillas invertidas en la descripción de una skill
                 de stack (la que tiene `references/template`) no existe en la
                 plantilla. Cuenta como ruta todo lo que lleve `/` o `.`; admite `*`.
  STACK-STATUS   la versión de un stack en `stacks/README.md` no coincide con el
                 `**Estado:**` de su guía
  VOLATILE-COUNT un conteo de tests ("36 tests", "16 e2e") en la documentación.
                 Se escribe "todos en verde" y lo demuestra el CI; solo el
                 CHANGELOG conserva conteos, porque registra un momento.

Con --base REF (CI, cambios que entran a main), desde el merge-base de REF y HEAD:
  VERSION-NOT-BUMPED   cambió un archivo que llega a un plugin (su directorio o
                       lo que alcanza por symlinks) y su `version` no es mayor
  CHANGELOG-UNRELEASED el CHANGELOG tiene una sección "Sin publicar"

Solo biblioteca estándar (Python 3.11+), igual que los hooks.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from urllib.parse import unquote

# Prefijo -> archivo donde se define (relativo a la raíz).
ID_SOURCES: dict[str, str] = {
    "HEX": "core/principles/hexagonal.md",
    "ERR": "core/principles/errors.md",
    "TDD": "core/principles/tdd.md",
    "VER": "core/principles/verification.md",
    "WEB": "core/principles/web.md",
    "RS": "stacks/rust/README.md",
    "AS": "stacks/astro/README.md",
}

# Directorios generados o de dependencias: no son fuente del harness.
IGNORED_DIRS = {
    ".git",
    "node_modules",
    "target",
    "dist",
    ".astro",
    ".wrangler",
    "test-results",
    "playwright-report",
    ".lighthouseci",
    "__pycache__",
}

# Rutas (relativas a la raíz) que se excluyen a propósito: los tests del
# verificador y sus fixtures contienen problemas inyectados.
IGNORED_PATHS = {"scripts/tests"}

TEXT_SUFFIXES = {
    ".md", ".py", ".rs", ".toml", ".ts", ".js", ".mjs", ".cjs", ".astro",
    ".json", ".jsonc", ".yml", ".yaml", ".css", ".txt",
}

_PREFIXES = "|".join(sorted(ID_SOURCES, key=len, reverse=True))
CITATION = re.compile(rf"\b({_PREFIXES})-(\d+)\b")
# Una definición es un encabezado (`## HEX-1 — …`) o la primera celda de una
# fila de tabla (`| RS-1 | …`).
DEFINITION = re.compile(rf"^(?:#{{2,}}\s+|\|\s*)(({_PREFIXES})-\d+)\b(?:\s*\||\s)")

FENCE = re.compile(r"^\s*(```|~~~)")
INLINE_CODE = re.compile(r"`[^`]*`")
INLINE_LINK = re.compile(r"!?\[[^\]]*\]\(\s*<?([^)\s>]+)>?(?:\s+\"[^\"]*\")?\s*\)")
REFERENCE_LINK = re.compile(r"^\s{0,3}\[[^\]]+\]:\s*<?(\S+?)>?(?:\s+\"[^\"]*\")?\s*$")
SCHEME = re.compile(r"^[a-zA-Z][a-zA-Z0-9+.-]*:")

STATUS_ROW = re.compile(r"^\|\s*\[[^\]]+\]\(([^)]+)\)[^|]*\|\s*([^|]*)\|")
VERSION = re.compile(r"\bv\d+(?:\.\d+)*\b")
VOLATILE_COUNT = re.compile(r"\b\d+\s+(?:tests?|unitarios|e2e)\b", re.IGNORECASE)
GUIDE_STATUS = re.compile(r"\*\*Estado:\*\*\s*(v\d+(?:\.\d+)*)")


@dataclass(frozen=True)
class Problem:
    path: str
    line: int
    code: str
    message: str

    def render(self) -> str:
        return f"{self.path}:{self.line}: {self.code}: {self.message}"


# --------------------------------------------------------------------- recorrido


def walk(root: Path) -> tuple[list[Path], list[Path]]:
    """Devuelve (archivos reales, symlinks) sin seguir symlinks a directorios."""
    files: list[Path] = []
    links: list[Path] = []
    for current, dirnames, filenames in os.walk(root):
        here = Path(current)
        kept = []
        for name in sorted(dirnames):
            path = here / name
            if name in IGNORED_DIRS or rel(root, path) in IGNORED_PATHS:
                continue
            if path.is_symlink():
                links.append(path)
            else:
                kept.append(name)
        dirnames[:] = kept
        for name in sorted(filenames):
            path = here / name
            (links if path.is_symlink() else files).append(path)
    return files, links


def rel(root: Path, path: Path) -> str:
    return path.relative_to(root).as_posix()


def read_lines(path: Path) -> list[str]:
    try:
        return path.read_text(encoding="utf-8").splitlines()
    except (UnicodeDecodeError, OSError):
        return []


# --------------------------------------------------------------------------- IDs


def check_ids(root: Path, files: list[Path]) -> list[Problem]:
    problems: list[Problem] = []
    defined: dict[str, set[str]] = {}
    for prefix, source in ID_SOURCES.items():
        path = root / source
        if not path.is_file():
            continue
        seen: dict[str, int] = {}
        for number, line in enumerate(read_lines(path), start=1):
            match = DEFINITION.match(line)
            if not match or match.group(2) != prefix:
                continue
            identifier = match.group(1)
            if identifier in seen:
                problems.append(Problem(
                    source, number, "ID-DUPLICATE",
                    f"{identifier} ya está definido en la línea {seen[identifier]}",
                ))
            else:
                seen[identifier] = number
        defined[prefix] = set(seen)

    for path in files:
        if path.suffix not in TEXT_SUFFIXES:
            continue
        for number, line in enumerate(read_lines(path), start=1):
            for match in CITATION.finditer(line):
                prefix, identifier = match.group(1), match.group(0)
                if identifier in defined.get(prefix, set()):
                    continue
                source = ID_SOURCES[prefix]
                where = source if (root / source).is_file() else f"{source} (no existe)"
                problems.append(Problem(
                    rel(root, path), number, "ID-UNDEFINED",
                    f"{identifier} no está definido en {where}",
                ))
    return problems


# ---------------------------------------------------------------------- symlinks


def check_symlinks(root: Path, links: list[Path]) -> list[Problem]:
    return [
        Problem(rel(root, link), 1, "SYMLINK", f"no resuelve: -> {os.readlink(link)}")
        for link in links
        if not link.exists()
    ]


# ----------------------------------------------------------------------- Markdown


def markdown_targets(lines: list[str]):
    """(línea, destino) de cada enlace fuera de bloques y spans de código."""
    in_fence = False
    for number, line in enumerate(lines, start=1):
        if FENCE.match(line):
            in_fence = not in_fence
            continue
        if in_fence:
            continue
        reference = REFERENCE_LINK.match(line)
        if reference:
            yield number, reference.group(1)
            continue
        for match in INLINE_LINK.finditer(INLINE_CODE.sub("", line)):
            yield number, match.group(1)


def check_markdown_links(root: Path, files: list[Path]) -> list[Problem]:
    problems: list[Problem] = []
    for path in files:
        if path.suffix != ".md":
            continue
        for number, target in markdown_targets(read_lines(path)):
            if SCHEME.match(target) or target.startswith(("#", "/")):
                continue
            relative = unquote(target.split("#", 1)[0].split("?", 1)[0])
            if not relative:
                continue
            if not (path.parent / relative).exists():
                problems.append(Problem(
                    rel(root, path), number, "MD-LINK", f"`{relative}` no existe",
                ))
    return problems


# -------------------------------------------------------------------- frontmatter


def frontmatter(path: Path) -> tuple[dict[str, str], dict[str, int]] | None:
    """Claves de primer nivel del frontmatter YAML y su línea; None si no hay.

    Basta con lo que usan las skills: `clave: valor` en una línea o un bloque
    `>`/`|` con las líneas indentadas que siguen.
    """
    lines = read_lines(path)
    if not lines or lines[0].strip() != "---":
        return None
    values: dict[str, str] = {}
    positions: dict[str, int] = {}
    key: str | None = None
    for number, line in enumerate(lines[1:], start=2):
        if line.strip() == "---":
            return values, positions
        top = re.match(r"^([A-Za-z_][\w-]*):\s*(.*)$", line)
        if top:
            key, value = top.group(1), top.group(2).strip()
            positions[key] = number
            values[key] = "" if value in (">", "|", ">-", "|-") else value.strip("'\"")
        elif key and line.startswith((" ", "\t")):
            values[key] = f"{values[key]} {line.strip()}".strip()
    return None


def documents_with_frontmatter(root: Path, files: list[Path]):
    """(archivo, nombre esperado) de cada SKILL.md y cada subagente."""
    for path in files:
        parts = Path(rel(root, path)).parts
        if not parts or parts[0] != "plugins":
            continue
        if path.name == "SKILL.md":
            yield path, path.parent.name
        elif path.suffix == ".md" and len(parts) >= 2 and parts[-2] == "agents":
            yield path, path.stem


def check_frontmatter(root: Path, files: list[Path]) -> list[Problem]:
    problems: list[Problem] = []
    for path, expected in documents_with_frontmatter(root, files):
        where = rel(root, path)
        parsed = frontmatter(path)
        if parsed is None:
            problems.append(Problem(where, 1, "FRONTMATTER", "falta el frontmatter (--- … ---)"))
            continue
        values, positions = parsed
        for key in ("name", "description"):
            if not values.get(key):
                problems.append(Problem(where, 1, "FRONTMATTER", f"falta `{key}` en el frontmatter"))
        name = values.get("name")
        if name and name != expected:
            problems.append(Problem(
                where, positions["name"], "FRONTMATTER",
                f"`name: {name}` no coincide con `{expected}`",
            ))
    return problems


# ------------------------------------------------------------------------ stacks


def check_stack_descriptions(root: Path, files: list[Path]) -> list[Problem]:
    problems: list[Problem] = []
    for path, _ in documents_with_frontmatter(root, files):
        template = path.parent / "references" / "template"
        if path.name != "SKILL.md" or not template.is_dir():
            continue
        parsed = frontmatter(path)
        if parsed is None:
            continue
        values, positions = parsed
        for token in re.findall(r"`([^`]+)`", values.get("description", "")):
            if "/" not in token and "." not in token:
                continue
            candidate = token.rstrip("/")
            exists = (
                any(template.glob(candidate)) if "*" in candidate
                else (template / candidate).exists()
            )
            if not exists:
                problems.append(Problem(
                    rel(root, path), positions.get("description", 1), "STACK-PATH",
                    f"`{token}` no existe en la plantilla ({rel(root, template)})",
                ))
    return problems


def check_stack_status(root: Path) -> list[Problem]:
    index = root / "stacks" / "README.md"
    if not index.is_file():
        return []
    where = rel(root, index)
    problems: list[Problem] = []
    for number, line in enumerate(read_lines(index), start=1):
        row = STATUS_ROW.match(line)
        if not row:
            continue
        target, status = row.group(1), row.group(2)
        guide = index.parent / target
        listed = VERSION.search(status)
        if not guide.is_file() or not listed:
            continue  # un enlace roto ya lo informa MD-LINK
        found = GUIDE_STATUS.search(guide.read_text(encoding="utf-8"))
        if not found:
            problems.append(Problem(
                where, number, "STACK-STATUS",
                f"{rel(root, guide)} no declara `**Estado:** vX.Y` (el índice dice {listed.group(0)})",
            ))
        elif found.group(1) != listed.group(0):
            problems.append(Problem(
                where, number, "STACK-STATUS",
                f"el índice dice {listed.group(0)} y {rel(root, guide)} dice {found.group(1)}",
            ))
    return problems


# ------------------------------------------------------------- números volátiles


def check_volatile_counts(root: Path, files: list[Path]) -> list[Problem]:
    """Incluye los bloques de código: ahí también se desactualizan."""
    problems: list[Problem] = []
    for path in files:
        if path.suffix != ".md" or rel(root, path) == "CHANGELOG.md":
            continue
        for number, line in enumerate(read_lines(path), start=1):
            for match in VOLATILE_COUNT.finditer(line):
                problems.append(Problem(
                    rel(root, path), number, "VOLATILE-COUNT",
                    f"`{match.group(0)}`: usa \"todos en verde\" y deja que el CI lo demuestre",
                ))
    return problems


# ----------------------------------------------------------------- publicación


UNRELEASED = re.compile(r"^##\s+Sin publicar\b", re.IGNORECASE)


def git(root: Path, *args: str) -> str:
    result = subprocess.run(
        ["git", "-C", str(root), *args], capture_output=True, text=True, check=False,
    )
    if result.returncode != 0:
        raise ReleaseError(f"git {' '.join(args)}: {result.stderr.strip()}")
    return result.stdout


class ReleaseError(Exception):
    """La comparación con la base no se pudo hacer (commit inexistente, sin git)."""


def plugins(root: Path) -> list[tuple[str, Path]]:
    """(nombre, directorio) de cada plugin con ruta relativa del marketplace."""
    marketplace = root / ".claude-plugin" / "marketplace.json"
    if not marketplace.is_file():
        return []
    entries = json.loads(marketplace.read_text(encoding="utf-8")).get("plugins", [])
    return [
        (entry["name"], (root / entry["source"]).resolve())
        for entry in entries
        if isinstance(entry.get("source"), str)
    ]


def reach(root: Path, plugin_dir: Path) -> list[str]:
    """Rutas (relativas a la raíz, sin `/` final) cuyo contenido llega al plugin.

    El propio directorio y el destino de cada symlink que contiene, también los
    symlinks dentro de directorios enlazados. No baja a IGNORED_DIRS: lo generado
    no está versionado y no viaja en una instalación desde git.
    """
    reached: set[str] = set()
    pending = [plugin_dir]
    while pending:
        current = pending.pop()
        try:
            target = rel(root, current.resolve())
        except ValueError:
            continue  # fuera del repo: Claude Code no lo copia
        if target in reached:
            continue
        reached.add(target)
        if not current.is_dir():
            continue
        for here, dirnames, filenames in os.walk(current):
            base = Path(here)
            kept = []
            for name in dirnames:
                if name in IGNORED_DIRS:
                    continue
                (pending.append(base / name) if (base / name).is_symlink() else kept.append(name))
            dirnames[:] = kept
            pending.extend(base / name for name in filenames if (base / name).is_symlink())
    return sorted(reached)


def reaches(path: str, roots: list[str]) -> bool:
    return any(root in ("", ".") or path == root or path.startswith(root + "/") for root in roots)


def version_key(version: str) -> tuple[int, ...] | None:
    match = re.fullmatch(r"(\d+)\.(\d+)\.(\d+)", version.strip())
    return tuple(int(part) for part in match.groups()) if match else None


def version_line(manifest: Path) -> int:
    for number, line in enumerate(read_lines(manifest), start=1):
        if '"version"' in line:
            return number
    return 1


def check_version_bumps(root: Path, base: str) -> list[Problem]:
    merge_base = git(root, "merge-base", base, "HEAD").strip()
    changed = [
        line for line in git(
            root, "diff", "--name-only", "--no-renames", merge_base, "HEAD",
        ).splitlines() if line
    ]
    problems: list[Problem] = []
    for name, plugin_dir in plugins(root):
        manifest = plugin_dir / ".claude-plugin" / "plugin.json"
        manifest_rel = rel(root, manifest)
        try:
            before = json.loads(git(root, "show", f"{merge_base}:{manifest_rel}"))
        except ReleaseError:
            continue  # plugin nuevo: su primera versión no compite con nada
        roots = reach(root, plugin_dir)
        touched = [path for path in changed if reaches(path, roots) and path != manifest_rel]
        if not touched:
            continue
        old = str(before.get("version", ""))
        new = str(json.loads(manifest.read_text(encoding="utf-8")).get("version", ""))
        old_key, new_key = version_key(old), version_key(new)
        bumped = new != old and (old_key is None or new_key is None or new_key > old_key)
        if bumped:
            continue
        shown = ", ".join(touched[:3]) + (f" (y {len(touched) - 3} más)" if len(touched) > 3 else "")
        state = f"sigue en {old}" if new == old else f"pasa de {old} a {new}, que no es mayor"
        problems.append(Problem(
            manifest_rel, version_line(manifest), "VERSION-NOT-BUMPED",
            f"`{name}` {state} y cambió su contenido: {shown}",
        ))
    return problems


def check_unreleased(root: Path) -> list[Problem]:
    changelog = root / "CHANGELOG.md"
    return [
        Problem("CHANGELOG.md", number, "CHANGELOG-UNRELEASED",
                "`main` no tiene \"Sin publicar\": publica esas entradas como una versión")
        for number, line in enumerate(read_lines(changelog), start=1)
        if UNRELEASED.match(line)
    ]


def check_release(root: Path, base: str) -> list[Problem]:
    """Lo que entra a `main` desde `base`: versiones subidas y nada sin publicar."""
    root = root.resolve()
    return sorted(
        [*check_version_bumps(root, base), *check_unreleased(root)],
        key=lambda p: (p.path, p.line, p.code, p.message),
    )


# --------------------------------------------------------------------------- main


def check(root: Path) -> list[Problem]:
    root = root.resolve()
    files, links = walk(root)
    problems = [
        *check_ids(root, files),
        *check_symlinks(root, links),
        *check_markdown_links(root, files),
        *check_frontmatter(root, files),
        *check_stack_descriptions(root, files),
        *check_stack_status(root),
        *check_volatile_counts(root, files),
    ]
    return sorted(problems, key=lambda p: (p.path, p.line, p.code, p.message))


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description="Verificador de consistencia del harness.")
    parser.add_argument("root", nargs="?", type=Path, default=Path(__file__).resolve().parent.parent)
    parser.add_argument(
        "--base", metavar="REF",
        help="commit base del cambio que entra a main: exige versiones nuevas y nada sin publicar",
    )
    args = parser.parse_args(argv[1:])
    problems = check(args.root)
    if args.base:
        try:
            problems += check_release(args.root, args.base)
        except ReleaseError as error:
            print(f"No se pudo comparar con la base {args.base}: {error}", file=sys.stderr)
            return 2
    for problem in problems:
        print(problem.render())
    if problems:
        print(f"\n{len(problems)} problema(s) de consistencia.", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
