import tailwindcss from "@tailwindcss/vite";
import { defineConfig, passthroughImageService } from "astro/config";

export default defineConfig({
  site: "https://kerbaras.github.io",
  base: "/mangas",
  trailingSlash: "ignore",
  image: { service: passthroughImageService() },
  vite: { plugins: [tailwindcss()] },
});
