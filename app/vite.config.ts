import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// Config recommended by Tauri: fixed port and no terminal clearing.
export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  build: {
    target: "es2022",
    // Three pages: the main window, the overlay drawn over the game and the in-game toolbar
    rollupOptions: {
      input: {
        main: "index.html",
        overlay: "overlay.html",
        toolbar: "toolbar.html",
      },
    },
  },
});
