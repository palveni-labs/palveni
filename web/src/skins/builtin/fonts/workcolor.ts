import figtreeExt from "@fontsource-variable/figtree/files/figtree-latin-ext-wght-normal.woff2?url";
import figtree from "@fontsource-variable/figtree/files/figtree-latin-wght-normal.woff2?url";
import fragmentMono from "@fontsource/fragment-mono/files/fragment-mono-latin-400-normal.woff2?url";
import type { FontAsset } from "../../schema";
import { LATIN, LATIN_EXT } from "../stacks";

/** 作品色: Figtree (variable) for the interface, Fragment Mono for file paths. */
const fonts: FontAsset[] = [
  { family: "Figtree", src: figtree, weight: "300 900", style: "normal", unicodeRange: LATIN },
  { family: "Figtree", src: figtreeExt, weight: "300 900", style: "normal", unicodeRange: LATIN_EXT },
  { family: "Fragment Mono", src: fragmentMono, weight: "400", style: "normal", unicodeRange: LATIN },
];

export default fonts;
