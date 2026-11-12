// @ts-check
import cloudflare from "@astrojs/cloudflare";
import sitemap from "@astrojs/sitemap";
import tailwindcss from "@tailwindcss/vite";
import { defineConfig, envField, fontProviders } from "astro/config";
import { REDIRECTS, isIndexable } from "./src/routes.ts";
import { CSP_DIRECTIVES } from "./src/security.ts";
import { SITE } from "./src/site.ts";

const isBuild = process.argv.includes("build");

// https://astro.build/config
export default defineConfig({
  site: SITE.url,
  // WEB-1: todo estático salvo las rutas con `export const prerender = false` (ver ON_DEMAND).
  output: "static",
  // Cada página vive en /ruta/ (coincide con canonical y sitemap; evita URLs duplicadas).
  trailingSlash: "always",
  // Imágenes optimizadas en el build (WEB-5); en runtime se sirven tal cual, sin binding de Images.
  adapter: cloudflare({ imageService: "compile" }),
  // Sin sesiones: no hacen falta y evitan exigir un KV.
  session: false,
  // WEB-2: redirecciones 301 reales en Cloudflare.
  redirects: Object.fromEntries(
    Object.entries(REDIRECTS).map(([from, to]) => [from, { status: 301, destination: to }]),
  ),
  // WEB-6: Astro agrega el hash de cada script y estilo que emite. Solo en build: en `astro dev`
  // Vite inyecta estilos y scripts inline que una CSP estricta bloquearía.
  security: {
    csp: isBuild && { directives: [...CSP_DIRECTIVES] },
  },
  // Prism emite clases CSS; Shiki (por defecto) usa estilos inline que la CSP estricta bloquea.
  markdown: {
    syntaxHighlight: "prism",
  },
  integrations: [
    // WEB-9: fuera del sitemap las rutas noindex.
    sitemap({ filter: (page) => isIndexable(new URL(page).pathname) }),
  ],
  // WEB-5/WEB-7: fuentes descargadas en el build y servidas desde el propio dominio.
  fonts: [
    {
      provider: fontProviders.google(),
      name: "Inter",
      cssVariable: "--font-body",
      weights: [400, 600],
      styles: ["normal"],
      subsets: ["latin"],
      fallbacks: ["ui-sans-serif", "system-ui", "sans-serif"],
    },
    {
      provider: fontProviders.google(),
      name: "Fraunces",
      cssVariable: "--font-heading",
      weights: [600],
      styles: ["normal"],
      subsets: ["latin"],
      fallbacks: ["ui-serif", "Georgia", "serif"],
    },
  ],
  // WEB-10: toda variable de entorno, declarada con su contexto y acceso.
  env: {
    schema: {
      RESEND_API_KEY: envField.string({ context: "server", access: "secret" }),
      CONTACT_TO_EMAIL: envField.string({ context: "server", access: "secret" }),
      CONTACT_FROM_EMAIL: envField.string({
        context: "server",
        access: "secret",
        default: "Sitio <onboarding@resend.dev>",
      }),
    },
  },
  vite: {
    plugins: [tailwindcss()],
  },
});
