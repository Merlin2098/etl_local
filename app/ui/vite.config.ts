import { svelte } from "@sveltejs/vite-plugin-svelte";
import { defineConfig } from "vitest/config";

// ADR-0005 §3: un proyecto con dos entradas: `app.html` (solo Tauri) y `tablero.html` (Tauri y navegador).
export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    // Desarrollo del tablero: `etl-local tablero` (o la app en debug) sirve la API en el 8787.
    proxy: { "/api": { target: "http://127.0.0.1:8787", changeOrigin: true } },
  },
  build: {
    target: "es2022",
    rollupOptions: { input: { app: "app.html", tablero: "tablero.html" } },
  },
  test: {
    environment: "node",
    include: ["src/**/*.test.ts"],
  },
});
