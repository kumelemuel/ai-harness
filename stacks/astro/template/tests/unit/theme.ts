/**
 * Detector de AS-2: los colores y las tipografías salen de `@theme`
 * (`src/styles/global.css`), no de valores sueltos ni de la paleta por defecto de
 * Tailwind. Lo usa `theme.test.ts`.
 */
export interface ThemeTokens {
  colors: Set<string>;
  fonts: Set<string>;
}

/** Contenido del bloque `@theme { … }` (sin bloques anidados). */
const themeBlock = (css: string): string => /@theme\s*\{([^}]*)\}/.exec(css)?.[1] ?? "";

export const themeTokens = (css: string): ThemeTokens => {
  const block = themeBlock(css);
  const names = (kind: string) =>
    new Set([...block.matchAll(new RegExp(`--${kind}-([a-z0-9-]+)\\s*:`, "g"))].map((m) => m[1]));
  return { colors: names("color"), fonts: names("font") };
};

/** Utilidades de Tailwind cuyo valor es un color. */
const COLOR_UTILITIES = [
  "bg",
  "text",
  "border",
  "border-x",
  "border-y",
  "border-t",
  "border-r",
  "border-b",
  "border-l",
  "border-s",
  "border-e",
  "ring",
  "ring-offset",
  "outline",
  "fill",
  "stroke",
  "decoration",
  "divide",
  "placeholder",
  "accent",
  "caret",
  "from",
  "via",
  "to",
  "shadow",
];

/** Colores de la paleta por defecto de Tailwind. */
const PALETTE =
  /^(?:(?:slate|gray|zinc|neutral|stone|red|orange|amber|yellow|lime|green|emerald|teal|cyan|sky|blue|indigo|violet|purple|fuchsia|pink|rose)-\d{2,3}|white|black)$/;

/** Un valor arbitrario que es un color: `[#…]`, `[rgb(…)]`, `[color:…]`… */
const ARBITRARY_COLOR = /^\[(?:#|(?:rgba?|hsla?|oklch|oklab|lab|lch|hwb|color-mix)\(|color:)/;

/** Familias de Tailwind que solo valen si `@theme` las declara. */
const FONT_FAMILIES = new Set(["sans", "serif", "mono"]);

/** La clase sin variantes (`hover:`, `md:`, `aria-[…]:`), sin `!` y sin `/opacidad`. */
const base = (token: string): string => {
  let rest = token;
  for (;;) {
    const variant = /^(?:[a-z0-9-]*\[[^\]]*\]|[a-z0-9-]+):/.exec(rest);
    if (!variant) break;
    rest = rest.slice(variant[0].length);
  }
  return rest.replace(/^!/, "").replace(/\/[0-9.]+$/, "");
};

/** Clases de `code` que usan un color o una tipografía fuera de `@theme`. */
export const arbitraryOrPaletteColors = (code: string, tokens: ThemeTokens): string[] => {
  const found: string[] = [];
  for (const raw of code.split(/[\s"'`{}]+/)) {
    const cls = base(raw);
    const font = /^font-(.+)$/.exec(cls);
    if (font) {
      const value = font[1];
      if (value.startsWith("[") || (FONT_FAMILIES.has(value) && !tokens.fonts.has(value))) {
        found.push(cls);
      }
      continue;
    }
    const utility = COLOR_UTILITIES.filter((u) => cls.startsWith(`${u}-`)).sort(
      (a, b) => b.length - a.length,
    )[0];
    if (!utility) continue;
    const value = cls.slice(utility.length + 1);
    if (tokens.colors.has(value)) continue;
    if (ARBITRARY_COLOR.test(value) || PALETTE.test(value)) found.push(cls);
  }
  return found;
};

/** Propiedades CSS cuyo valor es (o incluye) un color. */
const COLOR_PROPERTY =
  /^(?:color|background(?:-color)?|border(?:-[a-z]+)*-color|border(?:-[a-z]+)?|outline(?:-color)?|fill|stroke|text-decoration(?:-color)?|box-shadow|caret-color|accent-color)$/;

/** Un color escrito a mano: hexadecimal, función de color o `white`/`black`. */
const LITERAL_COLOR = /#[0-9a-f]{3,8}\b|\b(?:rgba?|hsla?|oklch|oklab|lab|lch|hwb)\(|\b(?:white|black)\b/i;

/** Declaraciones de `css`, fuera de `@theme`, que escriben un color a mano. */
export const looseCssColors = (css: string): string[] => {
  const outside = css.replace(/@theme\s*\{[^}]*\}/g, "").replace(/\/\*[\s\S]*?\*\//g, "");
  return [...outside.matchAll(/([a-z-]+)\s*:\s*([^;{}]+);/gi)]
    .filter(([, property, value]) => COLOR_PROPERTY.test(property) && LITERAL_COLOR.test(value))
    .map(([, property, value]) => `${property}: ${value.trim()}`);
};
