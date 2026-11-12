# Plantilla de sitio — craft-web

Sitio informativo Astro 7, estático, en Cloudflare Workers, con Tailwind 4. Implementa los
principios [`WEB-*`](../../../core/principles/web.md) y los verifica en CI. Guía completa:
[`stacks/astro`](../README.md).

## Requisitos

- Node.js 24 (`.nvmrc`; Astro 7 requiere ≥ 22.23)
- Cuenta de Cloudflare (gratis) y de [Resend](https://resend.com) para el formulario

```sh
nvm use
npm install
npx playwright install chromium
cp .env.example .dev.vars   # completa RESEND_API_KEY y CONTACT_TO_EMAIL
npm run dev                 # http://localhost:4321
```

## Scripts

| Comando              | Acción                                                                  |
| :------------------- | :---------------------------------------------------------------------- |
| `npm run dev`        | Servidor de desarrollo                                                  |
| `npm run build`      | Build de producción en `dist/`                                          |
| `npm run preview`    | Build + `wrangler dev` (el sitio como en producción, con sus cabeceras) |
| `npm run deploy`     | Build + `wrangler deploy`                                               |
| `npm run types`      | Regenera `worker-configuration.d.ts` tras cambiar `wrangler.jsonc`      |
| `npm run check`      | `astro check`                                                           |
| `npm run lint`       | ESLint                                                                  |
| `npm run format`     | Prettier (`format:check` solo verifica)                                 |
| `npm test`           | Unitarias (Vitest): formulario, rutas, seguridad                        |
| `npm run test:e2e`   | E2E (Playwright + axe) contra el build en `wrangler dev`; requiere build |
| `npm run lighthouse` | Presupuestos de Lighthouse sobre el build; fuera de CI usa `CHROME_PATH` |

## Nuevo proyecto

1. `src/site.ts`: nombre, dominio, descripción (140–160 caracteres), correo, redes.
2. `wrangler.jsonc`: `name` del Worker; luego `npm run types`.
3. `src/styles/global.css`: tokens de marca en `@theme`. Valida el contraste de cada color
   usado como texto.
4. `public/og-image.png` (1200×630) y `public/favicon.svg`.
5. `src/routes.ts`: páginas del sitio; crea su archivo en `src/pages/`.
6. `src/pages/privacidad.astro`: revisa el aviso con el cliente.
7. Secretos de producción: `wrangler secret put RESEND_API_KEY` y `CONTACT_TO_EMAIL`.
8. GitHub: secretos `CLOUDFLARE_API_TOKEN` y `CLOUDFLARE_ACCOUNT_ID` para el deploy.

## Estructura

```text
src/
├── actions/       Astro Action del formulario (orquesta: límite, antispam, envío)
├── components/    Header, Footer, SeoMeta (meta, OG, JSON-LD), ContactForm
├── content/       Colecciones con schema (artículos)
├── layouts/       Layout base (lang, skip link, único <h1>)
├── lib/           Lógica pura con tests (reglas del formulario)
├── pages/         Una página por ruta de routes.ts; contacto/enviado es on-demand
├── styles/        Tailwind + tokens de marca
├── utils/         Consultas a colecciones
├── middleware.ts  Cabeceras de seguridad en páginas on-demand
├── routes.ts      Fuente única de URLs, on-demand, noindex y redirecciones
├── security.ts    Fuente única de CSP y cabeceras
└── site.ts        Identidad del sitio
tests/
├── unit/          contact, routes (WEB-1/2/9), security (WEB-6)
└── e2e/           pages (estructura, SEO, CSP, terceros, axe), contact (con y sin JS)
```
