import { useState } from "react";
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
  const [open, setOpen] = useState(false);

  return (
    <Tooltip open={open} onOpenChange={setOpen}>
      <TooltipTrigger asChild>
        <span
          className="inline-flex items-center justify-center rounded-full text-muted-foreground/40 transition-colors cursor-help select-none outline-none hover:text-muted-foreground/70 focus-visible:text-muted-foreground/70 focus-visible:ring-[3px] focus-visible:ring-ring/50 focus-visible:ring-offset-1 focus-visible:ring-offset-card"
          aria-label={text}
          tabIndex={0}
          role="button"
          onClick={(e) => {
            e.preventDefault();
            e.stopPropagation();
          }}
          onMouseDown={(e) => e.preventDefault()}
          onKeyDown={(e) => {
            if (e.key === "Enter" || e.key === " ") {
              e.preventDefault();
              e.stopPropagation();
              setOpen((o) => !o);
            }
          }}
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
