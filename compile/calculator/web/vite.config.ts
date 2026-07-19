import { defineConfig } from "vite";

export default defineConfig({
  base: "./",
  build: {
    // The complete Monaco editor includes suggest and snippet contributions.
    chunkSizeWarningLimit: 4_000,
  },
  worker: {
    format: "es",
  },
});
