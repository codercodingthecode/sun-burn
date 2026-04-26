import { defineConfig } from "vite";
import solid from "vite-plugin-solid";

// Tauri expects a fixed port in dev
const host = process.env["TAURI_DEV_HOST"];

export default defineConfig({
  plugins: [solid()],
  clearScreen: false,
  server: {
    host: host ?? "localhost",
    port: 1420,
    strictPort: true,
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
  build: {
    target: ["es2021", "chrome105", "safari13"],
    minify: !process.env["TAURI_DEBUG"] ? "esbuild" : false,
    sourcemap: !!process.env["TAURI_DEBUG"],
  },
});
