import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { MOCK_ACCOUNTS } from "@/mocks/db";
import { installMockDb } from "@/test/msw-server";
import { renderApp, signIn } from "@/test/render-app";

// The mock's 剧集 library was last scanned while one of its two paths was unplugged
// (`scan.summary` with `unavailablePaths: 1`).
const GAPS = "1 个路径不可用，其中的条目保持不变";

describe("a scan that could not read everything", () => {
  it("is marked in the task list and explained in its detail", async () => {
    const db = installMockDb(); // M0: tasks come from the jobs API
    signIn(db, MOCK_ACCOUNTS.admin.username);
    const user = userEvent.setup();
    renderApp("/tasks");

    const finished = await screen.findByRole("region", { name: "已结束" }, { timeout: 5000 });
    const title = within(finished).getByRole("button", { name: /剧集/ });
    const row = title.parentElement?.parentElement as HTMLElement;
    expect(within(row).getByText("扫描不完整")).toBeInTheDocument();
    expect(within(row).getByText(GAPS)).toBeInTheDocument();
    // Clean scans keep the ordinary finished state.
    expect(within(finished).getAllByText("已完成").length).toBeGreaterThan(0);

    await user.click(title);
    const warning = await screen.findByRole("region", { name: "这次扫描不完整" });
    expect(within(warning).getByText(GAPS)).toBeInTheDocument();
    expect(within(warning).getByText(/不会被移除/)).toBeInTheDocument();
    expect(within(warning).getByRole("button", { name: "重新扫描" })).toBeInTheDocument();
  }, 20_000);

  it("needs the user on the overview until the library is scanned again", async () => {
    const db = installMockDb();
    signIn(db, MOCK_ACCOUNTS.admin.username);
    renderApp("/");

    const attention = await screen.findByRole("region", { name: "需要你处理" }, { timeout: 5000 });
    expect(await within(attention).findByText("扫描「剧集」")).toBeInTheDocument();
    expect(within(attention).getByText(GAPS, { exact: false })).toBeInTheDocument();
    expect(within(attention).getByRole("button", { name: "重新扫描" })).toBeInTheDocument();
  }, 20_000);
});
