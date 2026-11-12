# Checklist de entrega — {Cliente}

> Se completa antes de anunciar el sitio. Cada ítem marcado tiene evidencia (enlace,
> captura o salida de un comando).

## Calidad (CI en verde en `main`)

- [ ] `lint`, `format:check`, `check`, unitarias y build
- [ ] E2E: axe sin violaciones, CSP sin violaciones, cero terceros, formulario con y sin JS
- [ ] Lighthouse: rendimiento ≥ 90, accesibilidad = 100, buenas prácticas ≥ 95, SEO ≥ 90
- [ ] `npm audit --audit-level=high` sin hallazgos

## Contenido

- [ ] Todos los textos aprobados por el cliente, sin texto de relleno de la plantilla
- [ ] `SITE` completo: nombre, dominio, descripción de 140 a 160 caracteres, correo, redes
- [ ] Cada imagen con `alt` que describe lo que muestra (revisado por una persona)
- [ ] `og-image.png` propia (1200×630) y favicon
- [ ] Aviso de privacidad revisado y coherente con lo que el sitio hace

## Producción

- [ ] Dominio apuntando a Cloudflare; HTTPS activo; `www` → dominio principal (o al revés)
- [ ] Secretos de producción: `RESEND_API_KEY`, `CONTACT_TO_EMAIL` (y `CONTACT_FROM_EMAIL`
      con un dominio verificado en Resend: SPF y DKIM)
- [ ] **Formulario probado en producción con un envío real**, recibido en el buzón del cliente
- [ ] Redirecciones 301 del sitio anterior verificadas una por una
- [ ] Sitemap enviado en Google Search Console; propiedad verificada
- [ ] Analítica activa, si se acordó
- [ ] Rulesets de ramas: `main` solo por PR con CI en verde

## Traspaso

- [ ] El cliente tiene acceso a: registrador del dominio, Cloudflare, Resend, repositorio
- [ ] Capacitación: cómo publicar un artículo o editar un producto
- [ ] Documento con contactos, accesos (sin contraseñas) y qué hacer si el formulario deja
      de llegar
