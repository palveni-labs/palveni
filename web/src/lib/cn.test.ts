import { tv } from "tailwind-variants";
import { describe, expect, it } from "vitest";
import { cn } from "./cn";

describe("cn", () => {
  it("keeps a skin font size next to a text colour", () => {
    expect(cn("text-headline", "text-heading")).toBe("text-headline text-heading");
  });

  it("keeps a heading family next to a skin weight", () => {
    expect(cn("font-heading", "font-title")).toBe("font-heading font-title");
  });

  it("still resolves real conflicts", () => {
    expect(cn("font-mid", "font-title")).toBe("font-title");
    expect(cn("rounded-tile", "rounded-panel")).toBe("rounded-panel");
  });
});

describe("component slot merging", () => {
  // HeroUI composes a slot's own classes with ours through tailwind-variants.
  const slots = tv({ slots: { heading: "modal__heading font-medium" } });

  it("keeps family, weight, size and colour given to a slot", () => {
    expect(slots().heading({ className: "font-heading text-headline font-title text-heading" })).toBe("modal__heading font-heading text-headline font-title text-heading");
  });

  it("lets a skin weight replace the slot's weight", () => {
    expect(slots().heading({ className: "font-strong" })).toBe("modal__heading font-strong");
  });
});
