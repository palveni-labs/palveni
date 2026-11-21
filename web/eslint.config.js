import js from "@eslint/js";
import reactHooks from "eslint-plugin-react-hooks";
import reactRefresh from "eslint-plugin-react-refresh";
import globals from "globals";
import tseslint from "typescript-eslint";

export default tseslint.config(
  { ignores: ["dist", "coverage", "src/routeTree.gen.ts"] },
  {
    files: ["**/*.{ts,tsx}"],
    extends: [js.configs.recommended, ...tseslint.configs.recommendedTypeChecked],
    languageOptions: {
      ecmaVersion: 2023,
      globals: globals.browser,
      parserOptions: {
        projectService: true,
        tsconfigRootDir: import.meta.dirname,
      },
    },
    plugins: {
      "react-hooks": reactHooks,
      "react-refresh": reactRefresh,
    },
    rules: {
      ...reactHooks.configs.recommended.rules,
      "react-refresh/only-export-components": ["warn", { allowConstantExport: true, allowExportNames: ["Route"] }],
      "@typescript-eslint/no-unused-vars": ["error", { argsIgnorePattern: "^_", varsIgnorePattern: "^_" }],
      "@typescript-eslint/consistent-type-imports": ["error", { fixStyle: "inline-type-imports" }],
      "@typescript-eslint/no-misused-promises": ["error", { checksVoidReturn: { attributes: false } }],
      // Route guards throw TanStack Router redirects, which are intentionally not Error objects.
      "@typescript-eslint/only-throw-error": ["error", { allow: [{ from: "package", package: "@tanstack/router-core", name: "Redirect" }] }],
    },
  },
  {
    // Host components and stores export their hooks next to the components that use them.
    files: ["src/ui/**/*.tsx", "src/skins/**/*.tsx", "src/shell/inbox-store.tsx", "src/shell/status.tsx"],
    rules: { "react-refresh/only-export-components": "off" },
  },
  {
    files: ["**/*.test.{ts,tsx}", "src/test/**", "src/mocks/**"],
    rules: { "react-refresh/only-export-components": "off" },
  },
  {
    files: ["vite.config.ts", "eslint.config.js"],
    languageOptions: { globals: globals.node },
  },
);
