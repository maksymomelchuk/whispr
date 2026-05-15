import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";

interface Props {
  text: string;
}

export function InfoTip({ text }: Props) {
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <span
          className="inline-flex items-center justify-center w-3.5 h-3.5 rounded-full border border-[var(--border-strong)] text-[10px] font-semibold leading-none text-[var(--text-tertiary)] bg-[var(--bg-elevated)] cursor-help select-none outline-none"
          aria-label={text}
          tabIndex={0}
          onClick={(e) => {
            e.preventDefault();
            e.stopPropagation();
          }}
          onMouseDown={(e) => e.preventDefault()}
        >
          ?
        </span>
      </TooltipTrigger>
      <TooltipContent>{text}</TooltipContent>
    </Tooltip>
  );
}
