# Stack — Astro (sitios informativos)

> **Estado:** v0.1 (2026-11-12)
> **Toolchain:** Astro 7, Node 24, Cloudflare Workers, Tailwind 4.
> **Referencia ejecutable:** [`template/`](template/): build limpio, `astro check` sin errores,
> 22 tests unitarios y 16 e2e en verde. Los tests detectan cada violación inyectada:
> `unsafe-inline`, una página on-demand no declarada, un `<h1>` duplicado, una imagen sin
> `alt` y una fuente de terceros. Lighthouse está configurado pero **aún no se corrió
> localmente**.

Traducción de [`core/principles/web.md`](../../core/principles/web.md) a Astro. Destilado de
kumelemuel.dev (barra de calidad) y kidus-akaal.com (Cloudflare, antispam).

---

## Decisiones del stack

| ID | Decisión | Motivo |
|----|----------|--------|
| AS-1 | **Astro estático + Cloudflare Workers** (`imageService: "compile"`, `session: false`) | Gratis, con rate limiter nativo y Turnstile disponible. Imágenes optimizadas en el build: no hace falta el binding de Images. |
| AS-2 | **Tailwind 4**, con los tokens de marca en `@theme` | Rápido de repetir entre clientes. El CSS sale en archivo, así que es compatible con la CSP estricta. |
| AS-3 | **Código en inglés**; contenido, URLs y mensajes en el idioma del cliente | El código lo lee cualquier desarrollador; el sitio lo lee el cliente. |
| AS-4 | **CSP con los hashes de Astro** (`security.csp`); cabeceras desde `src/security.ts` | Sin `unsafe-inline`. Una sola fuente de política para `_headers`, el middleware y la config. |
| AS-5 | **Formulario = Astro Action + página on-demand de confirmación** | Funciona sin JS. Las reglas son puras y con tests; el handler solo orquesta. |
| AS-6 | **E2E contra el build real en `wrangler dev`** | Prueba páginas estáticas con `_headers` y páginas on-demand con el middleware en el mismo runtime de producción (workerd). Un solo servidor, no dos. |

---

## Traducción de principios

| Principio | En la plantilla | Verificación |
|-----------|-----------------|--------------|
| WEB-1 Estático por defecto | `output: "static"`; `prerender = false` solo en las rutas de `ON_DEMAND` | `routes.test.ts`: las páginas on-demand son exactamente `ON_DEMAND` |
| WEB-2 Una sola fuente de rutas | `src/routes.ts`: `ROUTES`, `route()`, `articlePath()`, `NAV`, `REDIRECTS` (301 en `astro.config.mjs`) | `routes.test.ts`: cada ruta tiene su página, ningún `.astro` escribe `href="/…"` a mano y cada redirección tiene destino; e2e de redirecciones |
| WEB-3 Contenido tipado | `src/site.ts`; colección `articles` con schema (descripción de 70 a 170); `robots.txt` generado desde `SITE` | El build falla si el schema no se cumple; `astro check` |
| WEB-4 Accesibilidad | Un único `<h1>` desde el Layout, `lang`, enlace de salto, `:focus-visible`, `prefers-reduced-motion`, enlaces subrayados | e2e: axe WCAG 2.2 AA en cada página después del scroll; un `<h1>`; enlace de salto |
| WEB-5 Rendimiento | Imágenes en build, fuentes con `fonts` de Astro (servidas desde el propio dominio), JS mínimo | `lighthouserc.cjs` con aserciones `error` |
| WEB-6 CSP estricta | `CSP_DIRECTIVES` + hashes de Astro (`<meta>` en estáticas, cabecera en on-demand); `SECURITY_HEADERS` en `_headers` y en el middleware | `security.test.ts`: sin `unsafe-inline` y `_headers` alineado; e2e: cabeceras, CSP y cero violaciones |
| WEB-7 Privacidad | Sin terceros en el navegador; aviso en `privacidad.astro` | e2e: toda solicitud va al propio origen |
| WEB-8 Formularios | `src/lib/contact.ts` (reglas puras), `src/actions/index.ts` (orquesta), `contacto/enviado.astro` (on-demand), `CONTACT_RATE_LIMITER` | `contact.test.ts`; e2e con y sin JS por la vía del campo trampa |
| WEB-9 SEO | `SeoMeta.astro` (canonical absoluto, OG, JSON-LD `WebSite` + `Organization` + `Article`), sitemap filtrado por `NOINDEX` | e2e: metadatos, `noindex`, sitemap y robots |
| WEB-10 Secretos | `env.schema` en `astro.config.mjs`; `.env.example`; `.gitignore` con `.env*` y `.dev.vars` | El build y el runtime fallan si falta una variable |

---

## Hooks del plugin craft-web

Se activan solos en proyectos con `astro.config.*` y `src/routes.ts`, y usan el Node de
`.nvmrc` (vía nvm):

| Hook | Cuándo | Qué hace | Costo |
|------|--------|----------|-------|
| `PostToolUse` | Tras editar en `src/`, `tests/`, `public/` o la config | Prettier del archivo + `vitest run` | ~5 s |
| `Stop` | Al terminar, si hay cambios sin commit | `eslint`, `astro check`, `vitest run`; bloquea una vez por turno | ~7 s |

El gate de Stop bloquea una sola vez por turno. Si el fallo lo causó el cambio del agente,
lo corrige antes de terminar (VER-1). Si el rojo es intencional (fase roja de TDD) o el
agente cree que el test está mal, no lo modifica: explica el estado y por qué, y termina
(TDD-1).

Los e2e y Lighthouse quedan para CI, porque necesitan un build. Variables:
`CRAFT_WEB_HOOKS=0`, `CRAFT_WEB_STOP_GATE=0`, `CRAFT_WEB_HOOKS_LOG=/ruta`.

## Detalles que costó descubrir

- **CSP en páginas on-demand.** Astro manda la CSP con hashes como **cabecera** en las
  respuestas on-demand, y como `<meta>` en las estáticas. El middleware **agrega**
  `frame-ancestors` a esa cabecera en vez de reemplazarla. Esa directiva no funciona dentro
  de un `<meta>`, así que en las estáticas va en `_headers`.
- **`_headers` no aplica a lo que responde el Worker.** Solo cubre archivos estáticos. Por
  eso existe `src/middleware.ts`.
- **Tailwind 4 quita el subrayado de los enlaces** (preflight). axe falla con
  `link-in-text-block`, así que la plantilla los vuelve a subrayar en `@layer base`.
- **Shiki usa estilos inline**, que la CSP bloquea. Se usa Prism (`markdown.syntaxHighlight`).
- **`startedAt` en una página estática** quedaría con la hora del build. El script lo
  estampa al cargar; sin JS no hay timestamp y solo aplica la trampa.
- **`wrangler dev` en scripts y CI:** usa `--show-interactive-dev-session=false`, pasa los
  secretos ficticios con `--var` y escucha en `127.0.0.1`.

---

## Captcha (opcional): Turnstile

Actívalo cuando aparezca spam que pase la trampa, el tiempo y el límite por IP. **No está
verificado en la plantilla**; los pasos son estos:

1. `astro.config.mjs` → `env.schema`: `TURNSTILE_SECRET_KEY` (server, secret) y
   `PUBLIC_TURNSTILE_SITE_KEY` (client, public).
2. `src/security.ts`: `https://challenges.cloudflare.com` en `script-src` (vía
   `security.csp.scriptDirective.resources`, junto a `'self'`), en `frame-src` y en
   `connect-src`, con un comentario que lo justifique.
3. `ContactForm.astro`: el widget. `src/lib/contact.ts`: la validación de la respuesta de
   `siteverify` como función pura (acción, hostname), con tests.
4. Actualiza `privacidad.astro`: Turnstile es un tercero en el navegador (WEB-7) y el test
   de terceros necesitará ese origen en su lista permitida.

kidus-akaal.com ya lo implementa (`src/pages/api/contacto.ts`).

---

## Alternativa: Vercel

Para un sitio en Vercel (como kumelemuel.dev):

- Adaptador `@astrojs/vercel` con `staticHeaders: true` (la CSP sale como cabecera también
  en las estáticas). Las cabeceras de `SECURITY_HEADERS` van en `vercel.json`.
- Sin rate limiter nativo: quita `CONTACT_RATE_LIMITER` y el import de `cloudflare:workers`,
  o usa un proveedor (Upstash, etc.).
- Los e2e no pueden usar `astro preview` con el adaptador de Vercel. kumelemuel.dev sirve el
  build estático con un servidor propio que aplica las cabeceras, y prueba el formulario
  contra `astro dev`.
- El plan Hobby de Vercel no admite uso comercial: para clientes necesitas Pro.

---

## Violaciones típicas en Astro

| En vez de | Usar |
|-----------|------|
| `href="/contacto/"` en un componente | `route("contact")` |
| `output: "server"` para todo el sitio | `static` + `prerender = false` declarado en `ON_DEMAND` |
| `<link href="https://fonts.googleapis.com/…">` | `fonts` de Astro (descarga en el build) |
| `style="…"` o `onclick="…"` | Clases de Tailwind y `<script>` empaquetado |
| Validar el formulario dentro del handler | `src/lib/contact.ts` con tests |
| `process.env.RESEND_API_KEY` | `import { RESEND_API_KEY } from "astro:env/server"` |
| `robots.txt` con el dominio escrito a mano | `src/pages/robots.txt.ts` desde `SITE` |
| `npm install` sin revisar avisos de `allow-scripts` | Revisar con `npm approve-scripts`: npm 11 no corre los postinstall de `workerd`/`esbuild` hasta que los apruebes. Hoy el build funciona sin ellos, pero conviene saberlo. |
