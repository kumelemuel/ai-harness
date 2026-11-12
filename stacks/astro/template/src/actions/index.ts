import { ActionError, defineAction } from "astro:actions";
import { CONTACT_FROM_EMAIL, CONTACT_TO_EMAIL, RESEND_API_KEY } from "astro:env/server";
import { env } from "cloudflare:workers";
import { Resend } from "resend";
import { CONTACT_MESSAGES, contactEmail, contactInput, isLikelyBot } from "../lib/contact";
import { SITE } from "../site";

/**
 * El handler solo orquesta (WEB-8): límite por IP → antispam → envío. Las reglas viven en
 * src/lib/contact.ts. La validación del input ocurre antes, con `contactInput`.
 */
export const server = {
  contact: defineAction({
    accept: "form",
    input: contactInput,
    handler: async (input, context) => {
      const ip = context.request.headers.get("CF-Connecting-IP") ?? "unknown";
      const { success: withinLimit } = await env.CONTACT_RATE_LIMITER.limit({ key: ip });
      if (!withinLimit) {
        throw new ActionError({ code: "TOO_MANY_REQUESTS", message: CONTACT_MESSAGES.tooMany });
      }

      // Bots: éxito falso para que no reintenten; no se envía nada.
      if (isLikelyBot(input, { isRpcCall: context.url.pathname.startsWith("/_actions/") })) {
        return { message: CONTACT_MESSAGES.success };
      }

      const { error } = await new Resend(RESEND_API_KEY).emails.send({
        from: CONTACT_FROM_EMAIL,
        to: CONTACT_TO_EMAIL,
        replyTo: input.email,
        ...contactEmail(SITE.name, input),
      });

      if (error) {
        console.error("[contact] Resend error:", error.name);
        throw new ActionError({ code: "INTERNAL_SERVER_ERROR", message: CONTACT_MESSAGES.sendFailed });
      }

      return { message: CONTACT_MESSAGES.success };
    },
  }),
};
