import type { ReactNode } from "react";

import { RowCard } from "@/components/RowCard";
import { Button } from "@/components/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";

interface RowActionButtonProps {
  icon: ReactNode;
  label: string;
  onClick: () => void;
  tone?: "default" | "destructive";
  disabled?: boolean;
}

export function RowActionButton({
  icon,
  label,
  onClick,
  tone = "default",
  disabled,
}: RowActionButtonProps) {
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <Button
          variant="ghost"
          size="icon-sm"
          aria-label={label}
          onClick={onClick}
          disabled={disabled}
          className={
            tone === "destructive"
              ? "text-muted-foreground/70 hover:text-destructive"
              : "text-muted-foreground hover:text-foreground"
          }
        >
          {icon}
        </Button>
      </TooltipTrigger>
      <TooltipContent>{label}</TooltipContent>
    </Tooltip>
  );
}

interface ListRowProps {
  label: ReactNode;
  meta?: ReactNode;
  actions?: ReactNode;
  flashing?: boolean;
  onClick?: () => void;
  expanded?: boolean;
  below?: ReactNode;
  className?: string;
}

// One row grammar for every list page: label region (truncating), trailing
// meta, and an action cluster that stays dim until hover or keyboard focus
// reaches the row. `below` renders a connected inline editor whose top border
// merges with the row when `expanded`.
export function ListRow({
  label,
  meta,
  actions,
  flashing,
  onClick,
  expanded,
  below,
  className,
}: ListRowProps) {
  const clickable = onClick != null;

  return (
    <div className="flex flex-col group/row">
      <RowCard
        interactive={clickable}
        flashing={flashing}
        onClick={onClick}
        className={cn(
          expanded &&
            "rounded-b-none border-b-0 group-hover/row:border-ring/55",
          className,
        )}
      >
        <div className="flex flex-1 min-w-0 items-center gap-2.5">
          {label}
          {meta}
        </div>
        {actions && (
          <div
            className={
              "flex items-center gap-0.5 shrink-0 transition-opacity " +
              "opacity-65 group-hover/row:opacity-100 group-focus-within/row:opacity-100"
            }
            onClick={(e) => e.stopPropagation()}
          >
            {actions}
          </div>
        )}
      </RowCard>

      {expanded && below && (
        <div className="rounded-b-lg border-x border-b border-border bg-card px-3 py-3 transition-[border-color] duration-150 group-hover/row:border-ring/55">
          {below}
        </div>
      )}
    </div>
  );
}
