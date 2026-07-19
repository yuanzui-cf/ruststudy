import { defineConfig } from "vite";

export default defineConfig({
  base: "./",
  build: {
    // Monaco's editor core is intentionally shipped as the playground's main UI.
    chunkSizeWarningLimit: 3_000,
  },
  worker: {
    format: "es",
  },
});
