import { screen, waitFor, within } from "@testing-library/react";
import { http, HttpResponse } from "msw";
import { describe, expect, it } from "vitest";
import type { PluginList } from "@/api/types";
import { MOCK_ACCOUNTS } from "@/mocks/db";
import { installMockDb, server } from "@/test/msw-server";
import { renderApp, signIn } from "@/test/render-app";

describe("应用 › 插件", () => {
  it("explains the empty list an M0 server returns and points to skins", async () => {
    const db = installMockDb();
    signIn(db, MOCK_ACCOUNTS.admin.username);
    renderApp("/apps/plugins");
    expect(await screen.findByText("还没有安装插件", undefined, { timeout: 5000 })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "前往外观" })).toHaveAttribute("href", "/apps/themes");
  });

  it("lists each plugin with its identifier, runtime in words, extension points and state", async () => {
    const db = installMockDb();
    signIn(db, MOCK_ACCOUNTS.admin.username);
    const list: PluginList = {
      plugins: [
        { id: "org.example.tmdb", name: "TMDB", version: "1.2.0", runtime: "wasm", enabled: true, provides: ["metadata", "sync"] },
        { id: "org.example.whisper", name: "Whisper", version: "2.0.0", runtime: "sidecar", enabled: false, provides: ["x-custom"] },
      ],
    };
    server.use(http.get("/api/v1/plugins", () => HttpResponse.json(list)));
    renderApp("/apps/plugins");

    const section = await screen.findByRole("region", { name: "已安装的插件" }, { timeout: 5000 });
    const [tmdb, whisper] = within(section).getAllByRole("listitem").filter((item) => item.querySelector("h3"));
    expect(within(tmdb!).getByRole("heading", { name: /TMDB/ })).toBeInTheDocument();
    expect(within(tmdb!).getByText("org.example.tmdb")).toBeInTheDocument();
    expect(within(tmdb!).getByText("WASM 组件")).toBeInTheDocument();
    expect(within(tmdb!).getByText("元数据")).toBeInTheDocument();
    expect(within(tmdb!).getByText("已启用")).toBeInTheDocument();
    expect(within(whisper!).getByText(/不在沙箱里/)).toBeInTheDocument();
    expect(within(whisper!).getByText("x-custom")).toBeInTheDocument();
    expect(within(whisper!).getByText("已停用")).toBeInTheDocument();
  });

  it("sends members back to the overview", async () => {
    const db = installMockDb();
    signIn(db, MOCK_ACCOUNTS.user.username);
    const { router } = renderApp("/apps/plugins");
    await waitFor(() => expect(router.state.location.pathname).toBe("/"));
    expect(screen.queryByText("还没有安装插件")).not.toBeInTheDocument();
  });
});
