import { Plus } from "@phosphor-icons/react";
import type { ReactNode } from "react";

import { cn } from "@/lib/utils";
import { RowCardButton } from "@/components/RowCard";

interface Props {
  preview: ReactNode;
  hint?: string;
  action: string;
  onClick: () => void;
  className?: string;
}

export function EmptyRowCard({ preview, hint, action, onClick, className }: Props) {
  return (
    <RowCardButton onClick={onClick} className={cn("justify-between px-4 py-6", className)}>
      {hint ? (
        <div className="flex flex-col gap-1 text-left">
          {preview}
          <span className="text-xs text-muted-foreground/70">{hint}</span>
        </div>
      ) : (
        preview
      )}
      <span className="flex items-center gap-1.5 text-xs font-medium text-muted-foreground/80 group-hover:text-foreground transition-colors">
        <Plus size={13} />
        {action}
      </span>
    </RowCardButton>
  );
}
