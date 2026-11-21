import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { expect, it } from "vitest";

import { installMockDb } from "@/test/msw-server";
import { renderApp, signIn } from "@/test/render-app";

it("applies and clears server overrides through the administrator settings", async () => {
  const db = installMockDb();
  signIn(db, "admin");
  renderApp("/system/settings");
  const user = userEvent.setup();
  await user.click(await screen.findByRole("button", { name: "编辑服务器设置" }, { timeout: 5000 }));
  await user.type(screen.getByRole("textbox", { name: "名称" }), "书房");
  await user.selectOptions(screen.getByRole("combobox", { name: "性能档位" }), "lite");
  await user.click(screen.getByRole("button", { name: "保存" }));
  await waitFor(() => expect(db.runtimeSettings).toEqual({ serverName: "书房", perfTier: "lite" }));
  await user.click(await screen.findByRole("button", { name: "编辑服务器设置" }));
  await user.clear(screen.getByRole("textbox", { name: "名称" }));
  await user.selectOptions(screen.getByRole("combobox", { name: "性能档位" }), "");
  await user.click(screen.getByRole("button", { name: "保存" }));
  await waitFor(() => expect(db.runtimeSettings).toEqual({ serverName: null, perfTier: null }));
});

it("shows a member's music password once and confirms rotation and revocation", async () => {
  const db = installMockDb();
  signIn(db, "family");
  renderApp("/system/settings");
  const user = userEvent.setup();
  await user.click(await screen.findByRole("button", { name: "创建口令" }));
  const first = (await screen.findByRole<HTMLInputElement>("textbox", { name: "音乐客户端口令" })).value;
  expect(first).toBeTruthy();
  await user.click(screen.getByRole("button", { name: "关闭" }));
  expect(screen.queryByDisplayValue(first)).toBeNull();
  await user.click(screen.getByRole("button", { name: "更换口令" }));
  await user.click(screen.getByRole("button", { name: "取消" }));
  await user.click(screen.getByRole("button", { name: "更换口令" }));
  await user.click(screen.getByRole("button", { name: "更换口令" }));
  const second = (await screen.findByRole<HTMLInputElement>("textbox", { name: "音乐客户端口令" })).value;
  expect(second).not.toBe(first);
  await user.click(screen.getByRole("button", { name: "关闭" }));
  await user.click(screen.getByRole("button", { name: "撤销访问" }));
  await user.click(screen.getByRole("button", { name: "撤销访问" }));
  expect(await screen.findByRole("button", { name: "创建口令" })).toBeEnabled();
});
