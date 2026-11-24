import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { expect, it } from "vitest";
import { M0_FEATURES } from "@/mocks/db";
import { installMockDb } from "@/test/msw-server";
import { renderApp, selectUploadedSkin, signIn } from "@/test/render-app";

it("renders Jellyfin URL, creates a one-time API key, lists active keys and revokes it", async () => {
  await selectUploadedSkin();
  const db = installMockDb({ features: [...M0_FEATURES, "jellyfin"] });
  signIn(db, "family");
  const user = userEvent.setup();

  renderApp("/system/settings");

  // Jellyfin server URL displayed
  const urlInput = await screen.findByRole("textbox", { name: "Jellyfin 兼容服务地址" }, { timeout: 5000 });
  expect(urlInput).toHaveValue(window.location.origin);
  expect(await screen.findByText("暂未创建客户端 API 密钥")).toBeInTheDocument();

  // Create API key
  await user.type(screen.getByRole("textbox", { name: "密钥名称" }), "客厅 Infuse");
  await user.selectOptions(screen.getByRole("combobox", { name: "用途类型" }), "jellyfin");
  await user.click(screen.getByRole("button", { name: "生成密钥" }));

  // One-time token is shown inside secure surface
  const tokenInput = await screen.findByRole<HTMLInputElement>("textbox", { name: "新建 API 密钥" });
  expect(tokenInput.value).toMatch(/^pvk_/);
  expect(tokenInput.closest("[data-secure-surface=host]")).not.toBeNull();

  // Dismiss hides secret while active key remains in list
  await user.click(screen.getByRole("button", { name: "我已保存" }));
  expect(screen.queryByRole("textbox", { name: "新建 API 密钥" })).toBeNull();
  expect(await screen.findByText("客厅 Infuse")).toBeInTheDocument();

  // Revoke removes key from list
  await user.click(screen.getByRole("button", { name: "撤销 客厅 Infuse" }));
  await waitFor(() => expect(screen.queryByText("客厅 Infuse")).toBeNull());
  expect(screen.getByText("暂未创建客户端 API 密钥")).toBeInTheDocument();
});
