import type { ReactNode } from "react";

import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { cn } from "@/lib/utils";

import { InfoTip } from "./InfoTip";

interface Props {
  id: string;
  label: ReactNode;
  info?: string;
  checked: boolean;
  onCheckedChange: (checked: boolean) => void;
  disabled?: boolean;
  className?: string;
}

export function ToggleRow({
  id,
  label,
  info,
  checked,
  onCheckedChange,
  disabled,
  className,
}: Props) {
  return (
    <div
      data-slot="toggle-row"
      className={cn(
        "flex items-center justify-between gap-4 py-0.5",
        className,
      )}
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
      <Switch
        id={id}
        checked={checked}
        onCheckedChange={onCheckedChange}
        disabled={disabled}
      />
    </div>
  );
}
