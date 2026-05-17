import type { ReactNode } from "react";

import { cn } from "@/lib/utils";

interface Props {
  title: ReactNode;
  index?: number | string | null;
  badge?: ReactNode;
  trailing?: ReactNode;
  control?: ReactNode;
  className?: string;
}

export function SectionHeader({
  title,
  index,
  badge,
  trailing,
  control,
  className,
}: Props) {
  const indexLabel =
    typeof index === "number" ? String(index + 1).padStart(2, "0") : index;

  return (
    <div className={cn("flex items-start gap-3", className)}>
      {indexLabel != null && (
        <span className="font-mono text-eyebrow uppercase text-muted-foreground/55 tabular-nums">
          {indexLabel}
        </span>
      )}
      <div className="flex gap-2 items-center">
        <h3 className="font-mono text-eyebrow uppercase text-foreground">
          {title}
        </h3>
        {badge}
      </div>
      {trailing && (
        <span className="ml-auto text-help text-muted-foreground/70 tabular-nums">
          {trailing}
        </span>
      )}
      {control && (
        <div className="ml-auto inline-flex items-center gap-2">{control}</div>
      )}
    </div>
  );
}
