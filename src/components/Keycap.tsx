import type { ReactNode } from "react";

import { displayKey, MOD_LABEL } from "@/lib/shortcut";
import type { Shortcut } from "@/lib/types";

export type KeycapTone = "neutral" | "destructive" | "phantom" | "accent";
export type KeycapSize = "sm" | "lg";

const TONES: Record<KeycapTone, string> = {
  neutral:
    "border-border/80 bg-background text-foreground " +
    "shadow-[inset_0_-1px_0_0_hsl(var(--border)/0.7),0_1px_0_0_hsl(var(--border)/0.35)] " +
    "group-hover:border-ring/40",
  accent:
    "border-ring/40 bg-ring/10 text-foreground " +
    "shadow-[inset_0_-1px_0_0_hsl(var(--ring)/0.35)]",
  destructive:
    "border-destructive/45 bg-destructive/[0.06] text-destructive " +
    "shadow-[inset_0_-1px_0_0_hsl(var(--destructive)/0.35)]",
  phantom: "border-border/60 bg-transparent text-muted-foreground/60",
};

const SIZES: Record<KeycapSize, string> = {
  sm: "h-7 min-w-7 px-1.5 text-[12px]",
  lg: "h-10 min-w-10 px-2.5 text-[16px]",
};

const BASE =
  "inline-flex items-center justify-center rounded-md " +
  "font-mono font-medium leading-none tracking-tight border " +
  "transition-[border-color,background-color,color] duration-150";

export function Keycap({
  children,
  tone = "neutral",
  size = "sm",
}: {
  children: ReactNode;
  tone?: KeycapTone;
  size?: KeycapSize;
}) {
  return <kbd className={`${BASE} ${SIZES[size]} ${TONES[tone]}`}>{children}</kbd>;
}

export function ShortcutKeycaps({
  shortcut,
  tone = "neutral",
  size = "sm",
}: {
  shortcut: Shortcut;
  tone?: Exclude<KeycapTone, "phantom">;
  size?: KeycapSize;
}) {
  const gap = size === "lg" ? "gap-2" : "gap-1.5";
  return (
    <div className={`flex items-center ${gap} flex-wrap`}>
      {shortcut.modifiers.map((m, i) => (
        <Keycap key={`mod-${i}`} tone={tone} size={size}>
          {MOD_LABEL[m] ?? m}
        </Keycap>
      ))}
      <Keycap tone={tone} size={size}>
        {displayKey(shortcut.key)}
      </Keycap>
    </div>
  );
}
