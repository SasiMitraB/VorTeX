import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { fileURLToPath } from "node:url";

const monacoEsm = fileURLToPath(new URL("../node_modules/monaco-editor/esm/vs/", import.meta.url));

// Tauri loads the dev server at a fixed port and the production build from `dist/`.
export default defineConfig({
  plugins: [react()],
  resolve: {
    // monaco-editor's export map appends ".js" to deep imports, which breaks its CSS files.
    alias: [{ find: /^monaco-editor\/(.+\.css)$/, replacement: `${monacoEsm}$1` }],
  },
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**", "**/target/**"] },
  },
  build: {
    // WKWebView on macOS 12+.
    target: "safari15",
    outDir: "dist",
    emptyOutDir: true,
  },
});
