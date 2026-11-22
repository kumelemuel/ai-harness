/**
 * Detector de AS-2: los colores y las tipografías salen de `@theme`
 * (`src/styles/global.css`), no de valores sueltos ni de la paleta por defecto de
 * Tailwind. Lo usa `theme.test.ts`.
 *
 * Límites: no ve clases armadas en tiempo de ejecución (`` `bg-${color}-500` ``) ni
 * atributos `style=` (los bloquea la CSP, WEB-6), y solo recorre `.astro`, `.ts`,
 * `.tsx`, `.mdx` y `.css` de `src/`.
 */
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";

export interface ThemeTokens {
  colors: Set<string>;
  fonts: Set<string>;
}

/** Contenido de cada bloque `@theme { … }`, también `@theme inline` y `@theme static`. */
const THEME_BLOCK = /@theme(?:\s+[a-z]+)*\s*\{([^}]*)\}/g;

export const themeTokens = (css: string): ThemeTokens => {
  const blocks = [...css.matchAll(THEME_BLOCK)].map((m) => m[1]).join("\n");
  const names = (kind: string) =>
    new Set([...blocks.matchAll(new RegExp(`--${kind}-([a-z0-9-]+)\\s*:`, "g"))].map((m) => m[1]));
  return { colors: names("color"), fonts: names("font") };
};

/** Utilidades de Tailwind 4 cuyo valor es un color. */
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
  "inset-ring",
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
  "inset-shadow",
  "drop-shadow",
  "text-shadow",
];

/** Nombres de la paleta por defecto, leídos del Tailwind instalado. */
const installedPalette = (): Set<string> => {
  const theme = readFileSync(createRequire(import.meta.url).resolve("tailwindcss/theme.css"), "utf8");
  return new Set([...theme.matchAll(/--color-([a-z]+)-\d+\s*:/g)].map((m) => m[1]));
};

const PALETTE_NAMES = installedPalette();

const isPaletteColor = (value: string): boolean => {
  const match = /^([a-z]+)-\d{2,3}$/.exec(value);
  return value === "white" || value === "black" || (!!match && PALETTE_NAMES.has(match[1]));
};

/** Colores con nombre de CSS (sin `transparent`, `currentcolor` ni `inherit`). */
const CSS_NAMED_COLORS = new Set(
  (
    "aliceblue antiquewhite aqua aquamarine azure beige bisque black blanchedalmond blue " +
    "blueviolet brown burlywood cadetblue chartreuse chocolate coral cornflowerblue cornsilk " +
    "crimson cyan darkblue darkcyan darkgoldenrod darkgray darkgreen darkgrey darkkhaki " +
    "darkmagenta darkolivegreen darkorange darkorchid darkred darksalmon darkseagreen " +
    "darkslateblue darkslategray darkslategrey darkturquoise darkviolet deeppink deepskyblue " +
    "dimgray dimgrey dodgerblue firebrick floralwhite forestgreen fuchsia gainsboro ghostwhite " +
    "gold goldenrod gray green greenyellow grey honeydew hotpink indianred indigo ivory khaki " +
    "lavender lavenderblush lawngreen lemonchiffon lightblue lightcoral lightcyan " +
    "lightgoldenrodyellow lightgray lightgreen lightgrey lightpink lightsalmon lightseagreen " +
    "lightskyblue lightslategray lightslategrey lightsteelblue lightyellow lime limegreen linen " +
    "magenta maroon mediumaquamarine mediumblue mediumorchid mediumpurple mediumseagreen " +
    "mediumslateblue mediumspringgreen mediumturquoise mediumvioletred midnightblue mintcream " +
    "mistyrose moccasin navajowhite navy oldlace olive olivedrab orange orangered orchid " +
    "palegoldenrod palegreen paleturquoise palevioletred papayawhip peachpuff peru pink plum " +
    "powderblue purple rebeccapurple red rosybrown royalblue saddlebrown salmon sandybrown " +
    "seagreen seashell sienna silver skyblue slateblue slategray slategrey snow springgreen " +
    "steelblue tan teal thistle tomato turquoise violet wheat white whitesmoke yellow yellowgreen"
  ).split(" "),
);

const COLOR_FUNCTION = /^(?:rgba?|hsla?|oklch|oklab|lab|lch|hwb|color-mix|light-dark)\(/;

/**
 * Valor arbitrario de una utilidad de color, sin los corchetes o paréntesis:
 * `true` si es un color suelto. Acepta solo `var(--color-<token>)` de `@theme`.
 */
const isLooseColor = (inner: string, tokens: ThemeTokens): boolean => {
  const value = inner.replace(/^color:/, "");
  const variable = /^(?:var\()?--([a-z0-9-]+)\)?$/.exec(value);
  if (variable) {
    const token = /^color-(.+)$/.exec(variable[1]);
    return !token || !tokens.colors.has(token[1]);
  }
  if (inner.startsWith("color:")) return true;
  return value.startsWith("#") || COLOR_FUNCTION.test(value) || CSS_NAMED_COLORS.has(value);
};

/** Familias de Tailwind que solo valen si `@theme` las declara. */
const FONT_FAMILIES = new Set(["sans", "serif", "mono"]);

/** `font-[…]` o `font-(…)`: suelto salvo un peso o un token `--font-*` de `@theme`. */
const isLooseFont = (inner: string, tokens: ThemeTokens): boolean => {
  if (/^(?:weight:)?\d+$/.test(inner)) return false;
  const token = /^(?:family-name:)?(?:var\()?--font-([a-z0-9-]+)\)?$/.exec(inner);
  return !token || !tokens.fonts.has(token[1]);
};

/** Variante de Tailwind: `hover:`, `md:`, `aria-[…]:`, `group-hover/item:`, `*:`. */
const VARIANT = /^(?:[a-z0-9-]*\[[^\]]*\](?:\/[a-z0-9-]+)?|[a-z0-9-]+(?:\/[a-z0-9-]+)?|\*{1,2}):/;

/** La clase sin variantes, sin `!` (delante o detrás) y sin `/opacidad`. */
const base = (token: string): string => {
  let rest = token;
  for (;;) {
    const variant = VARIANT.exec(rest);
    if (!variant) break;
    rest = rest.slice(variant[0].length);
  }
  return rest.replace(/^!|!$/g, "").replace(/\/(?:[0-9.]+|\[[^\]]*\]|\([^)]*\))$/, "");
};

/** El valor entre `[…]` o `(…)`, si lo es. */
const bracketed = (value: string): string | undefined =>
  /^\[(.*)\]$/.exec(value)?.[1] ?? /^\((.*)\)$/.exec(value)?.[1];

/** Clases de `code` que usan un color o una tipografía fuera de `@theme`. */
export const arbitraryOrPaletteColors = (code: string, tokens: ThemeTokens): string[] => {
  const found: string[] = [];
  for (const raw of code.split(/[\s"'`{}]+/)) {
    const cls = base(raw);
    const font = /^font-(.+)$/.exec(cls);
    if (font) {
      const value = font[1];
      const inner = bracketed(value);
      if (
        (inner !== undefined && isLooseFont(inner, tokens)) ||
        (FONT_FAMILIES.has(value) && !tokens.fonts.has(value))
      ) {
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
    const inner = bracketed(value);
    if (inner !== undefined ? isLooseColor(inner, tokens) : isPaletteColor(value)) {
      found.push(cls);
    }
  }
  return found;
};

/** Contenido de los bloques `<style>` de un `.astro`. */
export const astroStyles = (astro: string): string =>
  [...astro.matchAll(/<style\b[^>]*>([\s\S]*?)<\/style>/g)].map((m) => m[1]).join("\n");

/** Propiedades CSS cuyo valor es (o incluye) un color. */
const COLOR_PROPERTY =
  /^(?:color|background(?:-color|-image)?|border(?:-(?:top|right|bottom|left|inline|block)(?:-(?:start|end))?)?(?:-color)?|outline(?:-color)?|fill|stroke|text-decoration(?:-color)?|text-shadow|box-shadow|caret-color|accent-color|scrollbar-color|column-rule(?:-color)?)$/;

/** ¿El valor escribe un color a mano? Hexadecimal, función de color o nombre de CSS. */
const hasLiteralColor = (value: string): boolean =>
  /#[0-9a-f]{3,8}\b/i.test(value) ||
  /\b(?:rgba?|hsla?|oklch|oklab|lab|lch|hwb)\(/i.test(value) ||
  value
    .toLowerCase()
    .split(/[^a-z]+/)
    .some((word) => CSS_NAMED_COLORS.has(word));

/** Declaraciones de `css`, fuera de `@theme`, que escriben un color a mano. */
export const looseCssColors = (css: string): string[] => {
  const outside = css.replace(THEME_BLOCK, "").replace(/\/\*[\s\S]*?\*\//g, "");
  return [...outside.matchAll(/([a-z-]+)\s*:\s*([^;{}]+?)\s*(?:;|(?=\}))/gi)]
    .filter(([, property, value]) => COLOR_PROPERTY.test(property) && hasLiteralColor(value))
    .map(([, property, value]) => `${property}: ${value.trim()}`);
};
