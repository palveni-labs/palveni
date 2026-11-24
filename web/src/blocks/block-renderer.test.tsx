import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { BlockRenderer } from "./block-renderer";
import type { Block } from "./types";

const preview: Block = {
  type: "change-preview",
  id: "p",
  title: "将移到回收站的文件",
  rows: [
    { id: "a", before: "a (1).jpg", after: "回收站/a (1).jpg", selected: true },
    { id: "b", before: "b (1).jpg", after: "回收站/b (1).jpg", selected: true },
    { id: "c", before: "c (1).jpg", after: "回收站/c (1).jpg", selected: false },
  ],
};
const approval: Block = {
  type: "approval",
  id: "ok",
  title: "把选中的文件移到回收站",
  effects: ["30 天内可以恢复"],
  risk: "medium",
  previewId: "p",
  confirmLabel: "移到回收站",
};

describe("BlockRenderer", () => {
  it("renders every block type from data", () => {
    const blocks: Block[] = [
      { type: "text", text: "整理完成", tone: "success" },
      { type: "stat", label: "重复组", value: 23 },
      { type: "collection", items: [{ id: "i1", title: "Blue Station", subtitle: "Golden Harbors", kind: "music" }], more: 4 },
      { type: "table", columns: [{ key: "name", label: "文件" }, { key: "size", label: "大小", align: "end", format: "bytes" }], rows: [{ name: "big.mkv", size: 2 * 1024 ** 3 }] },
      { type: "progress", label: "转码", value: 0.5 },
      { type: "chart", title: "按媒体库", kind: "donut", unit: "count", series: [{ label: "电影", value: 3 }, { label: "音乐", value: 1 }] },
      { type: "form", id: "f", fields: [{ kind: "text", name: "name", label: "名称", value: "x" }] },
      { type: "device", device: { id: "d", name: "主灯", online: true, state: { type: "light", on: true, brightness: 40 } } },
    ];
    render(<BlockRenderer blocks={blocks} />);
    expect(screen.getByText("整理完成")).toBeInTheDocument();
    expect(screen.getByText("23")).toBeInTheDocument();
    expect(screen.getByText("Blue Station")).toBeInTheDocument();
    expect(screen.getByText("还有 4 项")).toBeInTheDocument();
    expect(screen.getByText("2 GB")).toBeInTheDocument();
    expect(screen.getByText("50%")).toBeInTheDocument();
    expect(screen.getByRole("img", { name: "按媒体库" })).toBeInTheDocument();
    expect(screen.getByText("电影")).toBeInTheDocument();
    expect(screen.getByLabelText("名称")).toHaveValue("x");
    expect(screen.getByText("主灯")).toBeInTheDocument();
  });

  it("confirms exactly the rows the user left checked", async () => {
    const user = userEvent.setup();
    const onAction = vi.fn();
    render(<BlockRenderer blocks={[preview, approval]} onAction={onAction} />);
    const confirm = screen.getByRole("button", { name: /移到回收站/ });
    expect(confirm).toHaveTextContent("2");
    expect(screen.getByText("已选 2 / 3")).toBeInTheDocument();

    const list = screen.getByRole("region", { name: "将移到回收站的文件" });
    const boxes = within(list).getAllByRole("checkbox");
    await user.click(boxes[1] as HTMLElement); // row "a" off (index 0 is select-all)
    await user.click(boxes[3] as HTMLElement); // row "c" on
    expect(screen.getByText("已选 2 / 3")).toBeInTheDocument();
    await user.click(confirm);
    expect(onAction).toHaveBeenCalledWith({ type: "approve", blockId: "ok", selectedRowIds: ["b", "c"] });
  });

  it("disables confirmation when nothing is selected and reports cancel", async () => {
    const user = userEvent.setup();
    const onAction = vi.fn();
    render(<BlockRenderer blocks={[preview, approval]} onAction={onAction} />);
    await user.click(screen.getByRole("checkbox", { name: "全选" }));
    await user.click(screen.getByRole("checkbox", { name: "全选" }));
    expect(screen.getByRole("button", { name: /移到回收站/ })).toBeDisabled();
    await user.click(screen.getByRole("button", { name: "取消" }));
    expect(onAction).toHaveBeenCalledWith({ type: "cancel", blockId: "ok" });
  });

  it("shows decided approvals read-only", () => {
    render(<BlockRenderer blocks={[preview, { ...approval, status: "approved" }]} readOnly />);
    expect(screen.getByText("已确认")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /移到回收站/ })).not.toBeInTheDocument();
    expect(screen.queryAllByRole("checkbox")).toHaveLength(0);
  });

  it("turns device controls into patches for the host", async () => {
    const user = userEvent.setup();
    const onAction = vi.fn();
    render(
      <BlockRenderer
        onAction={onAction}
        blocks={[
          { type: "device", device: { id: "lamp", name: "台灯", online: true, state: { type: "light", on: false, brightness: 30 } } },
          { type: "device", device: { id: "ac", name: "空调", online: true, state: { type: "climate", on: true, mode: "cool", current: 26, target: 24 } } },
          { type: "device", device: { id: "plug", name: "插座", online: false, state: { type: "switch", on: false } } },
        ]}
      />,
    );
    await user.click(screen.getByRole("switch", { name: "开关台灯" }));
    await user.click(screen.getByRole("button", { name: "调高温度" }));
    expect(onAction).toHaveBeenNthCalledWith(1, { type: "device", deviceId: "lamp", patch: { on: true } });
    expect(onAction).toHaveBeenNthCalledWith(2, { type: "device", deviceId: "ac", patch: { target: 25 } });
    expect(screen.getByRole("switch", { name: "开关插座" })).toBeDisabled();
    expect(screen.getByText("离线")).toBeInTheDocument();
  });
});
