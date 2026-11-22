/// <reference types="node" />
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { cleanup, render, within } from "@testing-library/react";
import { readdirSync, readFileSync, statSync } from "node:fs";
import path from "node:path";
import type { ReactNode } from "react";
import { describe, expect, it, vi } from "vitest";
import { queryKeys } from "@/api/query-keys";
import { BUILTIN_SKIN, type SkinEntry } from "@/skins/registry";
import type { Slots } from "@/skins/schema";
import { SkinScope, useSlot } from "@/skins/scope";
import { AttentionMark, Measure } from "./marks";
import { PageHeader, SummaryFacts } from "./page";
import { RunProgress } from "./progress";
import { Section } from "./section";

function matchMediaStub(query: string): MediaQueryList {
  return {
    matches: false,
    media: query,
    onchange: null,
    addEventListener: () => {},
    removeEventListener: () => {},
    addListener: () => {},
    removeListener: () => {},
    dispatchEvent: () => false,
  };
}

function entryWith(slots: Partial<Slots>): SkinEntry {
  const manifest = { ...BUILTIN_SKIN.manifest, id: "test", slots: { ...BUILTIN_SKIN.manifest.slots, ...slots } };
  return { ...BUILTIN_SKIN, manifest, loadFonts: () => Promise.resolve([]) };
}

function renderIn(slots: Partial<Slots>, children: ReactNode) {
  const client = new QueryClient();
  client.setQueryData(queryKeys.system.info, { name: "NAS", version: "0", serverId: "x", setupCompleted: true, features: [] });
  const result = render(
    <QueryClientProvider client={client}>
      <SkinScope entry={entryWith(slots)} mode="light">
        {children}
      </SkinScope>
    </QueryClientProvider>,
  );
  return { ...result, ...within(result.container) };
}

describe("host components draw the variant their slot names", () => {
  it("progress: bar, separator and row-fill, and unknown progress is never 0%", () => {
    const row = <RunProgress value={0.37} label="转码">row</RunProgress>;
    const bar = renderIn({ progress: "bar" }, row).container;
    expect(bar.querySelector(".bg-progress-done, [style*='width: 37%']")).toBeNull();
    const separator = renderIn({ progress: "separator" }, row).container;
    expect(separator.querySelector(".bg-progress-done")).toHaveStyle({ width: "37%" });
    const fill = renderIn({ progress: "row-fill" }, row).container;
    expect(fill.querySelector("[style*='width: 37%']")).not.toBeNull();
    const unknown = renderIn({ progress: "separator" }, <RunProgress value={null} label="扫描">row</RunProgress>).container;
    expect(unknown.querySelector(".pv-dashed")).not.toBeNull();
    expect(unknown.querySelector("[role='progressbar']")).not.toHaveAttribute("aria-valuenow");
  });

  it("attention: seal, lamp, tint and thumbs", () => {
    expect(renderIn({ attention: "seal" }, <AttentionMark title="x" />).container.querySelector(".pv-seal")).not.toBeNull();
    expect(renderIn({ attention: "lamp" }, <AttentionMark title="x" />).container.querySelector(".bg-attention-mark")).not.toBeNull();
    expect(renderIn({ attention: "tint" }, <AttentionMark title="x" />).container.querySelector("svg")).not.toBeNull();
    const thumbs = renderIn({ attention: "thumbs" }, <AttentionMark title="八张专辑" previews={["a", "b"]} />);
    expect(thumbs.getByRole("img", { name: "八张专辑" })).toBeInTheDocument();
  });

  it("section: panel, rule and plain separation", () => {
    for (const section of ["panel", "rule", "plain"] as const) {
      const view = renderIn({ section }, <Section title="正在进行">body</Section>);
      expect(view.getByRole("region", { name: "正在进行" })).toHaveAttribute("data-section", section);
    }
    const raised = renderIn({ section: "rule" }, <Section title="需要你处理" raised>body</Section>);
    expect(raised.getByRole("region", { name: "需要你处理" })).toHaveAttribute("data-section", "panel");
  });

  it("page title: in the content or left to the toolbar", () => {
    const content = renderIn({ pageTitle: "content" }, <PageHeader title="概览" summary="3 件事等你处理" />);
    expect(content.getByRole("heading", { level: 1, name: "概览" })).not.toHaveClass("sr-only");
    expect(content.getByText("3 件事等你处理")).toBeInTheDocument();
    cleanup();

    const wide = vi.spyOn(window, "matchMedia").mockImplementation((query) => ({ ...matchMediaStub(query), matches: query === "(min-width: 768px)" }));
    const toolbar = renderIn({ pageTitle: "toolbar" }, <PageHeader title="概览" summary="3 件事等你处理" />);
    expect(toolbar.getByRole("heading", { level: 1, name: "概览" })).toHaveClass("sr-only");
    // The summary stays with the page: the toolbar only has room for the title.
    expect(toolbar.getByText("3 件事等你处理")).toBeInTheDocument();
    cleanup();
    wide.mockRestore();

    // Phones have no toolbar title, so the page shows its own.
    const phone = renderIn({ pageTitle: "toolbar" }, <PageHeader title="概览" summary="3 件事等你处理" />);
    expect(phone.getByRole("heading", { level: 1, name: "概览" })).not.toHaveClass("sr-only");
  });

  it("summary facts: cells or one line", () => {
    const facts = [{ label: "重复", value: 23, unit: "组" }];
    expect(renderIn({ summary: "cells" }, <SummaryFacts facts={facts} />).container.querySelector("dl")).toHaveClass("grid");
    expect(renderIn({ summary: "inline" }, <SummaryFacts facts={facts} />).container.querySelector("dl")).toHaveClass("flex");
  });

  it("measure: monospace only where the skin says readings are mono", () => {
    expect(renderIn({ measure: "mono" }, <Measure value="4.2×" />).container.querySelector(".font-mono")).not.toBeNull();
    expect(renderIn({ measure: "sans" }, <Measure value="4.2×" />).container.querySelector(".font-mono")).toBeNull();
  });

  it("a scope answers for its own skin, nested inside another", () => {
    function Probe() {
      return <span>{useSlot("nav")}</span>;
    }
    const view = renderIn(
      { nav: "topbar" },
      <SkinScope entry={entryWith({ nav: "sidebar" })} mode="dark">
        <Probe />
      </SkinScope>,
    );
    expect(view.getByText("sidebar")).toBeInTheDocument();
  });
});

describe("no skin identity in the app", () => {
  const SRC = path.resolve(import.meta.dirname, "..");
  const files = (dir: string): string[] =>
    readdirSync(dir).flatMap((name) => {
      const full = path.join(dir, name);
      return statSync(full).isDirectory() ? files(full) : [full];
    });

  it("never branches on a skin's id: structure comes from slots", () => {
    // `hls: "native" | "mse" | "none"` shares the word "native" with a skin id.
    const stripHlsMode = (source: string) => source.replaceAll(/\b(?:hls|HlsSupport)\s*(?::|===|!==|=)\s*["']native["']/g, "");
    const offenders = files(SRC)
      .filter((file) => /\.(ts|tsx)$/.test(file) && !/\.test\.tsx?$/.test(file))
      .filter((file) => !file.startsWith(path.join(SRC, "skins")) && !file.startsWith(path.join(SRC, "mocks")))
      .filter((file) => /["'`](workcolor|native|ink|gear)["'`]/.test(stripHlsMode(readFileSync(file, "utf8"))))
      .map((file) => path.relative(SRC, file));
    expect(offenders).toEqual([]);
  });
});
