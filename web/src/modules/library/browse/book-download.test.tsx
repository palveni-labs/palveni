import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { http, HttpResponse } from "msw";
import { expect, it, vi } from "vitest";
import { M0_FEATURES, MOCK_ACCOUNTS } from "@/mocks/db";
import { installMockDb, server } from "@/test/msw-server";
import { signIn } from "@/test/render-app";
import { ItemDetailsSheet } from "./item-details-sheet";

it("downloads the selected book from its details using the short-lived URL", async () => {
  const db = installMockDb({ features: [...M0_FEATURES, "bookDownloads"] });
  signIn(db, MOCK_ACCOUNTS.user.username);
  const library = db.libraries.find((entry) => entry.kind === "book")!;
  const folder = db.listChildren(library.id, null)[0]!;
  const book = db.listChildren(library.id, folder.id)[0]!;
  const path = `/api/v1/items/${book.id}/book-download`;
  const signed = path + "?ticket=test-ticket";
  server.use(http.post(path, ({ request }) => {
    expect(request.headers.get("Authorization")).toMatch(/^Bearer /);
    return HttpResponse.json({ url: signed, expiresAt: db.iso(900_000) });
  }));
  let requestUrl: string | undefined;
  const click = vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(function(this: HTMLAnchorElement) {
    expect(this).toHaveAttribute("download");
    requestUrl = this.href;
  });
  const client = new QueryClient();
  try {
    render(<QueryClientProvider client={client}>
      <ItemDetailsSheet item={{ ...book, path: null }} library={library} onClose={() => undefined} />
    </QueryClientProvider>);
    await userEvent.click(await screen.findByRole("button", { name: "下载原件" }));
    await waitFor(() => expect(requestUrl).toBe(window.location.origin + signed));
    expect(document.querySelector("a[download]")).toBeNull();
  } finally {
    click.mockRestore();
    client.clear();
  }
});
