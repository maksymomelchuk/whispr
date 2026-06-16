import { XIcon } from "@phosphor-icons/react";
import type { ReactNode } from "react";

interface Props {
  label: ReactNode;
  onRemove: () => void;
  removeLabel?: string;
}

export function Chip({ label, onRemove, removeLabel }: Props) {
  return (
    <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-md bg-primary/10 text-primary text-[12px] leading-snug ring-1 ring-inset ring-primary/15">
      {label}
      <button
        type="button"
        onClick={(e) => {
          e.stopPropagation();
          onRemove();
        }}
        className="inline-flex items-center justify-center p-1.5 rounded-sm text-primary/60 hover:text-destructive transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
        aria-label={removeLabel ?? "Remove"}
      >
        <XIcon size={10} weight="bold" />
      </button>
    </span>
  );
}
