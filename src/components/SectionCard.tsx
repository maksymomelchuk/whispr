import type { HTMLAttributes, ReactNode } from "react";

import { cn } from "@/lib/utils";

import { InfoTip } from "./InfoTip";

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
      className={cn(
        "rounded-[10px] border border-border bg-card p-4",
        className,
      )}
      {...rest}
    >
      {showHeader && (
        <div className="mb-2 flex items-center gap-2">
          {title !== undefined && (
            <h2 className="m-0 text-[13px] font-semibold">{title}</h2>
          )}
          {info && <InfoTip text={info} />}
          {status}
        </div>
      )}
      {children}
    </section>
  );
}
