import type { HTMLAttributes, ReactNode } from "react";

import { cn } from "@/lib/utils";

import { InfoTip } from "./InfoTip";
import { SectionHeader } from "./SectionHeader";

interface Props extends Omit<HTMLAttributes<HTMLElement>, "title"> {
  title?: ReactNode;
  info?: string;
  status?: ReactNode;
}

export function SectionCard({
  title,
  info,
  status,
  className,
  children,
  ...rest
}: Props) {
  const showHeader = title !== undefined || info || status;
  return (
    <section
      data-slot="section-card"
      className={cn("flex flex-col gap-2.5 rounded-[10px]", className)}
      {...rest}
    >
      {showHeader && title !== undefined && (
        <SectionHeader
          title={title}
          badge={info ? <InfoTip text={info} /> : undefined}
          trailing={status}
        />
      )}
      <div className="flex flex-col">{children}</div>
    </section>
  );
}
