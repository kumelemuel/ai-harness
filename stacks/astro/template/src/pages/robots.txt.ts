import type { APIRoute } from "astro";
import { SITE } from "../site";

/** robots.txt desde la identidad del sitio (WEB-3): el dominio no se repite a mano. */
export const GET: APIRoute = () =>
  new Response(
    ["User-agent: *", "Allow: /", "", `Sitemap: ${new URL("/sitemap-index.xml", SITE.url).href}`, ""].join(
      "\n",
    ),
    { headers: { "Content-Type": "text/plain; charset=utf-8" } },
  );
