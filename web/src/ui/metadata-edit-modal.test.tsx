import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { ALL_FEATURES, installMockDb } from "@/test/msw-server";
import { signIn } from "@/test/render-app";
import { MetadataEditModal } from "./metadata-edit-modal";

function createTestClient() {
  return new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });
}

describe("MetadataEditModal", () => {
  it("reports unconfigured TMDB error, scrapes Bangumi candidate, applies to form, saves and resets override", async () => {
    const db = installMockDb({ features: ALL_FEATURES });
    signIn(db, "admin");
    const sampleItem = db.allLeafItems()[0]!;
    const user = userEvent.setup();
    const onSaved = vi.fn();
    const onReset = vi.fn();
    const onClose = vi.fn();
    const queryClient = createTestClient();

    const { rerender } = render(
      <QueryClientProvider client={queryClient}>
        <MetadataEditModal
          isOpen
          onClose={onClose}
          itemId={sampleItem.id}
          defaultMediaType="movie"
          initialValues={{
            title: "沙丘",
            year: 2021,
            genres: ["科幻"],
            overview: "原始简介",
          }}
          onSaved={onSaved}
          onReset={onReset}
        />
      </QueryClientProvider>,
    );

    const dialog = await screen.findByRole("dialog", { name: "编辑与在线刮削元数据" });

    // 1. Searching TMDB without configured API key shows clear error
    await user.click(within(dialog).getByRole("button", { name: "搜索" }));
    const alert = await within(dialog).findByRole("alert");
    expect(alert.textContent).toMatch(/TMDB/i);

    // 2. Switch provider to Bangumi and search
    await user.selectOptions(within(dialog).getByRole("combobox", { name: "数据源" }), "bangumi");
    await user.click(within(dialog).getByRole("button", { name: "搜索" }));

    expect(await within(dialog).findByText("沙丘 · 番组计划条目")).toBeInTheDocument();

    // 3. Apply candidate to form -> switches to manual tab with populated fields
    await user.click(within(dialog).getByRole("button", { name: "填入表单" }));
    const titleInput = await within(dialog).findByRole("textbox", { name: "标题" });
    expect(titleInput).toHaveValue("沙丘 · 番组计划条目");

    // 4. Save override
    await user.click(within(dialog).getByRole("button", { name: "保存覆盖" }));
    await waitFor(() => expect(onSaved).toHaveBeenCalledTimes(1));
    expect(onSaved.mock.calls[0]?.[0]).toMatchObject({
      itemId: sampleItem.id,
      provider: "bangumi",
      title: "沙丘 · 番组计划条目",
    });

    // 5. Reopen modal -> shows active override badge & reset button -> delete override
    rerender(
      <QueryClientProvider client={queryClient}>
        <MetadataEditModal
          isOpen={false}
          onClose={onClose}
          itemId={sampleItem.id}
          defaultMediaType="movie"
          initialValues={{ title: "沙丘" }}
          onSaved={onSaved}
          onReset={onReset}
        />
      </QueryClientProvider>,
    );
    rerender(
      <QueryClientProvider client={queryClient}>
        <MetadataEditModal
          isOpen
          onClose={onClose}
          itemId={sampleItem.id}
          defaultMediaType="movie"
          initialValues={{ title: "沙丘" }}
          onSaved={onSaved}
          onReset={onReset}
        />
      </QueryClientProvider>,
    );

    const resetBtn = await screen.findByRole("button", { name: "恢复默认" });
    await user.click(resetBtn);
    await waitFor(() => expect(onReset).toHaveBeenCalledTimes(1));
  });
});
