import { expect, test, type Page } from "@playwright/test";
import { CONTACT_MESSAGES } from "../../src/lib/contact";
import { ROUTES } from "../../src/routes";
import { SECURITY_HEADERS } from "../../src/security";

// El éxito solo se prueba por la vía del campo trampa, que responde con éxito falso sin
// llamar a Resend (WEB-8).

const VALID = {
  name: "Ana Prueba",
  email: "ana@example.com",
  message: "Mensaje de prueba con más de veinte caracteres.",
};

async function fill(page: Page, values: Partial<typeof VALID>) {
  if (values.name !== undefined) await page.getByLabel("Tu nombre").fill(values.name);
  if (values.email !== undefined) await page.getByLabel("Correo electrónico").fill(values.email);
  if (values.message !== undefined) await page.getByLabel("Tu mensaje").fill(values.message);
}

const submit = (page: Page) => page.getByRole("button", { name: "Enviar mensaje" }).click();

for (const javaScriptEnabled of [false, true]) {
  test.describe(javaScriptEnabled ? "con JavaScript" : "sin JavaScript", () => {
    test.use({ javaScriptEnabled });

    test("muestra los errores por campo y conserva lo escrito", async ({ page }) => {
      await page.goto(ROUTES.contact);
      await fill(page, { name: "A", email: "ana@", message: "Hola" });
      const [response] = await Promise.all([page.waitForResponse(/\/contacto\/enviado\//), submit(page)]);

      expect(response.status()).toBe(400);
      // On-demand: Astro envía la CSP con hashes y el middleware le suma `frame-ancestors`.
      for (const [name, value] of Object.entries(SECURITY_HEADERS)) {
        expect(response.headers()[name.toLowerCase()], name).toContain(value);
      }
      const csp = response.headers()["content-security-policy"];
      expect(csp).toContain("script-src");
      expect(csp).not.toMatch(/unsafe-inline/);
      await expect(page.getByLabel("Tu nombre")).toHaveValue("A");
      await expect(page.getByLabel("Tu mensaje")).toHaveValue("Hola");
      await expect(page.getByLabel("Tu nombre")).toHaveAttribute("aria-invalid", "true");
      await expect(page.getByLabel("Tu nombre")).toHaveAccessibleDescription(CONTACT_MESSAGES.nameShort);
      await expect(page.getByLabel("Correo electrónico")).toHaveAccessibleDescription(CONTACT_MESSAGES.email);
      await expect(page.getByLabel("Tu mensaje")).toHaveAccessibleDescription(CONTACT_MESSAGES.messageShort);
    });

    test("un envío válido muestra la confirmación", async ({ page }) => {
      await page.goto(ROUTES.contact);
      await fill(page, VALID);
      await page.locator('input[name="website"]').fill("trampa", { force: true });
      await submit(page);

      await expect(page.getByRole("status")).toHaveText(CONTACT_MESSAGES.success);
      await expect(page.locator('meta[name="robots"][content="noindex"]')).toHaveCount(1);
    });
  });
}

test("con JavaScript, el formulario estampa el momento en que se mostró", async ({ page }) => {
  const before = Date.now();
  await page.goto(ROUTES.contact);
  const startedAt = Number(await page.locator('input[name="startedAt"]').inputValue());
  expect(startedAt).toBeGreaterThanOrEqual(before);
});

test("visitar la confirmación directamente lleva al formulario", async ({ page }) => {
  await page.goto(ROUTES.contactSent);
  await expect(page).toHaveURL(new RegExp(`${ROUTES.contact}$`));
});
