import { defineConfig, devices } from "@playwright/test";

const PORT = 4610;

/**
 * Los e2e corren contra el build real servido por `wrangler dev` (workerd): páginas estáticas
 * con sus cabeceras de public/_headers y páginas on-demand con el middleware. Requiere
 * `npm run build` antes.
 */
export default defineConfig({
  testDir: "tests/e2e",
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  reporter: process.env.CI ? [["github"], ["html", { open: "never" }]] : "list",
  use: {
    ...devices["Desktop Chrome"],
    baseURL: `http://127.0.0.1:${PORT}`,
    trace: "retain-on-failure",
  },
  webServer: {
    // Credenciales ficticias: los tests solo llegan al envío por la vía del campo trampa,
    // que nunca llama a Resend.
    command: [
      "wrangler dev",
      `--port ${PORT}`,
      "--ip 127.0.0.1",
      "--show-interactive-dev-session=false",
      "--var RESEND_API_KEY:re_test_dummy",
      "--var CONTACT_TO_EMAIL:test@example.com",
    ].join(" "),
    url: `http://127.0.0.1:${PORT}/`,
    env: { WRANGLER_SEND_METRICS: "false" },
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
  },
});
