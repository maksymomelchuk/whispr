import { ChevronRight } from "lucide-react";
import { useState, type ReactNode } from "react";

import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";

import { Card } from "./ui/card";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "./ui/collapsible";

interface Props {
  title: string;
  defaultOpen?: boolean;
  dirty?: boolean;
  info?: string;
  children: ReactNode;
}

export function CollapsibleCard({
  title,
  defaultOpen = true,
  dirty = false,
  info,
  children,
}: Props) {
  const [open, setOpen] = useState(defaultOpen);

  return (
    <Collapsible open={open} onOpenChange={setOpen}>
      <Card className="overflow-hidden">
        <div className="relative flex items-center">
          <CollapsibleTrigger
            className={cn(
              "flex flex-1 items-center gap-2.5 px-4 py-[14px]",
              "rounded-none border-0 bg-transparent text-left",
              "cursor-pointer select-none",
              "transition-colors duration-150 motion-reduce:transition-none",
              "hover:bg-[var(--bg-header-hover)]",
              "focus-visible:outline-2 focus-visible:outline-offset-[-2px] focus-visible:outline-[var(--primary)]",
            )}
          >
            <ChevronRight
              className={cn(
                "size-[10px] shrink-0 text-[var(--text-tertiary)]",
                "transition-transform duration-[180ms] ease-[cubic-bezier(0.25,0.46,0.45,0.94)] motion-reduce:transition-none",
                open && "rotate-90 text-[var(--text-secondary)]",
              )}
              aria-hidden
            />
            <h2 className="m-0 flex-1 text-[13px] font-semibold leading-none">
              {title}
            </h2>
            {dirty && !open && (
              <span
                className="ml-auto inline-block size-1.5 rounded-full bg-[var(--warning)] shadow-[0_0_0_3px_var(--warning-halo)]"
                aria-label="Unsaved changes"
              />
            )}
          </CollapsibleTrigger>
          {info && (
            <span className="absolute right-4 top-1/2 z-[2] -translate-y-1/2">
              <Tooltip>
                <TooltipTrigger asChild>
                  <span
                    className="inline-flex items-center justify-center w-3.5 h-3.5 rounded-full border border-[var(--border-strong)] text-[10px] font-semibold leading-none text-[var(--text-tertiary)] bg-[var(--bg-elevated)] cursor-help select-none outline-none"
                    aria-label={info}
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
                <TooltipContent>{info}</TooltipContent>
              </Tooltip>
            </span>
          )}
        </div>
        <CollapsibleContent>
          <div className="px-4 pb-4">{children}</div>
        </CollapsibleContent>
      </Card>
    </Collapsible>
  );
}
