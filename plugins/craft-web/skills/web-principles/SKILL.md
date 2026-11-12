---
name: web-principles
description: Barra de calidad para sitios web informativos (WEB-1 a WEB-10): estático por defecto, una sola fuente de rutas, contenido tipado, accesibilidad WCAG 2.2 AA, presupuestos de rendimiento, CSP sin unsafe-inline, privacidad sin terceros, formularios robustos, SEO técnico y secretos. Usar SIEMPRE al crear o modificar páginas, componentes, estilos, contenido, formularios, metadatos o cabeceras de un sitio web informativo o de un cliente, y al revisarlo.
---

# Principios web

Fuente completa con fundamentos: `references/web.md` (`WEB-1` a `WEB-10`). Si vas a
desviarte de una regla, **cita su ID y explica por qué**.

Para Astro, carga también la skill `astro-stack`, que dice cómo se hace cada cosa.

## Reglas operativas

- **WEB-1 Estático:** toda página se genera en el build. Servidor solo para lo
  imprescindible (el envío de un formulario), declarado explícitamente.
- **WEB-2 Rutas:** toda URL interna sale de un módulo único. **Nunca escribas
  `href="/…"` a mano.** Si renombras una URL publicada, agrega una 301.
- **WEB-3 Contenido:** textos, productos y artículos como datos con schema. Lo que se
  repite (marca, correo, redes) existe una sola vez.
- **WEB-4 Accesibilidad:** un solo `<h1>` por página, `lang`, enlace de salto, foco
  visible, `alt` que describe la imagen (vacío si es decorativa), enlaces subrayados en
  el texto, contraste AA, respeto de `prefers-reduced-motion`.
- **WEB-5 Rendimiento:** imágenes optimizadas en el build con dimensiones; fuentes
  servidas desde el propio dominio; nada de JS que no aporte.
- **WEB-6 CSP:** nunca `unsafe-inline` ni `unsafe-eval`. Nada de `style=""`,
  `onclick=""` ni scripts inline sin hash. Un origen externo nuevo se justifica.
- **WEB-7 Privacidad:** nada se carga desde otro origen en el navegador. Si agregas un
  proveedor, **actualiza el aviso de privacidad**.
- **WEB-8 Formularios:** funcionan sin JS; validación en el servidor; antispam en capas;
  **reglas en un módulo puro con tests**, handler que solo orquesta; escapa los datos
  del visitante.
- **WEB-9 SEO:** título único, descripción de 140 a 160 caracteres, canonical absoluto,
  OG con imagen, JSON-LD acorde al contenido visible, sitemap sin páginas noindex.
- **WEB-10 Secretos:** solo en el schema de entorno del servidor; nunca en el cliente ni
  en el repo.

## Lo que ningún test verifica (revísalo tú)

- Que el `alt` describa de verdad la imagen.
- Que la jerarquía de encabezados tenga sentido al leer.
- Que la descripción SEO sea única y atractiva, no relleno.
- Que el JSON-LD diga lo mismo que la página.
- Que el aviso de privacidad coincida con los proveedores reales.

Para una revisión completa, delega en el subagente `site-auditor`.
