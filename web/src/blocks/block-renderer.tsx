import { useState, type ReactNode } from "react";
import { cn } from "@/lib/cn";
import { SummaryFacts } from "@/ui/page";
import { CollectionBlockView, ProgressBlockView, TableBlockView, TextBlockView } from "./renderers/basic";
import { ApprovalView, ChangePreviewView } from "./renderers/changes";
import { ChartBlockView } from "./renderers/chart";
import { DeviceCard } from "./renderers/device";
import { FormBlockView } from "./renderers/form";
import type { Block, BlockAction, ChangePreviewBlock } from "./types";

export interface BlockRendererProps {
  blocks: readonly Block[];
  /** Receives user intents; the host decides which API call (if any) they become. */
  onAction?: (action: BlockAction) => void;
  /** Read-only rendering (history, resolved approvals). */
  readOnly?: boolean;
  /** Disables approval buttons while the host is working on a decision. */
  busy?: boolean;
  /** Keep confirmation requests in view at the bottom of the scrolling container. */
  pinApprovals?: boolean;
  className?: string;
}

type Selection = Record<string, ReadonlySet<string>>;

function defaultSelection(preview: ChangePreviewBlock): ReadonlySet<string> {
  return new Set(preview.rows.filter((row) => row.selected).map((row) => row.id));
}

/** Groups runs of stats and devices so they lay out as grids. */
function group(blocks: readonly Block[]): Array<Block | { type: "group"; kind: "stat" | "device"; blocks: Block[] }> {
  const out: Array<Block | { type: "group"; kind: "stat" | "device"; blocks: Block[] }> = [];
  for (const block of blocks) {
    const last = out[out.length - 1];
    if ((block.type === "stat" || block.type === "device") && last && last.type === "group" && last.kind === block.type) {
      last.blocks.push(block);
    } else if (block.type === "stat" || block.type === "device") {
      out.push({ type: "group", kind: block.type, blocks: [block] });
    } else {
      out.push(block);
    }
  }
  return out;
}

/**
 * Draws declarative blocks (see ./types.ts). Selection in change previews is held here so an
 * approval block can confirm exactly the rows the user left checked.
 */
export function BlockRenderer({ blocks, onAction, readOnly = false, busy = false, pinApprovals = false, className }: BlockRendererProps) {
  // Only rows the user touched are stored; untouched previews use the sender's defaults.
  const [selection, setSelection] = useState<Selection>({});
  const emit = (action: BlockAction) => onAction?.(action);

  const selectedOf = (preview: ChangePreviewBlock) => selection[preview.id] ?? defaultSelection(preview);
  const toggle = (preview: ChangePreviewBlock, rowId: string) =>
    setSelection((current) => {
      const next = new Set(current[preview.id] ?? defaultSelection(preview));
      if (next.has(rowId)) next.delete(rowId);
      else next.add(rowId);
      return { ...current, [preview.id]: next };
    });
  const toggleAll = (preview: ChangePreviewBlock, all: boolean) =>
    setSelection((current) => ({ ...current, [preview.id]: new Set(all ? preview.rows.map((row) => row.id) : []) }));

  const render = (block: Block, key: string | number): ReactNode => {
    switch (block.type) {
      case "text":
        return <TextBlockView key={key} block={block} />;
      case "stat":
        return <SummaryFacts key={key} facts={[{ label: block.label, value: block.value, unit: block.unit, note: block.hint }]} />;
      case "collection":
        return <CollectionBlockView key={key} block={block} onOpen={(itemId) => emit({ type: "open-item", itemId })} />;
      case "table":
        return <TableBlockView key={key} block={block} />;
      case "progress":
        return <ProgressBlockView key={key} block={block} />;
      case "chart":
        return <ChartBlockView key={key} block={block} />;
      case "form":
        return (
          <FormBlockView
            key={key}
            block={block}
            readOnly={readOnly}
            onSubmit={(values) => emit({ type: "submit", blockId: block.id, values })}
          />
        );
      case "device":
        return (
          <DeviceCard
            key={key}
            device={block.device}
            readOnly={readOnly}
            onPatch={(patch) => emit({ type: "device", deviceId: block.device.id, patch })}
          />
        );
      case "change-preview": {
        const pending = blocks.some((other) => other.type === "approval" && other.previewId === block.id && (other.status ?? "pending") === "pending");
        return (
          <ChangePreviewView
            key={key}
            block={block}
            selected={selectedOf(block)}
            readOnly={readOnly || !pending}
            onToggle={(rowId) => toggle(block, rowId)}
            onToggleAll={(all) => toggleAll(block, all)}
          />
        );
      }
      case "approval": {
        const preview = block.previewId
          ? blocks.find((other): other is ChangePreviewBlock => other.type === "change-preview" && other.id === block.previewId)
          : undefined;
        const selected = preview ? [...selectedOf(preview)] : [];
        return (
          <ApprovalView
            key={key}
            block={block}
            selectedCount={preview ? selected.length : null}
            busy={busy || readOnly}
            pending={busy}
            pinned={pinApprovals}
            onConfirm={() => emit({ type: "approve", blockId: block.id, selectedRowIds: selected })}
            onCancel={() => emit({ type: "cancel", blockId: block.id })}
            onEdit={() => {
              document.getElementById(`block-${block.previewId}`)?.scrollIntoView({ behavior: "smooth", block: "center" });
              emit({ type: "edit", blockId: block.id });
            }}
          />
        );
      }
    }
  };

  return (
    <div className={cn("grid gap-4", className)}>
      {group(blocks).map((entry, index) =>
        entry.type === "group" ? (
          entry.kind === "stat" ? (
            <SummaryFacts
              key={index}
              facts={entry.blocks.flatMap((block) =>
                block.type === "stat" ? [{ label: block.label, value: block.value, unit: block.unit, note: block.hint }] : [],
              )}
            />
          ) : (
            <div key={index} className="grid grid-cols-[repeat(auto-fill,minmax(10.5rem,1fr))] gap-3">
              {entry.blocks.map((block, inner) => render(block, `${index}-${inner}`))}
            </div>
          )
        ) : (
          render(entry, index)
        ),
      )}
    </div>
  );
}
