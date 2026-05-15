import { type HTMLAttributes, forwardRef } from "react";

interface SeparatorProps extends HTMLAttributes<HTMLDivElement> {
  orientation?: "horizontal" | "vertical";
}

export const Separator = forwardRef<HTMLDivElement, SeparatorProps>(
  ({ className = "", orientation = "horizontal", ...props }, ref) => (
    <div
      ref={ref}
      role="separator"
      aria-orientation={orientation}
      className={
        orientation === "horizontal"
          ? `h-px w-full bg-border ${className}`
          : `h-full w-px bg-border ${className}`
      }
      {...props}
    />
  ),
);
Separator.displayName = "Separator";
