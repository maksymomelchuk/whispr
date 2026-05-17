import * as React from "react";
import { cva, type VariantProps } from "class-variance-authority";

import { cn } from "@/lib/utils";

const badgeVariants = cva(
  "inline-flex items-center rounded-full px-1.5 py-0.5 text-[10px] font-medium tracking-wide whitespace-nowrap select-none",
  {
    variants: {
      variant: {
        neutral: "bg-muted text-muted-foreground",
        accent: "bg-primary/10 text-primary",
        warn: "bg-amber-500/15 text-amber-700 dark:text-amber-400",
        error:
          "bg-destructive/15 text-destructive dark:bg-destructive/20 dark:text-red-300",
      },
    },
    defaultVariants: { variant: "neutral" },
  },
);

function Badge({
  className,
  variant,
  ...props
}: React.ComponentProps<"span"> & VariantProps<typeof badgeVariants>) {
  return (
    <span
      data-slot="badge"
      className={cn(badgeVariants({ variant }), className)}
      {...props}
    />
  );
}

export { Badge, badgeVariants };
