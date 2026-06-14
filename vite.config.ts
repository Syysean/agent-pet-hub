import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import path from "path";

const frontendDist = "../dist";

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
      "@agent-pet-hub/protocol": path.resolve(__dirname, "./packages/protocol/src/index.ts"),
    },
  },
  // Tauri 要求开发模式下也构建前端到指定目录
  build: {
    outDir: frontendDist,
    emptyOutDir: true,
  },
  server: {
  port: 1420,
  strictPort: true,
},
});
