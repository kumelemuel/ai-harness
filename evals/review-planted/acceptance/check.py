"""Aceptación oculta de la kata de revisión.

`planted.diff` compila y pasa clippy y todos los tests: solo un revisor puede
ver estas violaciones. El reporte sigue el formato de `architecture-reviewer`:
una línea `ID | ruta:línea | descripción` por hallazgo, con líneas indentadas
debajo. Cada violación cuenta como detectada si un hallazgo tiene su ID, el
archivo en su ruta y una pista en su texto. Un ID mencionado de pasada ("TDD-1:
sin hallazgos") o un hallazgo sin ruta no cuenta.
"""

from __future__ import annotations

import os
import re
import sys
from pathlib import Path

# ID → (archivos posibles, pistas posibles; sin pistas basta el archivo)
PLANTED = {
    "HEX-7": (["handler.rs"], ["official", "staff", "prefi", "reserv"]),
    "ERR-2": (["error.rs", "handler.rs", "create_community.rs"], ["Reserved"]),
    "RS-2": (["handler.rs"], ["expect", "split_once", "panic"]),
    "TDD-1": (["community_slug.rs"], []),
}
# `ID | ruta:línea | descripción`, opcionalmente con viñeta o comillas invertidas.
FINDING = re.compile(r"^[-*\s]*`?([A-Z]+-\d+)`?\s*\|\s*([^|]+?)\s*\|\s*(.*)$")


def findings(report: list[str]) -> list[tuple[str, str, str]]:
    """(ID, ruta, texto) por hallazgo. El texto suma las líneas indentadas que
    lo siguen (`Por qué importa`, `Corrección`) hasta el próximo hallazgo."""
    found: list[list[str]] = []
    for line in report:
        match = FINDING.match(line)
        if match:
            found.append([match.group(1), match.group(2), match.group(3)])
        elif found and line.startswith((" ", "\t")) and line.strip():
            found[-1][2] += "\n" + line.strip()
        elif not line.strip():
            continue
        else:
            found.append(["", "", ""])  # cierra el hallazgo anterior
    return [(rule, path, text) for rule, path, text in found if rule]


def detected(report: list[str], rule: str, files: list[str], hints: list[str]) -> bool:
    for found_rule, path, text in findings(report):
        if found_rule != rule or not any(f in path for f in files):
            continue
        if not hints or any(h in text for h in hints):
            return True
    return False


def main() -> int:
    report = Path(os.environ["EVAL_RESULT_TEXT"]).read_text(encoding="utf-8").splitlines()
    failed = False
    for rule, (files, hints) in PLANTED.items():
        ok = detected(report, rule, files, hints)
        failed |= not ok
        print(f"{'PASS' if ok else 'FAIL'} {rule}")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
