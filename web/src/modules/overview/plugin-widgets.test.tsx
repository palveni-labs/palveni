import { screen } from "@testing-library/react";
import { http, HttpResponse } from "msw";
import { describe, expect, it } from "vitest";
import type { PluginList } from "@/api/types";
import { MOCK_ACCOUNTS } from "@/mocks/db";
import { ALL_FEATURES, installMockDb, server } from "@/test/msw-server";
import { renderApp, signIn } from "@/test/render-app";

describe("Overview installed widget plugins", () => {
  it("renders enabled widget plugins on Overview and skips disabled ones", async () => {
    const db = installMockDb({ features: ALL_FEATURES });
    signIn(db, MOCK_ACCOUNTS.admin.username);

    const list: PluginList = {
      plugins: [
        {
          id: "palveni.widget.clock_weather",
          name: "Home Overview Clock & Media Summary Widget",
          version: "1.0.0",
          runtime: "widget",
          enabled: true,
          provides: ["widget"],
          settings: { use24Hour: true, showStorageBadge: true },
        },
        {
          id: "palveni.widget.media_quote",
          name: "Daily Film & Book Quote Widget",
          version: "1.0.0",
          runtime: "widget",
          enabled: true,
          provides: ["widget"],
          settings: { sourceDomain: "all" },
        },
        {
          id: "org.example.disabled_widget",
          name: "Disabled Widget",
          version: "0.1.0",
          runtime: "widget",
          enabled: false,
          provides: ["widget"],
        },
      ],
    };
    server.use(http.get("/api/v1/plugins", () => HttpResponse.json(list)));

    renderApp("/");

    const container = await screen.findByTestId("overview-plugin-widgets", undefined, { timeout: 5000 });
    expect(container.querySelector('[data-plugin-widget="palveni.widget.clock_weather"]')).not.toBeNull();
    expect(container.querySelector('[data-plugin-widget="palveni.widget.media_quote"]')).not.toBeNull();
    expect(container.querySelector('[data-plugin-widget="org.example.disabled_widget"]')).toBeNull();
  });
});
