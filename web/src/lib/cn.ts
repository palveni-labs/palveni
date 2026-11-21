import { extendTailwindMerge } from "tailwind-merge";
import { defaultConfig as variantsConfig } from "tailwind-variants";

/**
 * The skin's own scale names (src/styles/globals.css). Without them Tailwind's conflict
 * resolution mistakes a font size for a colour (`text-headline` vs `text-heading`) and a
 * weight for a family (`font-title` vs `font-heading`), and drops one of the pair.
 */
const MERGE_CONFIG = {
  extend: {
    theme: {
      text: ["md", "headline", "stat"],
      "font-weight": ["body", "mid", "strong", "title"],
      radius: ["check", "control", "chip", "tile", "panel"],
      shadow: ["panel", "raised", "art"],
    },
  },
};

/** Class names joined with Tailwind conflicts resolved. */
export const cn = extendTailwindMerge(MERGE_CONFIG);

// HeroUI merges a `className` given to a component slot with its own classes through
// tailwind-variants, whose shared default config every slot reads at merge time. Teaching it
// the same names keeps `<Modal.Heading className="font-heading font-title">` intact.
if (variantsConfig.twMergeConfig) Object.assign(variantsConfig.twMergeConfig, MERGE_CONFIG);
else variantsConfig.twMergeConfig = MERGE_CONFIG;
