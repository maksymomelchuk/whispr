import type { ReactNode } from "react";

import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";

interface Props {
  title: ReactNode;
  index?: number | string | null;
  isDefault?: boolean;
  badge?: ReactNode;
  trailing?: ReactNode;
  className?: string;
}

export function SectionHeader({
  title,
  index,
  isDefault,
  badge,
  trailing,
  className,
}: Props) {
  const indexLabel =
    typeof index === "number" ? String(index + 1).padStart(2, "0") : index;

  return (
    <div
      className={cn(
        "flex items-baseline gap-3 pb-1.5 border-b border-border/40",
        className,
      )}
    >
      {indexLabel != null && (
        <span className="font-mono text-[10.5px] font-semibold uppercase tracking-[0.18em] text-muted-foreground/55 tabular-nums">
          {indexLabel}
        </span>
      )}
      <h3 className="text-[14px] font-semibold text-foreground tracking-[-0.005em]">
        {title}
      </h3>
      {isDefault && (
        <Badge className="text-[9.5px] font-semibold uppercase tracking-[0.08em] px-1.5 py-0">
          Default
        </Badge>
      )}
      {badge}
      {trailing && (
        <span className="ml-auto text-[11px] text-muted-foreground/70 tabular-nums">
          {trailing}
        </span>
      )}
    </div>
  );
}
