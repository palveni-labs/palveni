import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { MOCK_ACCOUNTS } from "@/mocks/db";
import { installMockDb } from "@/test/msw-server";
import { renderApp, signIn } from "@/test/render-app";

describe("hardware page", () => {
  it("renders the report and separates listed from verified encoders", async () => {
    const db = installMockDb();
    signIn(db, MOCK_ACCOUNTS.admin.username);
    renderApp("/system/hardware");

    expect(await screen.findByText("Intel(R) N100")).toBeInTheDocument();
    expect(screen.getByText("标准")).toBeInTheDocument();
    expect(screen.getByText("16 GB")).toBeInTheDocument();
    expect(screen.getByText("/usr/lib/jellyfin-ffmpeg/ffmpeg")).toBeInTheDocument();

    const status = (id: string) => screen.getAllByText(id)[0]?.closest("[data-status]")?.getAttribute("data-status");
    expect(status("h264_vaapi")).toBe("verified");
    expect(status("av1_vaapi")).toBe("listed");
    expect(status("h264_rkmpp")).toBe("unlisted");

    const failures = screen.getByRole("heading", { name: /已列出但未通过验证/ }).closest("section");
    expect(failures).not.toBeNull();
    const list = within(failures as HTMLElement);
    expect(list.getAllByText("Cannot load libcuda.so.1")).toHaveLength(2);
    expect(list.queryByText("h264_vaapi")).not.toBeInTheDocument();
  });

  it("re-probes on demand", async () => {
    const db = installMockDb();
    signIn(db, MOCK_ACCOUNTS.admin.username);
    const before = db.hardware.probedAt;
    const user = userEvent.setup();
    renderApp("/system/hardware");
    await user.click(await screen.findByRole("button", { name: "重新探测" }));
    await waitFor(() => expect(db.hardware.probedAt).not.toBe(before));
    expect(await screen.findByText("已重新探测硬件")).toBeInTheDocument();
  });
});
