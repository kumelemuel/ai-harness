/// <reference types="vitest/config" />
import { getViteConfig } from "astro/config";

// Config de Vite de Astro para que `astro/zod` y los imports del proyecto resuelvan igual que en la app.
export default getViteConfig({
  test: {
    include: ["tests/unit/**/*.test.ts"],
  },
});
