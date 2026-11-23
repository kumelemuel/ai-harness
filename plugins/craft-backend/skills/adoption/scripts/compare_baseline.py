#!/usr/bin/env python3
"""Compara la línea base de antes y de después de sincronizar `tools/architecture`.

Uso: compare_baseline.py VIEJA NUEVA

Una sincronización con el harness no cambia código de producción, así que nada
debería cerrarse. Cada entrada vieja tiene que seguir en la nueva, con la misma
clave o con otra que la normaliza igual (sin el `#sección.paquete` que los
tests de dependencias agregaron en 0.17.0). Grupos del informe (Markdown, para
el PR):

  Se mantienen       misma clave y mismo conteo.
  Cambian de clave   la clave vieja se reparte en una o más claves nuevas del
                     mismo test y el mismo archivo. Los conteos no se comparan:
                     desde 0.18.0, `n` cuenta las capas virtuales legacy.
  Desaparecen        sin clave nueva que la normalice igual, en el mismo test.
  Bajan de conteo    misma clave, `n` menor.
  Suben de conteo    misma clave, `n` mayor.
  Nuevas             claves sin equivalente en la vieja.

Desaparecen y Bajan detienen el PR (salida 1): pueden ser un cierre en falso,
una violación que la versión nueva dejó de detectar. Las nuevas y las que suben
necesitan un ID del inventario y una rebanada, pero no detienen. Un test
renombrado no se adivina: aparece como una que desaparece y otra nueva.
"""

from __future__ import annotations

import sys
from dataclasses import dataclass, field
from pathlib import Path

# Mismo parser que el hook `pre-edit`, que lee igual que el trinquete en Rust.
sys.path.insert(0, str(Path(__file__).resolve().parents[3] / "hooks"))

from craft_hooks import baseline_entries  # noqa: E402

Entry = tuple[str, str, int]
Change = tuple[str, str, int, int]


def normalized(key: str) -> str:
    """`ruta/Cargo.toml#sección.paquete` → `ruta/Cargo.toml`; el resto, igual."""
    return key.split("#", 1)[0]


@dataclass
class Comparison:
    kept: list[Change] = field(default_factory=list)
    rekeyed: dict[tuple[str, str], list[tuple[str, int]]] = field(default_factory=dict)
    gone: list[Entry] = field(default_factory=list)
    lowered: list[Change] = field(default_factory=list)
    raised: list[Change] = field(default_factory=list)
    added: list[Entry] = field(default_factory=list)
    old_counts: dict[tuple[str, str], int] = field(default_factory=dict)

    @property
    def ok(self) -> bool:
        return not self.gone and not self.lowered


def compare(old_text: str, new_text: str) -> Comparison:
    old, new = baseline_entries(old_text), baseline_entries(new_text)
    result = Comparison(old_counts=old)
    claimed: set[tuple[str, str]] = set()
    for (test, key), before in sorted(old.items()):
        if (test, key) in new:
            after = new[(test, key)]
            claimed.add((test, key))
            group = result.kept if after == before else (
                result.lowered if after < before else result.raised
            )
            group.append((test, key, before, after))
            continue
        targets = sorted(
            (k, n) for (t, k), n in new.items()
            if t == test and (t, k) not in old and normalized(k) == normalized(key)
        )
        if targets:
            result.rekeyed[(test, key)] = targets
            claimed.update((test, k) for k, _ in targets)
        else:
            result.gone.append((test, key, before))
    result.added = sorted((t, k, n) for (t, k), n in new.items() if (t, k) not in claimed)
    return result


def entry(test: str, key: str, n: int) -> str:
    return f"`{test} {key}" + (f" {n}" if n != 1 else "") + "`"


def report(result: Comparison) -> str:
    new_keys = sum(len(v) for v in result.rekeyed.values())
    lines = [f"### Se mantienen ({len(result.kept)})", ""]
    lines += [f"- {entry(t, k, n)}" for t, k, n, _ in result.kept]
    lines += ["", f"### Cambian de clave ({len(result.rekeyed)} → {new_keys})", ""]
    for (test, key), targets in result.rekeyed.items():
        lines.append(
            f"- {entry(test, key, result.old_counts[(test, key)])} → "
            + ", ".join(f"`{k}" + (f" {n}" if n != 1 else "") + "`" for k, n in targets)
        )
    lines += ["", f"### Desaparecen ({len(result.gone)})", ""]
    lines += [f"- {entry(t, k, n)}" for t, k, n in result.gone]
    lines += ["", f"### Bajan de conteo ({len(result.lowered)})", ""]
    lines += [f"- {entry(t, k, a)} → {b}" for t, k, a, b in result.lowered]
    lines += ["", f"### Suben de conteo ({len(result.raised)})", ""]
    lines += [f"- {entry(t, k, a)} → {b}" for t, k, a, b in result.raised]
    lines += ["", f"### Nuevas ({len(result.added)})", ""]
    lines += [f"- {entry(t, k, n)}" for t, k, n in result.added]
    lines.append("")
    if result.ok:
        lines.append(
            "Ninguna entrada se cierra. Las nuevas y las que suben necesitan un ID del "
            "inventario y una rebanada antes del merge."
        )
    else:
        lines.append(
            "DETENER: una sincronización no cambia código, así que una entrada que "
            "desaparece o baja puede ser un cierre en falso (la versión nueva dejó de "
            "detectar la violación). Explícala con evidencia antes de seguir."
        )
    return "\n".join(lines)


def main(argv: list[str]) -> int:
    if len(argv) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    old_text, new_text = (Path(p).read_text(encoding="utf-8") for p in argv)
    result = compare(old_text, new_text)
    print(report(result))
    return 0 if result.ok else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
