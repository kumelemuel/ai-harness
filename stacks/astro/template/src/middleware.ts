import { defineMiddleware } from "astro:middleware";
import { SECURITY_HEADERS } from "./security";

const CSP = "Content-Security-Policy";

/**
 * Cabeceras de seguridad para las páginas on-demand (WEB-6). Las estáticas las reciben de
 * public/_headers, porque Cloudflare las sirve sin pasar por el Worker.
 *
 * En on-demand, Astro ya envía la CSP con los hashes como cabecera: se le agrega
 * `frame-ancestors` en vez de reemplazarla.
 */
export const onRequest = defineMiddleware(async (_context, next) => {
  const response = await next();
  for (const [name, value] of Object.entries(SECURITY_HEADERS)) {
    const current = response.headers.get(name);
    if (name === CSP && current && !current.includes(value)) {
      response.headers.set(name, `${current.replace(/;\s*$/, "")}; ${value}`);
    } else if (!current) {
      response.headers.set(name, value);
    }
  }
  return response;
});
