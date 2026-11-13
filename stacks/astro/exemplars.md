# Ejemplos canónicos (Astro)

Los archivos de la plantilla que muestran una página y un formulario de punta
a punta. **Lee estos en vez de explorar `template/`.** Abre otro archivo de la
plantilla solo si lo que necesitas no está en esta lista.

Las rutas son relativas a `references/template/`.

## Una página

| Archivo | Qué ilustra |
|---------|-------------|
| `src/routes.ts` | Fuente única de rutas: `ROUTES`, `NAV`, `ON_DEMAND`, `NOINDEX` y `REDIRECTS` (WEB-2) |
| `src/pages/sobre.astro` | Página estática con `Layout`, título y descripción |
| `src/components/Header.astro` | Menú desde `NAV` y la etiqueta de cada ruta en `LABELS` |
| `src/layouts/Layout.astro` | `<h1>`, SEO y `noindex` que recibe cada página |
| `tests/unit/routes.test.ts` | Lo que se verifica de cada ruta: página existente, sin `href` a mano |
| `tests/e2e/pages.spec.ts` | E2E de las páginas con axe (WEB-4) |

## Un formulario

| Archivo | Qué ilustra |
|---------|-------------|
| `src/lib/contact.ts` | Reglas puras del formulario: validación, antispam, armado del correo |
| `tests/unit/contact.test.ts` | Tests unitarios de esas reglas |
| `src/actions/index.ts` | Astro Action que solo orquesta: parsea, aplica reglas, envía y responde |
| `src/components/ContactForm.astro` | El formulario: campos, errores accesibles y envío sin JavaScript |
| `src/pages/contacto/index.astro` | Página del formulario |
| `src/pages/contacto/enviado.astro` | Página on-demand (`prerender = false`, en `ON_DEMAND` y `NOINDEX`) |
| `tests/e2e/contact.spec.ts` | E2E del envío |
| `src/security.ts` | Orígenes de terceros permitidos por la CSP |
