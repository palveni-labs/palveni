const GRID_GAP = 16;
/** Title + subtitle under each tile (text-sm and text-xs lines plus the gap above them). */
const CAPTION_HEIGHT = 48;

/** Columns and tile size for the available width; posters and squares get different minimums. */
export function gridLayout(width: number, aspect: number) {
  const narrow = width < 640;
  const minTile = aspect < 1 ? (narrow ? 96 : 128) : narrow ? 100 : 140;
  const gap = narrow ? 12 : GRID_GAP;
  const columns = Math.max(2, Math.floor((width + gap) / (minTile + gap)));
  const tileWidth = width > 0 ? (width - gap * (columns - 1)) / columns : minTile;
  const rowHeight = Math.ceil(tileWidth / aspect + CAPTION_HEIGHT + gap);
  return { columns, gap, tileWidth, rowHeight };
}
