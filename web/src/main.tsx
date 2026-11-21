import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "./styles/globals.css";
import "./i18n";
import { App } from "./app";
import { initSkin } from "./skins/store";
import { createAppRouter, createQueryClient } from "./router";

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
