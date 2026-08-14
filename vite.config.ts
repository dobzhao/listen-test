import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import path from "node:path";

// Tauri 2.0 推荐 Vite 配置：固定端口、关闭清屏、设置环境变量
export default defineConfig(async () => ({
  plugins: [react()],

  // 防止 Vite 清除终端输出，方便查看 Tauri 日志
  clearScreen: false,
  // Tauri 期望一个固定的端口
  server: {
    port: 1420,
    strictPort: true,
    host: false,
    hmr: {
      protocol: "ws",
      host: "localhost",
      port: 1421,
    },
    watch: {
      // 告诉 Vite 忽略 Rust 编译目标目录
      ignored: ["**/src-tauri/**"],
    },
  },

  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },

  // 环境变量前缀（仅暴露 TAURI_* 等）
  envPrefix: ["VITE_", "TAURI_"],

  build: {
    // Tauri 在生产环境使用 Chromium，目标设为 esnext 即可
    target: "esnext",
    // 不要 minify，方便调试
    minify: !process.env.TAURI_DEBUG ? "esbuild" : false,
    // 生成 sourcemap
    sourcemap: !!process.env.TAURI_DEBUG,
  },
}));
