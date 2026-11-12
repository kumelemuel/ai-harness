/**
 * Reglas del formulario de contacto (WEB-8): validación, antispam y armado del correo.
 * Sin imports del runtime de Astro ni del proveedor de correo, para poder probarlas
 * con tests unitarios. El handler (src/actions/index.ts) solo orquesta.
 */
import { z } from "astro/zod";

export const MIN_FILL_TIME_MS = 3000;
export const MIN_NAME_LENGTH = 2;
export const MAX_NAME_LENGTH = 120;
export const MAX_EMAIL_LENGTH = 254;
export const MIN_MESSAGE_LENGTH = 20;
export const MAX_MESSAGE_LENGTH = 5000;

export const CONTACT_MESSAGES = {
  nameShort: `El nombre debe tener al menos ${MIN_NAME_LENGTH} caracteres.`,
  nameLong: `El nombre no puede superar los ${MAX_NAME_LENGTH} caracteres.`,
  email: "Revisa que el correo tenga un formato válido.",
  messageShort: `El mensaje debe tener al menos ${MIN_MESSAGE_LENGTH} caracteres.`,
  messageLong: `El mensaje no puede superar los ${MAX_MESSAGE_LENGTH} caracteres.`,
  success: "Gracias por escribir. Te responderemos a la brevedad.",
  sendFailed: "No pudimos enviar tu mensaje. Inténtalo de nuevo en unos minutos.",
  tooMany: "Recibimos varios mensajes seguidos. Espera un minuto e inténtalo de nuevo.",
} as const;

export type ContactField = "name" | "email" | "message";

const trimmed = (value: unknown) => (typeof value === "string" ? value.trim() : "");

export const contactInput = z
  .object({
    name: z.preprocess(trimmed, z.string()),
    email: z.preprocess(trimmed, z.string()),
    message: z.preprocess(trimmed, z.string()),
    // Antispam: campo trampa que una persona nunca llena, y momento en que se mostró el formulario.
    website: z.preprocess(trimmed, z.string()),
    startedAt: z.preprocess((value) => Number(value) || 0, z.number()),
  })
  .superRefine((data, ctx) => {
    const issue = (path: ContactField, message: string) =>
      ctx.addIssue({ code: "custom", path: [path], message });

    if (data.name.length < MIN_NAME_LENGTH) issue("name", CONTACT_MESSAGES.nameShort);
    else if (data.name.length > MAX_NAME_LENGTH) issue("name", CONTACT_MESSAGES.nameLong);

    if (data.email.length > MAX_EMAIL_LENGTH || !z.email().safeParse(data.email).success) {
      issue("email", CONTACT_MESSAGES.email);
    }

    if (data.message.length < MIN_MESSAGE_LENGTH) issue("message", CONTACT_MESSAGES.messageShort);
    else if (data.message.length > MAX_MESSAGE_LENGTH) issue("message", CONTACT_MESSAGES.messageLong);
  });

export type ContactInput = z.infer<typeof contactInput>;

/**
 * A un bot se le responde con un éxito falso y no se envía nada (WEB-8).
 * - Campo trampa: una persona nunca llena `website`.
 * - Tiempo de llenado: el script de la página estampa `startedAt`. Las llamadas por JS
 *   (/_actions/…) deben traerlo; un envío HTML sin JS no puede, así que ahí solo aplica la trampa.
 */
export function isLikelyBot(
  input: Pick<ContactInput, "website" | "startedAt">,
  { isRpcCall, now = Date.now() }: { isRpcCall: boolean; now?: number },
): boolean {
  if (input.website.length > 0) return true;
  if (!input.startedAt) return isRpcCall;
  return now - input.startedAt < MIN_FILL_TIME_MS;
}

export const escapeHtml = (value: string): string =>
  value
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");

/** Asunto a partir de datos del visitante: sin saltos de línea (inyección de cabeceras) y acotado. */
export const contactSubject = (siteName: string, name: string): string =>
  `[${siteName}] Nuevo mensaje de ${name.replace(/[\r\n]+/g, " ").slice(0, MAX_NAME_LENGTH)}`;

/** Correo listo para el proveedor: todo dato del visitante va escapado en el HTML. */
export function contactEmail(siteName: string, input: Pick<ContactInput, ContactField>) {
  return {
    subject: contactSubject(siteName, input.name),
    text: [`Nombre: ${input.name}`, `Correo: ${input.email}`, "", input.message].join("\n"),
    html: [
      `<p><strong>Nombre:</strong> ${escapeHtml(input.name)}</p>`,
      `<p><strong>Correo:</strong> ${escapeHtml(input.email)}</p>`,
      "<hr />",
      `<p style="white-space: pre-wrap">${escapeHtml(input.message)}</p>`,
    ].join("\n"),
  };
}
