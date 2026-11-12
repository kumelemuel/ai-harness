// Lighthouse CI (WEB-5): presupuestos sobre el build de producción servido por `wrangler dev`.
// Después de `npm run build`:  npm run lighthouse
// No es dependencia del proyecto a propósito (arrastra paquetes con avisos de seguridad);
// se ejecuta con una versión fijada vía npx. Fuera de CI, apunta CHROME_PATH a un
// Chrome/Chromium (p. ej. el que instala Playwright).
const PORT = 4620;
// Páginas indexables representativas (las noindex bajan SEO por diseño).
const pages = ["/", "/sobre/", "/articulos/", "/articulos/bienvenida/", "/contacto/"];

module.exports = {
  ci: {
    collect: {
      startServerCommand: [
        "npx wrangler dev",
        `--port ${PORT}`,
        "--ip 127.0.0.1",
        "--show-interactive-dev-session=false",
        "--var RESEND_API_KEY:re_test_dummy",
        "--var CONTACT_TO_EMAIL:test@example.com",
      ].join(" "),
      startServerReadyPattern: "Ready on",
      startServerReadyTimeout: 120000,
      url: pages.map((page) => `http://127.0.0.1:${PORT}${page}`),
      numberOfRuns: 3,
    },
    assert: {
      assertions: {
        "categories:performance": ["error", { minScore: 0.9 }],
        "categories:accessibility": ["error", { minScore: 1 }],
        "categories:best-practices": ["error", { minScore: 0.95 }],
        "categories:seo": ["error", { minScore: 0.9 }],
        "largest-contentful-paint": ["warn", { maxNumericValue: 2500 }],
        "cumulative-layout-shift": ["error", { maxNumericValue: 0.1 }],
        "total-blocking-time": ["warn", { maxNumericValue: 200 }],
      },
    },
    upload: { target: "filesystem", outputDir: ".lighthouseci" },
  },
};
