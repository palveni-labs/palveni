import "./styles/globals.css";
import "./i18n";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./app";
import { initSkin } from "./skins/store";
import { createAppRouter, createQueryClient } from "./router";

const PRELOAD_RELOAD_KEY = "palveni.preload-reload";

// Recover once when an open tab references hashed JS/CSS chunks replaced by a newer server deploy.
window.addEventListener("vite:preloadError", (event) => {
  try {
    const marker = `${window.location.pathname}${window.location.search}`;
    if (window.sessionStorage.getItem(PRELOAD_RELOAD_KEY) === marker) return;
    window.sessionStorage.setItem(PRELOAD_RELOAD_KEY, marker);
  } catch {
    // Proceed with reload even if sessionStorage is restricted.
  }
  event.preventDefault();
  window.location.reload();
});

void initSkin().then(() => {
  const queryClient = createQueryClient();
  const router = createAppRouter(queryClient);
  const container = document.getElementById("root");
  if (!container) throw new Error("#root missing from index.html");
  createRoot(container).render(
    <StrictMode>
      <App router={router} queryClient={queryClient} />
    </StrictMode>,
  );
});
