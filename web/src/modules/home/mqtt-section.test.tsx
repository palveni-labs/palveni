import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { http, HttpResponse } from "msw";
import { describe, expect, it } from "vitest";
import { MOCK_ACCOUNTS } from "@/mocks/db";
import type { HomeDevice } from "@/provisional/types";
import { installMockDb, server } from "@/test/msw-server";
import { signIn } from "@/test/render-app";
import { MqttSection } from "./mqtt-section";

const DEVICES: HomeDevice[] = [
  {
    id: "living-strip",
    roomId: "living",
    room: "客厅",
    name: "氛围灯带",
    online: true,
    state: { type: "light", on: true, brightness: 40, colorTemp: 3000 },
  },
  {
    id: "study-plug",
    roomId: "study",
    room: "书房",
    name: "音箱插座",
    online: true,
    state: { type: "switch", on: true },
  },
];

function renderMqtt(open = true) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>
      <MqttSection open={open} devices={DEVICES} />
    </QueryClientProvider>,
  );
}

describe("MqttSection", () => {
  it("lists lights only in ambient light dropdown, creates, tests, and deletes an MQTT bridge", async () => {
    const db = installMockDb();
    signIn(db, MOCK_ACCOUNTS.admin.username);

    let testCalledForId: string | null = null;
    server.use(
      http.post("/api/v1/home/mqtt/:id/test", ({ params }) => {
        testCalledForId = String(params.id);
        return HttpResponse.json({
          ok: true,
          topic: "palveni/living/status",
          error: null,
        });
      }),
    );

    const user = userEvent.setup();
    renderMqtt(true);

    expect(await screen.findByText(/尚未配置 MQTT 代理/)).toBeInTheDocument();

    // Only light devices appear in ambient light dropdown
    const ambientSelect = screen.getByLabelText("绑定氛围灯");
    expect(ambientSelect).toHaveTextContent("客厅 · 氛围灯带");
    expect(ambientSelect).not.toHaveTextContent("音箱插座");

    // Fill out and submit new MQTT bridge
    const hostInput = screen.getByPlaceholderText("192.168.1.50");
    await user.type(hostInput, "192.168.1.60");

    const topicInput = screen.getByPlaceholderText("palveni");
    await user.clear(topicInput);
    await user.type(topicInput, "palveni/living");

    await user.selectOptions(ambientSelect, "living-strip");
    await user.click(screen.getByRole("button", { name: "保存 MQTT 桥接" }));

    expect(await screen.findByText("192.168.1.60:1883 · palveni/living")).toBeInTheDocument();
    expect(screen.getByText("氛围灯：氛围灯带")).toBeInTheDocument();

    // Run Test Publish
    await user.click(screen.getByRole("button", { name: "测试推送" }));
    await waitFor(() => {
      expect(testCalledForId).not.toBeNull();
    });

    // Delete MQTT bridge
    await user.click(screen.getByRole("button", { name: "删除 MQTT 桥接" }));
    await waitFor(() => {
      expect(screen.queryByText("192.168.1.60:1883 · palveni/living")).not.toBeInTheDocument();
    });
    expect(await screen.findByText(/尚未配置 MQTT 代理/)).toBeInTheDocument();
  });
});
