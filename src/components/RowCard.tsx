import { forwardRef, type HTMLAttributes, type ReactNode } from "react";

import { cn } from "@/lib/utils";

export type RowCardTone = "neutral" | "destructive" | "accent" | "dashed";

interface BaseProps {
  tone?: RowCardTone;
  flashing?: boolean;
  interactive?: boolean;
  className?: string;
  children: ReactNode;
}

const BASE =
  "group relative flex items-center gap-3 rounded-[10px] border pl-3 pr-2 py-2.5 " +
  "shadow-xs transition-[border-color,box-shadow,background-color,outline-color] duration-150 " +
  "outline outline-2 outline-offset-0";

const TONES: Record<RowCardTone, string> = {
  neutral: "border-border bg-card",
  destructive: "border-destructive/45 bg-destructive/[0.04]",
  accent: "border-ring/60 bg-ring/[0.04]",
  dashed: "border-dashed border-border/80 bg-card/30",
};

const INTERACTIVE_TONES: Record<RowCardTone, string> = {
  neutral: "hover:border-ring/55 hover:shadow-sm",
  destructive: "hover:border-destructive/65",
  accent: "",
  dashed: "hover:border-ring/55 hover:bg-card hover:shadow-xs",
};

function rowClasses(
  tone: RowCardTone,
  flashing: boolean,
  interactive: boolean,
  extra?: string,
) {
  return cn(
    BASE,
    TONES[tone],
    interactive && INTERACTIVE_TONES[tone],
    flashing
      ? "outline-ring/45"
      : "outline-transparent motion-safe:duration-[600ms]",
    extra,
  );
}

export const RowCard = forwardRef<HTMLDivElement, BaseProps & HTMLAttributes<HTMLDivElement>>(
  function RowCard(
    {
      tone = "neutral",
      flashing = false,
      interactive = true,
      className,
      children,
      ...rest
    },
    ref,
  ) {
    return (
      <div
        ref={ref}
        className={rowClasses(tone, flashing, interactive, className)}
        {...rest}
      >
        {children}
      </div>
    );
  },
);

interface ButtonProps
  extends BaseProps,
    Omit<React.ButtonHTMLAttributes<HTMLButtonElement>, "children"> {}

export const RowCardButton = forwardRef<HTMLButtonElement, ButtonProps>(
  function RowCardButton(
    {
      tone = "dashed",
      flashing = false,
      interactive = true,
      className,
      children,
      ...rest
    },
    ref,
  ) {
    return (
      <button
        ref={ref}
        type="button"
        className={cn(
          rowClasses(tone, flashing, interactive, className),
          "text-left focus-visible:outline-none focus-visible:ring-[3px] focus-visible:ring-ring/50",
        )}
        {...rest}
      >
        {children}
      </button>
    );
  },
);
