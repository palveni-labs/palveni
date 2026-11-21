import "@testing-library/jest-dom/vitest";
import { cleanup } from "@testing-library/react";
import { afterAll, afterEach, beforeAll, beforeEach } from "vitest";
import { setUnauthorizedHandler } from "@/api/client";
import { resetSessionCache } from "@/api/session";
import i18n from "@/i18n";
import { resetSkinStore } from "@/skins/store";
import { disposeMockDb, server } from "./msw-server";

// jsdom lacks a few layout APIs the UI touches.
class ResizeObserverStub {
  observe() {}
  unobserve() {}
  disconnect() {}
}
const noop = () => {};
Object.assign(globalThis, { ResizeObserver: globalThis.ResizeObserver ?? ResizeObserverStub });
Object.assign(window, {
  // jsdom defines scrollTo but only logs "not implemented"; the router calls it on navigation.
  scrollTo: noop,
  matchMedia:
    typeof window.matchMedia === "function"
      ? window.matchMedia.bind(window)
      : (query: string) => ({
          matches: false,
          media: query,
          onchange: null,
          addEventListener: noop,
          removeEventListener: noop,
          addListener: noop,
          removeListener: noop,
          dispatchEvent: () => false,
        }),
});
Object.assign(Element.prototype, {
  scrollTo: noop,
  scrollIntoView: noop,
  hasPointerCapture: () => false,
});

beforeAll(() => {
  server.listen({ onUnhandledFrame: "error" });
});

beforeEach(async () => {
  window.localStorage.clear();
  resetSessionCache();
  resetSkinStore();
  await i18n.changeLanguage("zh-CN");
});

afterEach(() => {
  cleanup();
  setUnauthorizedHandler(null);
  disposeMockDb();
  server.resetHandlers();
});

afterAll(() => {
  server.close();
});
