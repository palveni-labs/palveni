import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { ALL_FEATURES, installMockDb } from "@/test/msw-server";
import { renderApp, signIn } from "@/test/render-app";

describe("maintenance doctor, live logs, metadata config, and protocol endpoints", () => {
  it("renders system doctor checks, filters live logs by level, saves masked metadata keys, and shows WebDAV/Prometheus URLs", async () => {
    const db = installMockDb({ features: ALL_FEATURES });
    signIn(db, "admin");
    renderApp("/system/maintenance");
    const user = userEvent.setup();

    // 1. System Doctor checks render & refresh
    expect(await screen.findByText("系统体检", {}, { timeout: 5000 })).toBeInTheDocument();
    expect(await screen.findByText("Database")).toBeInTheDocument();
    expect(screen.getByText("FFmpeg & Transcoding")).toBeInTheDocument();
    expect(screen.getByText("Storage & Mounts")).toBeInTheDocument();
    expect(screen.getByText("Libraries & Index")).toBeInTheDocument();
    expect(screen.getByText("Protocol Surfaces")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "重新体检" }));
    expect(await screen.findByText(/sqlite connected/i)).toBeInTheDocument();

    // 2. Live server logs filter by WARN and ERROR
    expect(screen.getByText("运行日志")).toBeInTheDocument();
    expect(
      await screen.findByText(/Rejected unauthenticated WebDAV Basic Auth request/),
    ).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "ERROR" }));
    await waitFor(() =>
      expect(
        screen.queryByText(/Rejected unauthenticated WebDAV Basic Auth request/),
      ).not.toBeInTheDocument(),
    );
    expect(await screen.findByText(/ffprobe timed out on sample stream/)).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "WARN" }));
    expect(
      await screen.findByText(/Rejected unauthenticated WebDAV Basic Auth request/),
    ).toBeInTheDocument();

    // 3. Metadata Provider Keys save & mask
    const tmdbInput = await screen.findByLabelText("TMDB API Key / Read Access Token");
    await user.type(tmdbInput, "mock-tmdb-secret-123");
    await user.click(screen.getByRole("button", { name: "保存配置" }));

    await waitFor(() => expect(tmdbInput).toHaveValue(""));
    expect(await screen.findByText("已配置")).toBeInTheDocument();

    // 4. WebDAV & Prometheus copyable endpoints
    expect(screen.getByText("WebDAV 与监控接入")).toBeInTheDocument();
    expect(screen.getByText(/\/webdav\/$/)).toBeInTheDocument();
    expect(screen.getByText(/\/api\/v1\/system\/prometheus$/)).toBeInTheDocument();
  });
});
