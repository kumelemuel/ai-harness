/**
 * Fuente única de rutas (WEB-2). Menú, enlaces, canonical, sitemap, Lighthouse y tests
 * salen de aquí: nunca escribas una ruta interna a mano en el markup.
 */
export const ROUTES = {
  home: "/",
  about: "/sobre/",
  articles: "/articulos/",
  contact: "/contacto/",
  contactSent: "/contacto/enviado/",
  privacy: "/privacidad/",
} as const;

export type RouteKey = keyof typeof ROUTES;

export const route = (key: RouteKey): string => ROUTES[key];

export const articlePath = (slug: string): string => `${ROUTES.articles}${slug}/`;

/** Orden del menú principal. */
export const NAV: readonly RouteKey[] = ["home", "about", "articles", "contact"];

/** Rutas renderizadas en el servidor (WEB-1). Un test verifica que coincidan con las páginas. */
export const ON_DEMAND: readonly RouteKey[] = ["contactSent"];

/** Rutas fuera del sitemap y con `noindex` (WEB-9). */
export const NOINDEX: readonly RouteKey[] = ["contactSent", "privacy"];

/**
 * Redirecciones 301 de URLs publicadas que cambiaron (WEB-2): ruta vieja → ruta nueva.
 * Ejemplo: "/nosotros/": ROUTES.about
 */
export const REDIRECTS: Readonly<Record<string, string>> = {};

export const isIndexable = (path: string): boolean =>
  !NOINDEX.some((key) => path === ROUTES[key] || path.startsWith(ROUTES[key]));
