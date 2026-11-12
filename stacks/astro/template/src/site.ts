/**
 * Identidad del sitio (WEB-3): todo lo que se repite en varias páginas existe una sola vez.
 * Reemplaza estos valores al iniciar un proyecto.
 */
export const SITE = {
  name: "Nombre del Sitio",
  url: "https://example.com",
  /** Idioma del contenido (atributo `lang`) y su variante para Open Graph. */
  lang: "es",
  locale: "es_MX",
  /** Descripción por defecto para buscadores: 140 a 160 caracteres (WEB-9). */
  description:
    "Sitio de ejemplo construido con la plantilla craft-web: estático, accesible, rápido y con un formulario de contacto robusto y sin cookies.",
  email: "contacto@example.com",
  /** Imagen para redes sociales (1200×630, en public/). */
  ogImage: "/og-image.png",
  /** Perfiles públicos: aparecen en el pie de página y en el JSON-LD (`sameAs`). */
  social: [] as { label: string; url: string }[],
} as const;
