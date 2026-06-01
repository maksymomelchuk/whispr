import { PlugIcon } from "@phosphor-icons/react";

import { cn } from "@/lib/utils";

const CUSTOM_TILE_BACKGROUND =
  "linear-gradient(135deg, #64748b 0%, #475569 100%)";

export function CustomLogo({ className }: { className?: string }) {
  return (
    <span
      aria-hidden="true"
      className={cn(
        "flex items-center justify-center overflow-hidden border border-black/10",
        className,
      )}
      style={{ background: CUSTOM_TILE_BACKGROUND }}
    >
      <PlugIcon size="60%" weight="fill" color="#ffffff" />
    </span>
  );
}
