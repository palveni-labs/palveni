/// <reference types="vitest/config" />
import { readFileSync } from "node:fs";
import path from "node:path";
import tailwindcss from "@tailwindcss/vite";
import { tanstackRouter } from "@tanstack/router-plugin/vite";
import react from "@vitejs/plugin-react";
import { defineConfig, type PluginOption } from "vite";

// Default port of the Palveni server (P-A-L-V on a phone keypad).
const API_TARGET = "http://127.0.0.1:7258";

/** Fonts compiled into the app (the built-in skin's), shipped with their licences. */
const BUNDLED_FONT_LICENSES = [
  { name: "figtree", file: "node_modules/@fontsource-variable/figtree/LICENSE" },
  { name: "fragment-mono", file: "node_modules/@fontsource/fragment-mono/LICENSE" },
];

function fontLicenses(): PluginOption {
  return {
    name: "palveni-font-licenses",
    apply: "build",
    generateBundle() {
      for (const { name, file } of BUNDLED_FONT_LICENSES) {
        this.emitFile({ type: "asset", fileName: `assets/licenses/${name}-OFL.txt`, source: readFileSync(path.resolve(import.meta.dirname, file), "utf8") });
      }
    },
  };
}

export default defineConfig(() => {
  const plugins: PluginOption[] = [
    // Route layout lives in tsr.config.json + src/routes.ts (shared with `tsr generate`).
    tanstackRouter({ target: "react", autoCodeSplitting: true }),
    react(),
    tailwindcss(),
    fontLicenses(),
  ];
  return {
    base: "/",
    plugins,
    resolve: {
      alias: { "@": path.resolve(import.meta.dirname, "src") },
    },
    server: {
      port: 5173,
      fs: { allow: [import.meta.dirname, path.resolve(import.meta.dirname, "../plugins")] },
      proxy: {
        "/api": { target: API_TARGET, changeOrigin: false },
        "/opds": { target: API_TARGET, changeOrigin: false },
      },
    },
    preview: {
      proxy: {
        "/api": { target: API_TARGET, changeOrigin: false },
        "/opds": { target: API_TARGET, changeOrigin: false },
      },
    },
    build: {
      outDir: "dist",
      assetsDir: "assets",
      emptyOutDir: true,
      sourcemap: false,
      target: "es2022",
      chunkSizeWarningLimit: 400,
      rolldownOptions: {
        output: {
          codeSplitting: {
            groups: [
              { name: "react", test: /node_modules[\\/](react|react-dom|scheduler)[\\/]/, priority: 30 },
              { name: "tanstack", test: /node_modules[\\/]@tanstack[\\/]/, priority: 20 },
              { name: "aria", test: /node_modules[\\/](react-aria|react-aria-components|@react-aria|@react-stately|@react-types|@internationalized)[\\/]/, priority: 11 },
              { name: "heroui", test: /node_modules[\\/](@heroui|tailwind-variants|tailwind-merge)[\\/]/, priority: 10 },
            ],
          },
        },
      },
    },
    test: {
      environment: "jsdom",
      globals: true,
      setupFiles: ["./src/test/setup.ts"],
      css: false,
      restoreMocks: true,
      include: ["src/**/*.test.{ts,tsx}"],
    },
  };
});
