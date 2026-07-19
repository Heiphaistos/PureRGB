import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

export default defineConfig({
  plugins: [vue()],
  clearScreen: false,
  server: {
    port: 1430,
    strictPort: true,
  },
  build: {
    target: "chrome110",
    // vite 8 (rolldown) : minification oxc intégrée, esbuild retiré
    minify: true,
  },
});
