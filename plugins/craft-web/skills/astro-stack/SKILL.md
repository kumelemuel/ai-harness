---
name: astro-stack
description: Cómo se construye un sitio informativo con la plantilla craft-web en Astro 7, Cloudflare Workers y Tailwind 4 (`src/routes.ts`, `src/site.ts`, `src/security.ts`, Astro Actions, `wrangler.jsonc`). Usar SIEMPRE al trabajar en un proyecto Astro que tenga `src/routes.ts`, al crear una página, colección, formulario o cabecera, y al arrancar un sitio nuevo de cliente. Se usa junto con web-principles.
---

# Astro (craft-web)

Complementa `web-principles`: esa skill dice **qué**; esta dice **cómo en Astro**.
**Carga también `web-principles` antes de crear o modificar algo** (si no está
cargada): esta skill no repite sus reglas.

- `references/astro.md`: guía completa (decisiones AS-1 a AS-6, tabla WEB → mecanismo,
  detalles que costó descubrir, Turnstile, alternativa Vercel).
- `references/exemplars.md`: índice de los archivos canónicos de una página y un
  formulario, con qué ilustra cada uno. **Antes de crear algo, lee el ejemplo
  equivalente del índice y copia su forma.**
- `references/template/`: el código de la plantilla, que pasa todos los tests. No lo
  explores: ábrelo solo si el índice no cubre lo que necesitas.

## Dónde va cada cosa

| Necesito… | Archivo |
|-----------|---------|
| Una página nueva | Clave en `ROUTES` (`src/routes.ts`) + archivo en `src/pages/` + etiqueta en `LABELS` (`Header.astro`; `astro check` falla si falta); agrégala a `NAV` si va en el menú |
| Un enlace interno | `route("clave")` o `articlePath(slug)`. Un test falla si escribes `href="/…"` |
| Renombrar una URL | Cambia `ROUTES` y agrega la ruta vieja a `REDIRECTS` (301) |
| Una página on-demand | `export const prerender = false` **y** su clave en `ON_DEMAND` (un test lo verifica) |
| Excluir del sitemap | Clave en `NOINDEX` + prop `noindex` del Layout |
| Nombre, dominio, correo, redes | `src/site.ts` |
| Colores y tipografía | `@theme` en `src/styles/global.css`; fuentes en `fonts` de `astro.config.mjs` |
| Contenido repetible | Colección en `src/content.config.ts` con schema |
| Origen externo (captcha, mapa) | `src/security.ts` + el test de terceros + `privacidad.astro` |
| Variable de entorno | `env.schema` en `astro.config.mjs`; `import … from "astro:env/server"` |
| Regla de un formulario | `src/lib/` (puro, con test en `tests/unit/`); la Action solo orquesta |

## Convenciones

- Código en inglés; contenido, URLs y mensajes en el idioma del cliente.
- Clases de Tailwind; nunca `style=""`. Scripts como `<script>` (Astro los empaqueta).
- El `<h1>` lo pone el Layout (`title`); la portada usa `hideTitle` y pone el suyo.
- Node 24 (`.nvmrc`). Los e2e corren contra el build: `npm run build` antes.

## Antes de terminar

```sh
npm run lint && npm run format:check && npm run check && npm test
npm run build && npm run test:e2e
```
