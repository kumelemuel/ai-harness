import { defineCollection } from "astro:content";
import { glob } from "astro/loaders";
import { z } from "astro/zod";

/**
 * Artículos (WEB-3). El nombre del archivo es el slug de la URL.
 * Con `draft: true` se ve en desarrollo pero no se publica.
 */
const articles = defineCollection({
  loader: glob({ pattern: "**/*.md", base: "./src/content/articles" }),
  schema: z.object({
    title: z.string().max(70),
    /** Para buscadores y para la lista: 140 a 160 caracteres (WEB-9). */
    description: z.string().min(70).max(170),
    publishedAt: z.coerce.date(),
    updatedAt: z.coerce.date().optional(),
    draft: z.boolean().default(false),
  }),
});

export const collections = { articles };
