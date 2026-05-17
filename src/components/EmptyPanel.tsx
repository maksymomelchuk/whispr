import type { ReactNode } from "react";

interface Props {
  icon?: ReactNode;
  title: string;
  hint?: string;
}

export function EmptyPanel({ icon, title, hint }: Props) {
  return (
    <div className="flex flex-col items-center justify-center gap-2.5 rounded-lg border border-dashed border-border/80 bg-card/30 px-6 py-14 text-center">
      {icon && <span className="text-muted-foreground/50">{icon}</span>}
      <div className="text-md font-semibold text-muted-foreground">{title}</div>
      {hint && (
        <div className="max-w-[280px] text-xs text-muted-foreground/70">{hint}</div>
      )}
    </div>
  );
}
