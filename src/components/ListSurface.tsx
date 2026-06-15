import { MagnifyingGlassIcon } from "@phosphor-icons/react";
import type { ReactNode } from "react";

import { PageShell } from "@/components/PageShell";
import { Input } from "@/components/ui/input";

interface SearchConfig {
  value: string;
  onChange: (value: string) => void;
  placeholder: string;
}

export function ListSearch({ value, onChange, placeholder }: SearchConfig) {
  return (
    <div className="relative max-w-xs">
      <MagnifyingGlassIcon
        size={14}
        aria-hidden
        className="absolute left-2.5 top-1/2 -translate-y-1/2 text-muted-foreground/60"
      />
      <Input
        value={value}
        onChange={(e) => onChange(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Escape" && value) {
            e.preventDefault();
            onChange("");
          }
        }}
        placeholder={placeholder}
        className="h-8 pl-8 text-sm"
      />
    </div>
  );
}

interface ListSurfaceProps {
  title: string;
  description?: string;
  count?: ReactNode;
  search?: SearchConfig;
  children: ReactNode;
}

// Shared scaffold for the list pages: page title with a count, optional search,
// then a body slot the page fills with rows. Search renders only when the page
// passes a config (each page gates that on its own item threshold).
export function ListSurface({
  title,
  description,
  count,
  search,
  children,
}: ListSurfaceProps) {
  return (
    <PageShell title={title} description={description} trailing={count}>
      {search && <ListSearch {...search} />}
      {children}
    </PageShell>
  );
}
