import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { http, HttpResponse } from "msw";
import { expect, it, vi } from "vitest";
import { M0_FEATURES, MOCK_ACCOUNTS } from "@/mocks/db";
import { installMockDb, server } from "@/test/msw-server";
import { renderLoadedApp, signIn } from "@/test/render-app";

it("defers subtitle content until selection and keeps the film available when the selected file disappeared", async () => {
  const height = vi.spyOn(Element.prototype, "clientHeight", "get").mockReturnValue(640);
  const width = vi.spyOn(Element.prototype, "clientWidth", "get").mockReturnValue(960);
  try {
    const db = installMockDb({ features: [...M0_FEATURES, "video", "videoSubtitles"] });
    const library = db.createLibrary("Short films", "video", ["/media/films"]);
    signIn(db, MOCK_ACCOUNTS.user.username);
    const id = "e9d06427-9c59-4004-a9d5-2df18039cad4";
    const seen: string[] = [];
    server.use(
      http.get(`/api/v1/libraries/${library.id}/videos`, () => HttpResponse.json({
        entries: [{ id, kind: "movie", title: "Beacon", fileCount: 1, year: 2025, itemId: id, posterItemId: null, overview: null, seasonNumber: null, episodeNumber: null }], nextCursor: null,
      })),
      http.post(`/api/v1/items/${id}/playback`, () => HttpResponse.json({ mode: "direct", url: "/media/beacon.webm", expiresAt: new Date(Date.now()+900_000).toISOString() })),
      http.get(`/api/v1/me/items/${id}`, () => HttpResponse.json({ favorite: false, stars: 0, positionMs: 0, completed: false, playbackVersion: 0, lastPlayedAt: null, playCount: 0 })),
      http.get(`/api/v1/items/${id}/subtitles`, () => HttpResponse.json({ tracks: [
        { id: "zh.forced.srt", language: "zh", forced: true, hearingImpaired: false, format: "srt" },
        { id: "embedded-2", language: "en", forced: false, hearingImpaired: false, format: "vtt" },
      ], truncated: false })),
      http.get(`/api/v1/items/${id}/subtitles/zh.forced.srt`, ({ request }) => {
        seen.push(request.headers.get("Authorization")?.split(" ")[0] ?? "");
        return HttpResponse.json({ error: { code: "not_found", message: "Subtitle unavailable" } }, { status: 404 });
      }),
    );
    const user = userEvent.setup();
    const { container } = await renderLoadedApp(`/library/${library.id}`);
    await user.click(await screen.findByRole("button", { name: /Beacon/ }));
    await user.click(await screen.findByRole("button", { name: "预览原片" }));
    const select = await screen.findByRole("combobox", { name: "字幕" });
    expect(select).toHaveValue("");
    expect(seen).toEqual([]);
    expect(screen.getByRole("option", { name: /强制字幕/ })).toBeInTheDocument();
    expect(screen.getByRole("option", { name: /VTT/ })).toHaveValue("embedded-2");
    await user.selectOptions(select, "zh.forced.srt");
    await screen.findByRole("alert");
    expect(seen).toEqual(["Bearer"]);
    expect(container.querySelector("video")).toHaveAttribute("src", "/media/beacon.webm");
    await user.selectOptions(select, "");
    await waitFor(() => expect(screen.queryByRole("alert")).not.toBeInTheDocument());
  } finally { height.mockRestore(); width.mockRestore(); }
});
