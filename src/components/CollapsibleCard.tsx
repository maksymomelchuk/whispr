import { CaretRightIcon } from "@phosphor-icons/react";
import { useState, type ReactNode } from "react";

import { cn } from "@/lib/utils";

import { InfoTip } from "./InfoTip";
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
    <Collapsible
      open={open}
      onOpenChange={setOpen}
      className="flex flex-col gap-2.5"
    >
      <CollapsibleTrigger
        className={cn(
          "flex w-full items-baseline gap-3 pb-1.5 border-b border-border/40",
          "text-left bg-transparent border-x-0 border-t-0 cursor-pointer select-none",
          "transition-colors duration-150 motion-reduce:transition-none",
          "hover:text-foreground",
          "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50 focus-visible:rounded-sm",
        )}
      >
        <CaretRightIcon
          className={cn(
            "size-[10px] shrink-0 self-center text-muted-foreground/55",
            "transition-transform duration-base ease-[cubic-bezier(0.25,0.46,0.45,0.94)] motion-reduce:transition-none",
            open && "rotate-90 text-muted-foreground",
          )}
          aria-hidden
        />
        <h3 className="text-[14px] font-semibold text-foreground tracking-[-0.005em]">
          {title}
        </h3>
        {info && <InfoTip text={info} />}
        {dirty && !open && (
          <span
            className="ml-auto inline-block size-1.5 rounded-full bg-primary ring-[3px] ring-primary/15"
            aria-label="Unsaved changes"
          />
        )}
      </CollapsibleTrigger>
      <CollapsibleContent>{children}</CollapsibleContent>
    </Collapsible>
  );
}
