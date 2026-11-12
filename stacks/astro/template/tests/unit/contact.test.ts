import { describe, expect, it } from "vitest";
import {
  CONTACT_MESSAGES,
  MAX_MESSAGE_LENGTH,
  MAX_NAME_LENGTH,
  MIN_FILL_TIME_MS,
  contactEmail,
  contactInput,
  contactSubject,
  escapeHtml,
  isLikelyBot,
} from "../../src/lib/contact";

const valid = {
  name: "Ana Pérez",
  email: "ana@example.com",
  message: "Hola, quisiera cotizar un sitio para mi negocio.",
  website: "",
  startedAt: "1700000000000",
};

/** Campo → primer mensaje de error, como lo muestra el formulario. */
const errorsOf = (input: Record<string, unknown>) => {
  const result = contactInput.safeParse(input);
  if (result.success) return {};
  return Object.fromEntries(result.error.issues.map((issue) => [issue.path[0], issue.message]));
};

describe("contactInput", () => {
  it("accepts a valid message and trims every text field", () => {
    const result = contactInput.parse({ ...valid, name: "  Ana  ", email: " ana@example.com " });
    expect(result.name).toBe("Ana");
    expect(result.email).toBe("ana@example.com");
    expect(result.startedAt).toBe(1700000000000);
  });

  it("reports every invalid field at once", () => {
    expect(errorsOf({ ...valid, name: "A", email: "no-es-correo", message: "corto" })).toEqual({
      name: CONTACT_MESSAGES.nameShort,
      email: CONTACT_MESSAGES.email,
      message: CONTACT_MESSAGES.messageShort,
    });
  });

  it("enforces maximum lengths", () => {
    expect(errorsOf({ ...valid, name: "x".repeat(MAX_NAME_LENGTH + 1) }).name).toBe(
      CONTACT_MESSAGES.nameLong,
    );
    expect(errorsOf({ ...valid, message: "x".repeat(MAX_MESSAGE_LENGTH + 1) }).message).toBe(
      CONTACT_MESSAGES.messageLong,
    );
  });

  it("treats missing fields as empty instead of crashing", () => {
    expect(Object.keys(errorsOf({}))).toEqual(["name", "email", "message"]);
  });
});

describe("isLikelyBot", () => {
  const now = 1_700_000_100_000;

  it("flags the honeypot regardless of timing", () => {
    expect(isLikelyBot({ website: "spam", startedAt: 1 }, { isRpcCall: false, now })).toBe(true);
  });

  it("flags submissions faster than the minimum fill time", () => {
    expect(
      isLikelyBot({ website: "", startedAt: now - MIN_FILL_TIME_MS + 1 }, { isRpcCall: false, now }),
    ).toBe(true);
    expect(isLikelyBot({ website: "", startedAt: now - MIN_FILL_TIME_MS }, { isRpcCall: false, now })).toBe(
      false,
    );
  });

  it("lets plain HTML posts through without a timestamp, but not RPC calls", () => {
    expect(isLikelyBot({ website: "", startedAt: 0 }, { isRpcCall: false, now })).toBe(false);
    expect(isLikelyBot({ website: "", startedAt: 0 }, { isRpcCall: true, now })).toBe(true);
  });
});

describe("email", () => {
  it("escapes HTML in every visitor field", () => {
    expect(escapeHtml(`<img src=x onerror="alert('x')">&`)).toBe(
      "&lt;img src=x onerror=&quot;alert(&#39;x&#39;)&quot;&gt;&amp;",
    );
    const { html } = contactEmail("Sitio", { name: "<b>Ana</b>", email: "a@b.co", message: "<script>" });
    expect(html).not.toContain("<script>");
    expect(html).not.toContain("<b>Ana</b>");
  });

  it("keeps the subject on one line and bounded", () => {
    const subject = contactSubject("Sitio", "Ana\r\nBcc: spam@example.com" + "x".repeat(500));
    expect(subject).not.toMatch(/[\r\n]/);
    expect(subject.length).toBeLessThanOrEqual("[Sitio] Nuevo mensaje de ".length + MAX_NAME_LENGTH);
  });
});
