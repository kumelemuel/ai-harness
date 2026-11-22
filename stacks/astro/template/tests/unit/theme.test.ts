import { readFileSync, readdirSync } from "node:fs";
import { join, relative } from "node:path";
import { describe, expect, it } from "vitest";
import { arbitraryOrPaletteColors, looseCssColors, themeTokens } from "./theme";

const SRC = join(import.meta.dirname, "../../src");
const CSS = join(SRC, "styles/global.css");

const files = (dir: string, extensions: RegExp): string[] =>
  readdirSync(dir, { withFileTypes: true }).flatMap((entry) =>
    entry.isDirectory()
      ? files(join(dir, entry.name), extensions)
      : extensions.test(entry.name)
        ? [join(dir, entry.name)]
        : [],
  );

describe("AS-2: los tokens de marca viven en @theme", () => {
  const tokens = themeTokens(readFileSync(CSS, "utf8"));

  it("@theme declara colores y tipografías", () => {
    expect(tokens.colors.size).toBeGreaterThan(0);
    expect(tokens.fonts.size).toBeGreaterThan(0);
  });

  it("las clases usan colores y tipografías de @theme, no valores sueltos ni la paleta de Tailwind", () => {
    const found = files(SRC, /\.(astro|ts|tsx|mdx)$/).flatMap((file) =>
      arbitraryOrPaletteColors(readFileSync(file, "utf8"), tokens).map(
        (cls) => `${relative(SRC, file)}: ${cls}`,
      ),
    );
    expect(found).toEqual([]);
  });

  it("las hojas de estilo no escriben colores fuera de @theme", () => {
    const found = files(SRC, /\.css$/).flatMap((file) =>
      looseCssColors(readFileSync(file, "utf8")).map((decl) => `${relative(SRC, file)}: ${decl}`),
    );
    expect(found).toEqual([]);
  });
});

describe("AS-2: el detector", () => {
  const tokens = themeTokens("@theme {\n  --color-ink: #111;\n  --font-display: serif;\n}");

  it("acepta los tokens, con variantes y opacidad", () => {
    const code = 'class="text-ink hover:bg-ink/50 aria-[current=page]:border-ink font-display"';
    expect(arbitraryOrPaletteColors(code, tokens)).toEqual([]);
  });

  it("ignora las utilidades que no son de color", () => {
    const code = 'class="text-xl text-center border-2 border-b font-semibold -left-[9999px] bg-transparent"';
    expect(arbitraryOrPaletteColors(code, tokens)).toEqual([]);
  });

  it("rechaza los valores arbitrarios de color y tipografía", () => {
    const code = 'class="text-[#ff0000] bg-[rgb(0,0,0)] md:border-[oklch(0.5_0.1_20)] font-[Comic_Sans]"';
    expect(arbitraryOrPaletteColors(code, tokens)).toEqual([
      "text-[#ff0000]",
      "bg-[rgb(0,0,0)]",
      "border-[oklch(0.5_0.1_20)]",
      "font-[Comic_Sans]",
    ]);
  });

  it("rechaza la paleta por defecto de Tailwind y las familias que @theme no declara", () => {
    const code = 'class="bg-white text-red-500 hover:ring-slate-200 font-mono"';
    expect(arbitraryOrPaletteColors(code, tokens)).toEqual([
      "bg-white",
      "text-red-500",
      "ring-slate-200",
      "font-mono",
    ]);
  });

  it("encuentra colores sueltos en CSS fuera de @theme", () => {
    const css =
      "@theme {\n  --color-a: #fff;\n}\n.a {\n  color: white;\n  background-color: #123456;\n  border-color: var(--color-a);\n}";
    expect(looseCssColors(css)).toEqual(["color: white", "background-color: #123456"]);
  });
});
