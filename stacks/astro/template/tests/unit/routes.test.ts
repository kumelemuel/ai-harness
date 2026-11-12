import { existsSync, readFileSync, readdirSync } from "node:fs";
import { join, relative } from "node:path";
import { describe, expect, it } from "vitest";
import { NOINDEX, ON_DEMAND, REDIRECTS, ROUTES, isIndexable, type RouteKey } from "../../src/routes";

const SRC = join(import.meta.dirname, "../../src");
const PAGES = join(SRC, "pages");

/** Archivo de página que corresponde a una ruta: /a/b/ → a/b.astro o a/b/index.astro. */
const pageFileFor = (path: string) => {
  const base = path.replace(/^\/|\/$/g, "");
  return [`${base || "index"}.astro`, join(base, "index.astro")].find((file) =>
    existsSync(join(PAGES, file)),
  );
};

const allPages = (dir: string): string[] =>
  readdirSync(dir, { withFileTypes: true }).flatMap((entry) =>
    entry.isDirectory() ? allPages(join(dir, entry.name)) : [join(dir, entry.name)],
  );

describe("WEB-2: una sola fuente de rutas", () => {
  it.each(Object.entries(ROUTES))("%s (%s) tiene su archivo de página", (_key, path) => {
    expect(pageFileFor(path), `falta la página de ${path}`).toBeDefined();
  });

  it("toda ruta termina en /", () => {
    for (const path of Object.values(ROUTES)) expect(path).toMatch(/\/$/);
  });

  it("ningún componente escribe una ruta interna a mano (usa route() o articlePath())", () => {
    // Literales href="/…" sin extensión de archivo: los assets (/favicon.svg) sí se permiten.
    const hardcoded = allPages(SRC)
      .filter((file) => file.endsWith(".astro"))
      .flatMap((file) =>
        [...readFileSync(file, "utf8").matchAll(/href="(\/[^"]*)"/g)]
          .map((match) => match[1])
          .filter((href) => !/\.[a-z0-9]+$/i.test(href))
          .map((href) => `${relative(SRC, file)}: href="${href}"`),
      );
    expect(hardcoded).toEqual([]);
  });

  it("cada redirección apunta a una ruta existente", () => {
    const targets = new Set<string>(Object.values(ROUTES));
    for (const [from, to] of Object.entries(REDIRECTS)) {
      expect(targets.has(to), `${from} → ${to}`).toBe(true);
    }
  });
});

describe("WEB-1: estático por defecto", () => {
  it("las páginas on-demand son exactamente las declaradas en ON_DEMAND", () => {
    const onDemand = allPages(PAGES)
      .filter((file) => /export const prerender = false/.test(readFileSync(file, "utf8")))
      .map((file) => relative(PAGES, file))
      .sort();
    const declared = ON_DEMAND.map((key: RouteKey) => pageFileFor(ROUTES[key])).sort();
    expect(onDemand).toEqual(declared);
  });
});

describe("WEB-9: indexación", () => {
  it("las rutas noindex quedan fuera del sitemap", () => {
    for (const key of NOINDEX) expect(isIndexable(ROUTES[key])).toBe(false);
    expect(isIndexable(ROUTES.home)).toBe(true);
  });
});
