import { useVirtualizer } from "@tanstack/react-virtual";
import { useEffect, useRef } from "react";

import { cn } from "@/lib/utils";

import { formatSegmentTime } from "./meetingFormat";
import type { TranscriptLine } from "./transcriptLines";

const ESTIMATED_LINE_HEIGHT_PX = 44;
const OVERSCAN_LINES = 12;

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
  const scrollRef = useRef<HTMLDivElement>(null);
  const virtualizer = useVirtualizer({
    count: lines.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => ESTIMATED_LINE_HEIGHT_PX,
    overscan: OVERSCAN_LINES,
  });

  useEffect(() => {
    if (focusedIndex === null) return;
    const position = lines.findIndex((line) => line.index === focusedIndex);
    if (position >= 0) virtualizer.scrollToIndex(position, { align: "center" });
  }, [focusedIndex, lines, virtualizer]);

  return (
    <div
      ref={scrollRef}
      data-testid="transcript-scroll"
      className="max-h-[calc(100svh-20rem)] min-h-64 overflow-y-auto rounded-md border"
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
                className="w-12 shrink-0 text-left font-mono text-xs tabular-nums text-muted-foreground hover:text-foreground hover:underline"
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
