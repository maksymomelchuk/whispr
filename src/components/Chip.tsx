import { X } from "@phosphor-icons/react";

interface Props {
  label: string;
  onRemove: () => void;
}

export function Chip({ label, onRemove }: Props) {
  return (
    <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-md bg-primary/10 text-primary text-[12px] leading-snug ring-1 ring-inset ring-primary/15">
      {label}
      <button
        type="button"
        onClick={(e) => {
          e.stopPropagation();
          onRemove();
        }}
        className="text-primary/60 hover:text-destructive transition-colors"
        aria-label={`Remove ${label}`}
      >
        <X size={10} weight="bold" />
      </button>
    </span>
  );
}
