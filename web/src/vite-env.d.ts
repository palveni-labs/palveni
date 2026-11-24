/// <reference types="vite/client" />

interface ImportMetaEnv {
  /** "1" when the dev server runs with the in-browser API mock (`npm run dev:mock`). */
  readonly VITE_API_MOCK: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
