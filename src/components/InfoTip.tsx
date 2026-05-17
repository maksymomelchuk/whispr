import { Question } from "@phosphor-icons/react";

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
          className="inline-flex items-center justify-center text-muted-foreground/40 hover:text-muted-foreground/70 transition-colors cursor-help select-none outline-none focus-visible:text-muted-foreground/70"
          aria-label={text}
          tabIndex={0}
          onClick={(e) => {
            e.preventDefault();
            e.stopPropagation();
          }}
          onMouseDown={(e) => e.preventDefault()}
        >
          <Question size={13} weight="bold" />
        </span>
      </TooltipTrigger>
      <TooltipContent className="max-w-[220px] text-center leading-snug">
        {text}
      </TooltipContent>
    </Tooltip>
  );
}
