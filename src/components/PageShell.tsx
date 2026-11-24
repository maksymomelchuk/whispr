import type { ReactNode } from "react";

import { PageHeader } from "@/components/PageHeader";
import { cn } from "@/lib/utils";

interface PageShellProps {
  title: string;
  description?: string;
  trailing?: ReactNode;
  children: ReactNode;
  className?: string;
}

// Every settings page opens the same way: a page title with optional trailing
// slot and description, then a body. ListSurface builds on this and adds list
// affordances.
export function PageShell({
  title,
  description,
  trailing,
  children,
  className,
}: PageShellProps) {
  return (
    <div className={cn("flex flex-col gap-6 p-6", className)}>
      <PageHeader title={title} subtitle={description} trailing={trailing} />
      {children}
    </div>
  );
}
