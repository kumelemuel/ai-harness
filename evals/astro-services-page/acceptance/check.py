"""Aceptación oculta de la kata: se copia después de que el agente termina.

La página existe y está registrada en `src/routes.ts` (WEB-2), y el sitio pasa
lint, `astro check` y los tests unitarios. Imprime `PASS`/`FAIL` por chequeo.
"""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
FAILED = False


def report(name: str, ok: bool, detail: str = "") -> None:
    global FAILED
    FAILED |= not ok
    print(f"{'PASS' if ok else 'FAIL'} {name}{': ' + detail if detail and not ok else ''}", flush=True)


def main() -> int:
    pages = [ROOT / "src/pages/servicios.astro", ROOT / "src/pages/servicios/index.astro"]
    report("page_exists", any(p.is_file() for p in pages), "ni servicios.astro ni servicios/index.astro")
    routes = (ROOT / "src/routes.ts").read_text(encoding="utf-8")
    report("route_registered", re.search(r"[\"']/servicios/[\"']", routes) is not None, "falta /servicios/ en src/routes.ts")
    for name, script in (("lint", "lint"), ("astro_check", "check"), ("unit_tests", "test")):
        result = subprocess.run(["npm", "run", "-s", script], cwd=ROOT, capture_output=True, text=True)
        report(name, result.returncode == 0, (result.stdout + result.stderr)[-2000:])
    return 1 if FAILED else 0


if __name__ == "__main__":
    sys.exit(main())
