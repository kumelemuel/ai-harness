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
  CLAUDE-MD-SIZE el `CLAUDE.md` de la raíz supera CLAUDE_MD_MAX_WORDS palabras
  TEMPLATE-MAP-PATH una ruta entre comillas invertidas del mapa de una plantilla
                 (`stacks/*/template/CLAUDE.md`) no existe en ella
  TEMPLATE-MAP-SIZE el mapa de una plantilla supera TEMPLATE_MAP_MAX_WORDS palabras
  STACK-STATUS   la versión de un stack en `stacks/README.md` no coincide con el
                 `**Estado:**` de su guía
  VOLATILE-COUNT un conteo de tests ("36 tests", "16 e2e") en la documentación.
                 Se escribe "todos en verde" y lo demuestra el CI; solo el
                 CHANGELOG conserva conteos, porque registra un momento.
  MISSING-TEST   un test citado entre comillas invertidas en una sección
                 "Verificación" o en una tabla de Enforcement no existe en la
                 plantilla: una función con prefijo de principio (`hex_4_…`,
                 admite `*`), opcionalmente con su archivo
                 (`tests/naming.rs::hex_4_*`), o un archivo de test
                 (`routes.test.ts`, `tests/naming.rs`)

  Inventario `core/verification.md` (si existe; VER-1 lo enlaza, así que
  borrarlo es un MD-LINK). Una tabla `ID | Aspecto | Mecanismo | Detalle`:
  VERIF-MISSING-ID    un ID definido en `core/principles/` o en una guía de
                      stack no tiene ninguna fila
  VERIF-UNKNOWN-ID    una fila con un ID que no está definido
  VERIF-BAD-MECHANISM un mecanismo fuera de la lista cerrada (`test`,
                      `herramienta`, `revisor:<agente>`, `pendiente`), o una
                      `herramienta` o un `pendiente` sin detalle
  VERIF-REVIEWER      `revisor:<agente>`: el agente no existe en
                      `plugins/*/agents/` o su archivo no menciona el ID
  MISSING-TEST        una fila `test` no cita ningún test, o cita uno que no
                      existe (misma búsqueda que en "Verificación")

Con --base REF (CI, cambios que entran a main), desde el merge-base de REF y HEAD:
  VERSION-NOT-BUMPED   cambió un archivo que llega a un plugin (su directorio o
                       lo que alcanza por symlinks) y su `version` no es mayor
  CHANGELOG-UNRELEASED el CHANGELOG tiene una sección "Sin publicar"

Solo biblioteca estándar (Python 3.11+), igual que los hooks.
"""

from __future__ import annotations

import argparse
import fnmatch
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


def definitions(root: Path) -> list[tuple[str, str, int]]:
    """(ID, archivo fuente, línea) de cada definición, en orden y con repeticiones."""
    found: list[tuple[str, str, int]] = []
    for prefix, source in ID_SOURCES.items():
        path = root / source
        if not path.is_file():
            continue
        for number, line in enumerate(read_lines(path), start=1):
            match = DEFINITION.match(line)
            if match and match.group(2) == prefix:
                found.append((match.group(1), source, number))
    return found


def check_ids(root: Path, files: list[Path]) -> list[Problem]:
    problems: list[Problem] = []
    defined: dict[str, set[str]] = {}
    seen: dict[str, int] = {}
    for identifier, source, number in definitions(root):
        if identifier in seen:
            problems.append(Problem(
                source, number, "ID-DUPLICATE",
                f"{identifier} ya está definido en la línea {seen[identifier]}",
            ))
        else:
            seen[identifier] = number
            defined.setdefault(identifier.split("-")[0], set()).add(identifier)

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


# Se lee entero en cada sesión que trabaja en el harness: el procedimiento
# cabe en ~600 palabras; el margen hasta 800 absorbe ajustes, no un manual.
CLAUDE_MD_MAX_WORDS = 800


def check_claude_md(root: Path) -> list[Problem]:
    path = root / "CLAUDE.md"
    if not path.is_file():
        return []
    words = len(re.findall(r"\w+", path.read_text(encoding="utf-8")))
    if words <= CLAUDE_MD_MAX_WORDS:
        return []
    return [Problem(
        "CLAUDE.md", 1, "CLAUDE-MD-SIZE",
        f"{words} palabras, el límite es {CLAUDE_MD_MAX_WORDS}: enlaza en vez de copiar",
    )]


# El mapa de una plantilla (`stacks/*/template/CLAUDE.md`) se carga en cada
# sesión del proyecto: orienta, no enseña. Los principios viven en las skills.
TEMPLATE_MAP_MAX_WORDS = 600


def check_template_maps(root: Path) -> list[Problem]:
    """Cada ruta del mapa existe en su plantilla y el mapa no crece sin control.

    Cuenta como ruta lo que va entre comillas invertidas fuera de un bloque de
    código, con `/` y sin espacios; admite `*`. Se saltan las que llevan un
    marcador (`{ctx}`, `<caso>`), que describen una forma y no un archivo.
    """
    problems: list[Problem] = []
    for path in sorted(root.glob("stacks/*/template/CLAUDE.md")):
        template = path.parent
        where = rel(root, path)
        in_code = False
        for number, line in enumerate(read_lines(path), start=1):
            if line.lstrip().startswith("```"):
                in_code = not in_code
                continue
            if in_code:
                continue
            for token in re.findall(r"`([^`]+)`", line):
                if "/" not in token or " " in token or any(c in token for c in "{}<>"):
                    continue
                candidate = token.split("::")[0].rstrip("/")
                exists = (
                    any(template.glob(candidate)) if "*" in candidate
                    else (template / candidate).exists()
                )
                if not exists:
                    problems.append(Problem(
                        where, number, "TEMPLATE-MAP-PATH",
                        f"`{token}` no existe en la plantilla ({rel(root, template)})",
                    ))
        words = len(re.findall(r"\w+", path.read_text(encoding="utf-8")))
        if words > TEMPLATE_MAP_MAX_WORDS:
            problems.append(Problem(
                where, 1, "TEMPLATE-MAP-SIZE",
                f"{words} palabras, el límite es {TEMPLATE_MAP_MAX_WORDS}: es un mapa, no una guía",
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


# ------------------------------------------------------------- tests citados

_TEST_PREFIXES = "|".join(sorted((p.lower() for p in ID_SOURCES), key=len, reverse=True))
# `hex_4_use_case_…`, `hex_2_core_*`: prefijo de principio, número y nombre.
TEST_FUNCTION = re.compile(rf"^(?:{_TEST_PREFIXES})_\d+(?:_[a-z0-9_*]*)?$")
# `routes.test.ts`, `tests/unit/routes.test.ts`, `tests/naming.rs`.
TEST_FILE = re.compile(r"^(?:[\w.-]+/)*(?:[\w-]+\.(?:test|spec)\.[cm]?[jt]sx?|tests/(?:[\w-]+/)*[\w-]+\.rs)$")
RUST_FN = re.compile(r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+(\w+)")
RUST_TEST_ATTR = re.compile(r"^\s*#\[(?:\w+::)*test\b")
HEADING = re.compile(r"^(#{1,6})\s+(.*)$")
TABLE_ROW = re.compile(r"^\s*\|")
VERIFICATION_WORDS = re.compile(r"\b(?:Verificación|Enforcement)\b")


def verification_lines(lines: list[str]):
    """(línea, texto) de lo que promete un test, fuera de bloques de código.

    - Un párrafo que empieza con `**Verificación…**`, hasta el siguiente
      encabezado, otra etiqueta en negrita o `---`.
    - Una sección cuyo encabezado dice "Verificación" o "Enforcement", hasta el
      siguiente encabezado del mismo nivel o superior.
    - Una tabla con una columna "Verificación" o "Enforcement".
    """
    in_fence = False
    section_level: int | None = None
    in_paragraph = False
    in_table = table_promises = False
    for number, line in enumerate(lines, start=1):
        if FENCE.match(line):
            in_fence = not in_fence
            continue
        if in_fence:
            continue
        heading = HEADING.match(line)
        if heading:
            level = len(heading.group(1))
            in_paragraph = False
            if section_level is not None and level <= section_level:
                section_level = None
            if section_level is None and VERIFICATION_WORDS.search(heading.group(2)):
                section_level = level
            continue
        stripped = line.strip()
        if stripped.startswith("**Verificación"):
            in_paragraph = True
        elif stripped.startswith("**") or stripped == "---":
            in_paragraph = False
        if TABLE_ROW.match(line):
            if not in_table:
                in_table = True
                table_promises = bool(VERIFICATION_WORDS.search(line))
        else:
            in_table = table_promises = False
        if section_level is not None or in_paragraph or (in_table and table_promises):
            yield number, line


def templates(root: Path) -> list[Path]:
    stacks = root / "stacks"
    return sorted(p for p in stacks.glob("*/template") if p.is_dir()) if stacks.is_dir() else []


def templates_for(root: Path, path: Path, every: list[Path]) -> list[Path]:
    """La plantilla del stack del documento; fuera de `stacks/<x>/`, todas."""
    parts = Path(rel(root, path)).parts
    if len(parts) >= 2 and parts[0] == "stacks":
        own = root / "stacks" / parts[1] / "template"
        if own in every:
            return [own]
    return every


def index_tests(template: Path) -> dict[str, set[str]]:
    """Archivo (relativo a la plantilla) -> funciones de test que declara.

    Cuenta como test todo archivo bajo un directorio `tests/` o con nombre
    `*.test.*`/`*.spec.*`. En Rust, todas las `fn` de un archivo de `tests/`, y
    en el resto solo las que llevan `#[test]` (o `#[tokio::test]`, etc.).
    """
    index: dict[str, set[str]] = {}
    for current, dirnames, filenames in os.walk(template):
        dirnames[:] = sorted(d for d in dirnames if d not in IGNORED_DIRS)
        for name in filenames:
            path = Path(current) / name
            relative = rel(template, path)
            in_tests = "tests" in Path(relative).parts[:-1]
            is_test_name = re.search(r"\.(?:test|spec)\.", name) is not None
            if path.suffix != ".rs":
                if in_tests or is_test_name:
                    index[relative] = set()
                continue
            functions: set[str] = set()
            pending_test = False
            for line in read_lines(path):
                if RUST_TEST_ATTR.match(line):
                    pending_test = True
                    continue
                fn = RUST_FN.match(line)
                if fn:
                    if in_tests or pending_test:
                        functions.add(fn.group(1))
                    pending_test = False
            if in_tests or functions:
                index[relative] = functions
    return index


def find_test(indexes: list[dict[str, set[str]]], file: str | None, function: str | None) -> bool:
    for index in indexes:
        files = (
            [f for f in index if f == file or f.endswith("/" + file)] if file
            else list(index)
        )
        if not files:
            continue
        if function is None:
            return True
        if any(fnmatch.fnmatchcase(name, function) for f in files for name in index[f]):
            return True
    return False


def cited_test(token: str) -> tuple[str | None, str | None] | None:
    """(archivo, función) que cita un token, o None si no es un test."""
    if "::" in token:
        file, _, function = token.rpartition("::")
        if TEST_FILE.match(file) and TEST_FUNCTION.match(function):
            return file, function
        return None
    if TEST_FUNCTION.match(token):
        return None, token
    if TEST_FILE.match(token):
        return token, None
    return None


def check_cited_tests(root: Path, files: list[Path]) -> list[Problem]:
    every = templates(root)
    indexes = {template: index_tests(template) for template in every}
    problems: list[Problem] = []
    for path in files:
        if path.suffix != ".md" or rel(root, path) in ("CHANGELOG.md", INVENTORY):
            continue  # el inventario lo revisa check_inventory, fila por fila
        searched = templates_for(root, path, every)
        for number, line in verification_lines(read_lines(path)):
            for token in re.findall(r"`([^`]+)`", line):
                cited = cited_test(token.strip())
                if cited is None:
                    continue
                if find_test([indexes[t] for t in searched], *cited):
                    continue
                where = ", ".join(rel(root, t) for t in searched) or "ninguna plantilla"
                problems.append(Problem(
                    rel(root, path), number, "MISSING-TEST",
                    f"`{token.strip()}` no existe en los tests de {where}",
                ))
    return problems


# ------------------------------------------------------------------ inventario

INVENTORY = "core/verification.md"
INVENTORY_HEADER = ["ID", "Aspecto", "Mecanismo", "Detalle"]
MECHANISMS = ("test", "herramienta", "pendiente")
REVIEWER_PREFIX = "revisor:"
ID_CELL = re.compile(rf"^(?:{_PREFIXES})-\d+$")


def table_cells(line: str) -> list[str]:
    """Celdas de una fila de tabla Markdown; `\\|` no separa."""
    body = line.strip()
    body = body[1:] if body.startswith("|") else body
    body = body[:-1] if body.endswith("|") and not body.endswith("\\|") else body
    return [cell.strip().replace("\\|", "|") for cell in re.split(r"(?<!\\)\|", body)]


def inventory_rows(lines: list[str]):
    """(línea, ID, aspecto, mecanismo, detalle) de las tablas del inventario."""
    in_fence = in_inventory = False
    for number, line in enumerate(lines, start=1):
        if FENCE.match(line):
            in_fence = not in_fence
            continue
        if in_fence or not TABLE_ROW.match(line):
            in_inventory = False
            continue
        cells = table_cells(line)
        if cells == INVENTORY_HEADER:
            in_inventory = True
            continue
        if not in_inventory or all(re.fullmatch(r":?-+:?", c) for c in cells):
            continue
        cells = (cells + ["", "", "", ""])[:4]
        identifier, aspect, mechanism, detail = cells
        yield number, identifier, aspect, mechanism.strip("`").strip(), detail


def agent_files(root: Path) -> dict[str, Path]:
    plugins_dir = root / "plugins"
    if not plugins_dir.is_dir():
        return {}
    return {path.stem: path for path in sorted(plugins_dir.glob("*/agents/*.md"))}


def check_inventory(root: Path) -> list[Problem]:
    path = root / INVENTORY
    if not path.is_file():
        return []
    defined = {identifier: (source, line) for identifier, source, line in reversed(definitions(root))}
    agents = agent_files(root)
    every = templates(root)
    indexes = [index_tests(template) for template in every]
    covered: set[str] = set()
    problems: list[Problem] = []

    def problem(number: int, code: str, message: str) -> None:
        problems.append(Problem(INVENTORY, number, code, message))

    for number, identifier, _aspect, mechanism, detail in inventory_rows(read_lines(path)):
        if not ID_CELL.match(identifier) or identifier not in defined:
            problem(number, "VERIF-UNKNOWN-ID",
                    f"`{identifier}` no es un ID definido en core/principles/ ni en una guía de stack")
            continue
        covered.add(identifier)
        if mechanism.startswith(REVIEWER_PREFIX):
            agent = mechanism[len(REVIEWER_PREFIX):].strip()
            if not agent:
                problem(number, "VERIF-BAD-MECHANISM", "`revisor:` sin agente: `revisor:<agente>`")
            elif agent not in agents:
                problem(number, "VERIF-REVIEWER",
                        f"el agente `{agent}` no existe en plugins/*/agents/")
            elif identifier not in {m.group(0) for m in CITATION.finditer(
                    agents[agent].read_text(encoding="utf-8"))}:
                problem(number, "VERIF-REVIEWER",
                        f"`{rel(root, agents[agent])}` no menciona {identifier}: el revisor no sabe que le toca")
        elif mechanism not in MECHANISMS:
            problem(number, "VERIF-BAD-MECHANISM",
                    f"`{mechanism}` no es un mecanismo: usa test, herramienta, revisor:<agente> o pendiente")
        elif mechanism == "test":
            cited = [c for c in (cited_test(t.strip()) for t in re.findall(r"`([^`]+)`", detail)) if c]
            if not cited:
                problem(number, "MISSING-TEST", f"la fila `test` de {identifier} no cita ningún test")
            for file, function in cited:
                if not find_test(indexes, file, function):
                    shown = f"{file}::{function}" if file and function else file or function
                    where = ", ".join(rel(root, t) for t in every) or "ninguna plantilla"
                    problem(number, "MISSING-TEST", f"`{shown}` no existe en los tests de {where}")
        elif not detail:
            problem(number, "VERIF-BAD-MECHANISM",
                    f"`{mechanism}` sin detalle: nombra la herramienta o explica por qué está pendiente")

    for identifier, (source, line) in sorted(defined.items(), key=lambda item: item[1]):
        if identifier not in covered:
            problems.append(Problem(source, line, "VERIF-MISSING-ID",
                                    f"{identifier} no tiene ninguna fila en {INVENTORY}"))
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
        *check_claude_md(root),
        *check_template_maps(root),
        *check_stack_status(root),
        *check_volatile_counts(root, files),
        *check_cited_tests(root, files),
        *check_inventory(root),
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
