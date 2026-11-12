import { getCollection, type CollectionEntry } from "astro:content";

/** Artículos publicados, del más reciente al más antiguo. En desarrollo incluye borradores. */
export async function publishedArticles(): Promise<CollectionEntry<"articles">[]> {
  const entries = await getCollection("articles", ({ data }) => import.meta.env.DEV || !data.draft);
  return entries.sort((a, b) => b.data.publishedAt.getTime() - a.data.publishedAt.getTime());
}

export const formatDate = (date: Date): string =>
  date.toLocaleDateString("es-MX", { year: "numeric", month: "long", day: "numeric", timeZone: "UTC" });
