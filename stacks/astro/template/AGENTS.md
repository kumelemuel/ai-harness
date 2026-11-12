# Instrucciones para agentes

Sitio Astro (estático) en Cloudflare Workers, con Tailwind 4. Sigue los principios
`WEB-*` de [craft-web](https://github.com/kumelemuel/ai-harness/blob/main/core/principles/web.md).

## Reglas

- **Rutas (WEB-2):** toda URL interna sale de `src/routes.ts` (`route("contact")`,
  `articlePath(slug)`). Nunca escribas una ruta a mano. Si renombras una URL publicada,
  agrega la 301 en `REDIRECTS`.
- **Contenido (WEB-3):** textos repetidos en `src/site.ts`; artículos en
  `src/content/articles/` con su schema. No incrustes contenido del cliente en componentes.
- **Servidor (WEB-1):** una página nueva es estática. Si de verdad necesita servidor,
  `export const prerender = false` **y** agrégala a `ON_DEMAND`.
- **CSP (WEB-6):** nunca `unsafe-inline`; nada de `style=""`, `onclick=""` ni scripts de
  terceros. Un origen externo nuevo se agrega en `src/security.ts` con su justificación.
- **Privacidad (WEB-7):** nada se carga desde otro origen. Si agregas un proveedor,
  actualiza `src/pages/privacidad.astro`.
- **Formulario (WEB-8):** las reglas viven en `src/lib/contact.ts` con tests; el handler
  (`src/actions/index.ts`) solo orquesta.
- **Código en inglés; contenido y URLs en el idioma del cliente.**

## Antes de terminar

```sh
npm run lint && npm run format:check && npm run check && npm test
npm run build && npm run test:e2e
```

Si un test falla, corrige el código, no el test.
