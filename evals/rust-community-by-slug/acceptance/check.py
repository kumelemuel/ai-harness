"""Aceptación oculta de la kata: se copia después de que el agente termina.

Caja negra sobre el servidor compilado (no depende de los nombres internos que
eligió el agente) y los tests de arquitectura. Imprime `PASS`/`FAIL` por chequeo.
"""

from __future__ import annotations

import json
import os
import socket
import subprocess
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
FAILED = False


def report(name: str, ok: bool, detail: str = "") -> None:
    global FAILED
    FAILED |= not ok
    print(f"{'PASS' if ok else 'FAIL'} {name}{': ' + detail if detail and not ok else ''}", flush=True)


def call(base: str, method: str, path: str, account: str, body: dict | None = None):
    data = json.dumps(body).encode() if body is not None else None
    request = urllib.request.Request(base + path, data=data, method=method)
    request.add_header("x-account-id", account)
    if data is not None:
        request.add_header("content-type", "application/json")
    try:
        with urllib.request.urlopen(request, timeout=10) as response:
            return response.status, response.headers.get("content-type", ""), response.read().decode()
    except urllib.error.HTTPError as error:
        return error.code, error.headers.get("content-type", ""), error.read().decode()


def free_port() -> int:
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


def start_server() -> tuple[subprocess.Popen, str] | None:
    build = subprocess.run(["cargo", "build", "-q", "-p", "server"], cwd=ROOT, capture_output=True, text=True)
    report("server_builds", build.returncode == 0, build.stderr[-2000:])
    if build.returncode != 0:
        return None
    target = Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target"))
    address = f"127.0.0.1:{free_port()}"
    process = subprocess.Popen([str(target / "debug" / "server")], cwd=ROOT,
                               env={**os.environ, "BIND_ADDRESS": address},
                               stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    for _ in range(100):
        try:
            socket.create_connection(tuple(address.split(":")), timeout=0.2).close()
            return process, f"http://{address}"
        except OSError:
            time.sleep(0.1)
    process.kill()
    report("server_starts", False, "no escucha")
    return None


def http_checks(base: str) -> None:
    status, _, body = call(base, "POST", "/communities", "alice",
                           {"slug": "rust-chile", "name": "Rust Chile", "is_public": True})
    if status != 201:
        report("create_community", False, f"{status} {body}")
        return
    community_id = json.loads(body)["id"]

    status, _, body = call(base, "GET", "/communities/rust-chile", "bob")
    found = json.loads(body or "{}") if status == 200 else {}
    ok = found.get("id") == community_id and found.get("name") == "Rust Chile"
    report("finds_by_slug", ok, f"{status} {body}")

    status, content_type, body = call(base, "GET", "/communities/no-existe", "bob")
    ok = status == 404 and content_type.startswith("application/problem+json")
    report("unknown_slug_is_not_found", ok, f"{status} {content_type} {body}")


def main() -> int:
    started = start_server()
    if started:
        process, base = started
        try:
            http_checks(base)
        finally:
            process.kill()
    tests = subprocess.run(["cargo", "test", "-q", "-p", "architecture"], cwd=ROOT, capture_output=True, text=True)
    report("architecture_tests", tests.returncode == 0, (tests.stdout + tests.stderr)[-2000:])
    return 1 if FAILED else 0


if __name__ == "__main__":
    sys.exit(main())
