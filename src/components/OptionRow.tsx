import type { ReactNode } from "react";

import { cn } from "@/lib/utils";

import { InfoTip } from "./InfoTip";

interface Props {
  htmlFor?: string;
  label: ReactNode;
  param?: string;
  info?: string;
  control?: ReactNode;
  children?: ReactNode;
  className?: string;
}

export function OptionRow({
  htmlFor,
  label,
  param,
  info,
  control,
  children,
  className,
}: Props) {
  const inner = (
    <>
      <div className="flex min-w-0 flex-1 flex-col gap-0.5">
        <div className="inline-flex items-center gap-2 text-md font-semibold text-foreground">
          {label}
          {info && <InfoTip text={info} />}
        </div>
        {param && (
          <div className="font-mono text-kbd text-muted-foreground">
            {param}
          </div>
        )}
        {children}
      </div>
      {control}
    </>
  );
  const base =
    "flex items-start gap-2.5 rounded-md p-2 transition-colors hover:bg-secondary/60";
  if (htmlFor) {
    return (
      <label
        htmlFor={htmlFor}
        className={cn(base, "cursor-pointer", className)}
      >
        {inner}
      </label>
    );
  }
  return <div className={cn(base, className)}>{inner}</div>;
}
