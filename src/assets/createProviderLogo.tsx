import type { IconType } from "@lobehub/icons";
import type { ComponentType } from "react";

import { cn } from "@/lib/utils";

export const LIGHT_TILE_BACKGROUND = "#ffffff";

interface ProviderLogoStyle {
  background: string;
  iconScale: number;
  color?: string;
}

export function createProviderLogo(
  Glyph: IconType,
  style: ProviderLogoStyle,
): ComponentType<{ className?: string }> {
  return function ProviderLogo({ className }: { className?: string }) {
    return (
      <span
        aria-hidden="true"
        className={cn(
          "flex items-center justify-center overflow-hidden border border-black/10",
          className,
        )}
        style={{ background: style.background }}
      >
        <Glyph
          color={style.color}
          size="100%"
          style={{ transform: `scale(${style.iconScale})` }}
        />
      </span>
    );
  };
}
