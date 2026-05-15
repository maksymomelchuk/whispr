import { formatShortcut } from "../lib/api";
import type { Shortcut } from "../lib/types";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";

interface Props {
  shortcut: Shortcut;
  onStartRecord: () => void;
}

export function ShortcutField({ shortcut, onStartRecord }: Props) {
  return (
    <section className="card">
      <div className="card-title-row">
        <h2 style={{ margin: 0 }}>Dictation Shortcut</h2>
        <Tooltip>
          <TooltipTrigger asChild>
            <span
              className="inline-flex items-center justify-center w-3.5 h-3.5 rounded-full border border-[var(--border-strong)] text-[10px] font-semibold leading-none text-[var(--text-tertiary)] bg-[var(--bg-elevated)] cursor-help select-none outline-none"
              aria-label="Hold this key to record. Release to transcribe and paste."
              tabIndex={0}
              onClick={(e) => { e.preventDefault(); e.stopPropagation(); }}
              onMouseDown={(e) => e.preventDefault()}
            >
              ?
            </span>
          </TooltipTrigger>
          <TooltipContent>Hold this key to record. Release to transcribe and paste.</TooltipContent>
        </Tooltip>
      </div>
      <div className="row">
        <div className="shortcut-display">{formatShortcut(shortcut)}</div>
        <button onClick={onStartRecord}>Record new</button>
      </div>
    </section>
  );
}
