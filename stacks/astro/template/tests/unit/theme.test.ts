import { readFileSync, readdirSync } from "node:fs";
import { join, relative } from "node:path";
import { describe, expect, it } from "vitest";
import { arbitraryOrPaletteColors, astroStyles, looseCssColors, themeTokens } from "./theme";

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

/** Clases de las líneas `@apply` de una hoja de estilo. */
const applied = (css: string): string => [...css.matchAll(/@apply\s+([^;]+);/g)].map((m) => m[1]).join(" ");

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

  it("las hojas de estilo y los <style> no escriben colores fuera de @theme", () => {
    const sheets = [
      ...files(SRC, /\.css$/).map((file) => [file, readFileSync(file, "utf8")] as const),
      ...files(SRC, /\.astro$/).map((file) => [file, astroStyles(readFileSync(file, "utf8"))] as const),
    ];
    const found = sheets.flatMap(([file, css]) => [
      ...looseCssColors(css).map((decl) => `${relative(SRC, file)}: ${decl}`),
      ...arbitraryOrPaletteColors(applied(css), tokens).map((cls) => `${relative(SRC, file)}: @apply ${cls}`),
    ]);
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

  it("conoce toda la paleta instalada de Tailwind", () => {
    expect(arbitraryOrPaletteColors('class="bg-mauve-500 text-olive-200"', tokens)).toEqual([
      "bg-mauve-500",
      "text-olive-200",
    ]);
  });

  it("ve las formas de Tailwind 4: `!` al final, opacidad arbitraria, grupos con nombre y `*`", () => {
    const code =
      'class="bg-white! bg-red-500/[0.3] text-red-500/(--x) group-hover/item:bg-red-500 peer-checked/x:text-white *:text-white"';
    expect(arbitraryOrPaletteColors(code, tokens)).toEqual([
      "bg-white",
      "bg-red-500",
      "text-red-500",
      "bg-red-500",
      "text-white",
      "text-white",
    ]);
  });

  it("ve las utilidades de color de sombras y anillos de Tailwind 4", () => {
    const code = 'class="text-shadow-red-500 drop-shadow-black inset-shadow-white inset-ring-gray-100"';
    expect(arbitraryOrPaletteColors(code, tokens)).toEqual([
      "text-shadow-red-500",
      "drop-shadow-black",
      "inset-shadow-white",
      "inset-ring-gray-100",
    ]);
  });

  it("rechaza colores con nombre y variables que no son tokens; acepta las que sí", () => {
    const bad = 'class="text-[red] bg-[white] bg-[var(--x)] text-(--x) bg-(color:--x)"';
    expect(arbitraryOrPaletteColors(bad, tokens)).toEqual([
      "text-[red]",
      "bg-[white]",
      "bg-[var(--x)]",
      "text-(--x)",
      "bg-(color:--x)",
    ]);
    const good =
      'class="text-[color:var(--color-ink)] bg-(--color-ink) font-[600] font-[family-name:var(--font-display)]"';
    expect(arbitraryOrPaletteColors(good, tokens)).toEqual([]);
  });

  it("lee todos los bloques @theme, también inline y static", () => {
    const all = themeTokens("@theme inline {\n  --font-mono: x;\n}\n@theme static {\n  --color-a: #fff;\n}");
    expect([...all.fonts, ...all.colors]).toEqual(["mono", "a"]);
  });

  it("encuentra colores con nombre, en más propiedades y en la última declaración sin `;`", () => {
    const css =
      ".a {\n  border: 1px solid red;\n  background-image: linear-gradient(#000, #fff);\n  text-shadow: 0 0 1px navy;\n  fill: #fff\n}";
    expect(looseCssColors(css)).toEqual([
      "border: 1px solid red",
      "background-image: linear-gradient(#000, #fff)",
      "text-shadow: 0 0 1px navy",
      "fill: #fff",
    ]);
  });

  it("extrae los <style> de un .astro", () => {
    const astro = "---\n---\n<p>x</p>\n<style>\n  p { color: red; }\n</style>";
    expect(looseCssColors(astroStyles(astro))).toEqual(["color: red"]);
  });

  it("encuentra colores sueltos en CSS fuera de @theme", () => {
    const css =
      "@theme {\n  --color-a: #fff;\n}\n.a {\n  color: white;\n  background-color: #123456;\n  border-color: var(--color-a);\n}";
    expect(looseCssColors(css)).toEqual(["color: white", "background-color: #123456"]);
  });
});
