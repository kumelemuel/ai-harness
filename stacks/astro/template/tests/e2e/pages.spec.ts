import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";
import { NOINDEX, ON_DEMAND, REDIRECTS, ROUTES, articlePath, type RouteKey } from "../../src/routes";
import { SECURITY_HEADERS } from "../../src/security";
import { SITE } from "../../src/site";

const STATIC_PAGES = (Object.keys(ROUTES) as RouteKey[])
  .filter((key) => !ON_DEMAND.includes(key))
  .map((key) => ROUTES[key]);
const PAGES = [...STATIC_PAGES, articlePath("bienvenida")];

/** Junta violaciones de CSP, errores de JS y solicitudes a otros orígenes (WEB-6, WEB-7). */
async function watch(page: Page) {
  const problems: string[] = [];
  const origin = new URL(test.info().project.use.baseURL!).origin;
  await page.addInitScript(() => {
    document.addEventListener("securitypolicyviolation", (event) => {
      console.error(`CSP violation: ${event.violatedDirective} blocked ${event.blockedURI || "inline"}`);
    });
  });
  page.on("console", (message) => {
    if (message.type() === "error") problems.push(message.text());
  });
  page.on("pageerror", (error) => problems.push(`pageerror: ${error.message}`));
  page.on("request", (request) => {
    const url = new URL(request.url());
    if (url.protocol.startsWith("http") && url.origin !== origin) problems.push(`third party: ${url.href}`);
  });
  return problems;
}

async function expectNoA11yViolations(page: Page) {
  const { violations } = await new AxeBuilder({ page })
    .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "wcag22aa"])
    .analyze();
  const summary = violations.map(
    (v) => `${v.id} (${v.impact}): ${v.help}\n    ${v.nodes.map((n) => n.target.join(" ")).join("\n    ")}`,
  );
  expect(summary, "axe violations").toEqual([]);
}

for (const path of PAGES) {
  test(`${path}: estructura, metadatos, CSP, terceros y axe`, async ({ page }) => {
    const problems = await watch(page);
    const response = await page.goto(path, { waitUntil: "networkidle" });

    expect(response?.status()).toBe(200);
    for (const [name, value] of Object.entries(SECURITY_HEADERS)) {
      expect(response?.headers()[name.toLowerCase()], name).toBe(value);
    }
    const csp = await page.locator('meta[http-equiv="content-security-policy"]').getAttribute("content");
    expect(csp).toContain("script-src");
    expect(csp).not.toMatch(/unsafe-inline/);

    await expect(page.locator("html")).toHaveAttribute("lang", SITE.lang);
    await expect(page.locator("h1")).toHaveCount(1);
    await expect(page).toHaveTitle(new RegExp(SITE.name));
    await expect(page.locator('meta[name="description"]')).toHaveAttribute("content", /^.{70,170}$/);
    await expect(page.locator('link[rel="canonical"]')).toHaveAttribute("href", new URL(path, SITE.url).href);
    await expect(page.locator('meta[property="og:image"]')).toHaveAttribute("content", /^https:\/\//);
    const jsonLd = await page.locator('script[type="application/ld+json"]').textContent();
    expect(() => JSON.parse(jsonLd ?? "")).not.toThrow();

    const noindex = NOINDEX.some((key) => ROUTES[key] === path);
    await expect(page.locator('meta[name="robots"][content="noindex"]')).toHaveCount(noindex ? 1 : 0);

    // Recorre la página para evaluar también lo que aparece al hacer scroll.
    await page.evaluate(() => window.scrollTo(0, document.body.scrollHeight));
    await expectNoA11yViolations(page);
    expect(problems, "errores de consola, CSP o solicitudes a terceros").toEqual([]);
  });
}

test("el enlace de salto lleva al contenido", async ({ page }) => {
  await page.goto(ROUTES.home);
  await page.keyboard.press("Tab");
  const skip = page.getByRole("link", { name: "Saltar al contenido" });
  await expect(skip).toBeFocused();
  await skip.press("Enter");
  await expect(page.locator("#main")).toBeFocused();
});

test("404: estado, contenido y accesibilidad", async ({ page }) => {
  const response = await page.goto("/no-existe/");
  expect(response?.status()).toBe(404);
  await expect(page.getByRole("link", { name: "Volver al inicio" })).toHaveAttribute("href", ROUTES.home);
  await expectNoA11yViolations(page);
});

test("sitemap: incluye las páginas indexables y excluye las noindex", async ({ request }) => {
  const sitemap = await (await request.get("/sitemap-0.xml")).text();
  for (const path of PAGES.filter((p) => !NOINDEX.some((key) => ROUTES[key] === p))) {
    expect(sitemap).toContain(`<loc>${new URL(path, SITE.url).href}</loc>`);
  }
  for (const key of NOINDEX) expect(sitemap).not.toContain(new URL(ROUTES[key], SITE.url).href);
});

test("robots.txt apunta al sitemap", async ({ request }) => {
  expect(await (await request.get("/robots.txt")).text()).toContain(
    `Sitemap: ${new URL("/sitemap-index.xml", SITE.url).href}`,
  );
});

for (const [from, to] of Object.entries(REDIRECTS)) {
  test(`${from} redirige con 301 a ${to}`, async ({ request }) => {
    const response = await request.get(from, { maxRedirects: 0 });
    expect(response.status()).toBe(301);
    expect(response.headers().location).toBe(to);
  });
}
