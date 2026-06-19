import type { ReactNode } from "react";

import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { cn } from "@/lib/utils";

import { InfoTip } from "./InfoTip";

interface Props {
  id: string;
  label: ReactNode;
  info?: string;
  value: string;
  options: { label: string; value: string }[];
  onValueChange: (value: string) => void;
  disabled?: boolean;
  className?: string;
}

export function SelectRow({
  id,
  label,
  info,
  value,
  options,
  onValueChange,
  disabled,
  className,
}: Props) {
  return (
    <div
      data-slot="select-row"
      className={cn("flex items-center justify-between gap-4 py-2", className)}
    >
      <Label
        htmlFor={id}
        className={cn(
          "inline-flex items-center gap-2 text-[13px] text-foreground cursor-pointer select-none",
          disabled && "opacity-60 cursor-not-allowed",
        )}
      >
        {label}
        {info && <InfoTip text={info} />}
      </Label>
      <Select value={value} onValueChange={onValueChange} disabled={disabled}>
        <SelectTrigger id={id} size="sm">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {options.map((opt) => (
            <SelectItem key={opt.value} value={opt.value}>
              {opt.label}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </div>
  );
}
