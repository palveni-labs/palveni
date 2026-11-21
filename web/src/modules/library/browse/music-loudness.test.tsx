import { toast } from "@heroui/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { http, HttpResponse } from "msw";
import { expect, it, vi } from "vitest";
import { queryKeys } from "@/api/query-keys";
import { MOCK_ACCOUNTS } from "@/mocks/db";
import { installMockDb, server } from "@/test/msw-server";
import { signIn } from "@/test/render-app";
import { AnalyzeLoudness } from "./music-loudness";

it("confirms the action the administrator requested when the job event precedes the HTTP reply", async () => {
  signIn(installMockDb(), MOCK_ACCOUNTS.admin.username);
  const client = new QueryClient();
  const key = queryKeys.jobs.list(50);
  client.setQueryData(key, { jobs: [] });
  let onReceipt!: () => void;
  let release!: () => void;
  const received = new Promise<void>((resolve) => { onReceipt = resolve; });
  const reply = new Promise<void>((resolve) => { release = resolve; });
  server.use(http.post("/api/v1/libraries/one/music/loudness", async () => {
    onReceipt();
    await reply;
    return HttpResponse.json({ jobId: "job" }, { status: 202 });
  }));
  const success = vi.spyOn(toast, "success").mockImplementation(() => "toast");
  try {
    render(<QueryClientProvider client={client}><AnalyzeLoudness libraryId="one" /></QueryClientProvider>);
    await userEvent.click(screen.getByRole("button", { name: "分析响度" }));
    await received;
    act(() => {
      client.setQueryData(key, { jobs: [{ id: "job", kind: "music.loudness", subject: { id: "one" }, state: "running" }] });
    });
    await screen.findByRole("button", { name: "停止响度分析" });
    release();
    await waitFor(() => expect(success).toHaveBeenCalledWith("响度分析已加入任务队列"));
  } finally {
    release();
    success.mockRestore();
    client.clear();
  }
});
