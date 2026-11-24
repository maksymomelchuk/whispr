import { useVirtualizer } from "@tanstack/react-virtual";
import { useCallback, useEffect, useState } from "react";

import { cn } from "@/lib/utils";

import { formatSegmentTime } from "./meetingFormat";
import type { TranscriptLine } from "./transcriptLines";

const ESTIMATED_LINE_HEIGHT_PX = 44;
const OVERSCAN_LINES = 12;
// When the speakers and search controls above leave too little room, the
// detail pane scrolls instead of squeezing the list to a sliver.
const MIN_LIST_HEIGHT_CLASS = "min-h-64";

interface TranscriptLineListProps {
  lines: TranscriptLine[];
  /** Full-transcript index to scroll into view and highlight. */
  focusedIndex: number | null;
  onTimestampClick: (line: TranscriptLine) => void;
}

export function TranscriptLineList({
  lines,
  focusedIndex,
  onTimestampClick,
}: TranscriptLineListProps) {
  const [scrollElement, setScrollElement] = useState<HTMLDivElement | null>(
    null,
  );
  // Measured heights are cached per key; positional keys would hand a row's
  // height to whichever line lands in its slot after filtering.
  const lineKey = useCallback(
    (position: number) => lines[position].index,
    [lines],
  );
  const virtualizer = useVirtualizer({
    count: lines.length,
    getItemKey: lineKey,
    getScrollElement: () => scrollElement,
    estimateSize: () => ESTIMATED_LINE_HEIGHT_PX,
    overscan: OVERSCAN_LINES,
  });

  useEffect(() => {
    if (focusedIndex === null) return;
    const position = lines.findIndex((line) => line.index === focusedIndex);
    if (position >= 0) virtualizer.scrollToIndex(position, { align: "center" });
  }, [focusedIndex, lines, virtualizer]);

  if (lines.length === 0) {
    return (
      <p className="py-6 text-center text-sm text-muted-foreground">
        No lines match.
      </p>
    );
  }

  return (
    <div
      ref={setScrollElement}
      className={cn(
        "flex-1 overflow-y-auto rounded-md border",
        MIN_LIST_HEIGHT_CLASS,
      )}
    >
      <ol
        className="relative w-full text-sm"
        style={{ height: virtualizer.getTotalSize() }}
      >
        {virtualizer.getVirtualItems().map((item) => {
          const line = lines[item.index];
          return (
            <li
              key={line.index}
              data-index={item.index}
              ref={virtualizer.measureElement}
              className={cn(
                "absolute left-0 top-0 flex w-full gap-3 px-3 py-1.5",
                line.segment.echo && "text-muted-foreground",
                line.index === focusedIndex && "bg-primary/10",
              )}
              style={{ transform: `translateY(${item.start}px)` }}
            >
              <button
                type="button"
                onClick={() => onTimestampClick(line)}
                title="Show in full transcript"
                className="w-16 shrink-0 text-left font-mono text-xs tabular-nums text-muted-foreground hover:text-foreground hover:underline"
              >
                {formatSegmentTime(line.segment.start_ms)}
              </button>
              <span className="min-w-0">
                <span className="font-semibold">{line.speakerName}: </span>
                {line.segment.text}
                {line.segment.echo && (
                  <span className="ml-2 rounded bg-muted px-1 text-[10px] uppercase tracking-wide">
                    echo
                  </span>
                )}
              </span>
            </li>
          );
        })}
      </ol>
    </div>
  );
}
