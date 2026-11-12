/**
 * Política de seguridad (WEB-6). Fuente única para:
 * - `astro.config.mjs`: la CSP con los hashes que Astro genera (en un <meta> por página).
 * - `public/_headers`: cabeceras de las páginas estáticas (Cloudflare). Un test verifica que coincidan.
 * - `src/middleware.ts`: las mismas cabeceras en las páginas on-demand.
 *
 * Nunca `unsafe-inline` ni `unsafe-eval` (un test lo verifica). Cada origen externo se justifica.
 */
export const CSP_DIRECTIVES = [
  "default-src 'self'",
  "img-src 'self' data:",
  "font-src 'self'",
  "connect-src 'self'",
  "object-src 'none'",
  "base-uri 'self'",
  "form-action 'self'",
  "upgrade-insecure-requests",
] as const;

export const SECURITY_HEADERS: Readonly<Record<string, string>> = {
  "Strict-Transport-Security": "max-age=63072000; includeSubDomains; preload",
  "X-Content-Type-Options": "nosniff",
  "X-Frame-Options": "DENY",
  "Referrer-Policy": "strict-origin-when-cross-origin",
  "Permissions-Policy": "camera=(), microphone=(), geolocation=(), interest-cohort=()",
  // `frame-ancestors` no funciona en <meta>: va como cabecera aparte.
  "Content-Security-Policy": "frame-ancestors 'none'",
};
