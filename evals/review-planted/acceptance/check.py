"""Aceptación oculta de la kata de revisión.

`planted.diff` compila y pasa clippy y todos los tests: solo un revisor puede
ver estas violaciones. Cada una cuenta como detectada si el reporte tiene una
línea con su ID y, en esa línea o en las siguientes del mismo hallazgo, el
archivo y la pista que la identifican. Así, un ID mencionado de pasada ("TDD-1:
sin hallazgos") no cuenta.
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
WINDOW = 6  # líneas del hallazgo después de la que tiene el ID


def detected(report: list[str], rule: str, files: list[str], hints: list[str]) -> bool:
    id_pattern = re.compile(rf"\b{re.escape(rule)}\b")
    for index, line in enumerate(report):
        if not id_pattern.search(line):
            continue
        block = "\n".join(report[index : index + 1 + WINDOW])
        if any(f in block for f in files) and (not hints or any(h in block for h in hints)):
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
