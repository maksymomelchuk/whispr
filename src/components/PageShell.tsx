import type { ReactNode } from "react";

import { PageHeader } from "@/components/PageHeader";

interface PageShellProps {
  title: string;
  description?: string;
  count?: ReactNode;
  children: ReactNode;
}

// Every settings page opens the same way: a page title with optional count and
// description, then a body. ListSurface builds on this and adds list affordances.
export function PageShell({
  title,
  description,
  count,
  children,
}: PageShellProps) {
  return (
    <div className="flex flex-col gap-6 p-6">
      <PageHeader title={title} subtitle={description} trailing={count} />
      {children}
    </div>
  );
}
