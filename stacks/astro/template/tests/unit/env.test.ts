import { execFileSync } from "node:child_process";
import { copyFileSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterAll, beforeAll, describe, expect, it } from "vitest";

const root = join(import.meta.dirname, "../..");

/**
 * WEB-10: el `.gitignore` se evalúa con git, no con una imitación de sus reglas. Se copia
 * solo a un repo temporal: dentro de otro repo (el del harness) un `.gitignore` de un
 * directorio superior o la configuración global podrían ignorar `.env` por él.
 */
describe("WEB-10: los secretos no llegan al repositorio", () => {
  let repo: string;

  beforeAll(() => {
    repo = mkdtempSync(join(tmpdir(), "web-10-"));
    git("init", "-q");
    copyFileSync(join(root, ".gitignore"), join(repo, ".gitignore"));
  });

  afterAll(() => rmSync(repo, { recursive: true, force: true }));

  function git(...args: string[]): string {
    return execFileSync("git", args, {
      cwd: repo,
      encoding: "utf8",
      env: { ...process.env, GIT_CONFIG_GLOBAL: "/dev/null", GIT_CONFIG_NOSYSTEM: "1" },
    });
  }

  function ignored(path: string): boolean {
    try {
      git("check-ignore", "-q", "--no-index", path);
      return true;
    } catch (error) {
      // Sale con 1 si no se ignora; cualquier otro código es un fallo real.
      if ((error as { status?: number }).status === 1) return false;
      throw error;
    }
  }

  it.each([".env", ".env.local", ".env.production", ".dev.vars", "src/.env"])(
    ".gitignore ignora %s",
    (path) => {
      expect(ignored(path)).toBe(true);
    },
  );

  it(".gitignore no ignora .env.example", () => {
    expect(ignored(".env.example")).toBe(false);
  });

  it(".env.example declara claves sin valores", () => {
    const lines = readFileSync(join(root, ".env.example"), "utf8")
      .split("\n")
      .map((line) => line.trim())
      .filter((line) => line !== "" && !line.startsWith("#"));

    expect(lines.length).toBeGreaterThan(0);
    for (const line of lines) {
      expect(line, "una clave en MAYÚSCULAS seguida de `=` y nada más").toMatch(/^[A-Z][A-Z0-9_]*=$/);
    }
  });
});
