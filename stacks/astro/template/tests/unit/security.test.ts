import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { CSP_DIRECTIVES, SECURITY_HEADERS } from "../../src/security";

const headersFile = readFileSync(join(import.meta.dirname, "../../public/_headers"), "utf8");

describe("WEB-6: CSP estricta", () => {
  it("la CSP nunca permite unsafe-inline ni unsafe-eval", () => {
    const policies = [...CSP_DIRECTIVES, ...Object.values(SECURITY_HEADERS), headersFile].join("\n");
    expect(policies).not.toMatch(/unsafe-inline|unsafe-eval/);
  });

  it("public/_headers envía todas las cabeceras de SECURITY_HEADERS", () => {
    for (const [name, value] of Object.entries(SECURITY_HEADERS)) {
      expect(headersFile, name).toContain(`${name}: ${value}`);
    }
  });
});
