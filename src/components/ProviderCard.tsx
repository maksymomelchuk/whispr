import { CheckFatIcon, GearIcon } from "@phosphor-icons/react";
import { useState } from "react";

import { cn } from "@/lib/utils";

import type { EngineDescriptor } from "../lib/speechModelCatalog";
import { ProviderSetupDialog } from "./ProviderSetupDialog";

interface Props {
  descriptor: EngineDescriptor;
  isConfigured: boolean;
  onConfiguredChange: (configured: boolean) => void;
}

export function ProviderCard({
  descriptor,
  isConfigured,
  onConfiguredChange,
}: Props) {
  const [dialogOpen, setDialogOpen] = useState(false);
  const { logo: Logo } = descriptor;

  return (
    <>
      <button
        type="button"
        onClick={() => setDialogOpen(true)}
        className={cn(
          "flex items-center gap-3 rounded-lg border border-border bg-card px-4 py-3",
          "text-left transition-colors hover:bg-accent/40 cursor-pointer w-full",
        )}
      >
        <Logo className="h-8 w-8 shrink-0 rounded-md" />
        <span className="flex-1 min-w-0 truncate text-sm font-medium leading-tight">
          {descriptor.name}
        </span>
        {isConfigured ? (
          <CheckFatIcon
            size={16}
            weight="fill"
            role="img"
            aria-label="Configured"
            className="shrink-0 text-green-600 dark:text-green-500"
          />
        ) : (
          <GearIcon
            size={16}
            role="img"
            aria-label="Set up"
            className="shrink-0 text-muted-foreground/50"
          />
        )}
      </button>

      <ProviderSetupDialog
        descriptor={descriptor}
        isConfigured={isConfigured}
        onConfiguredChange={onConfiguredChange}
        open={dialogOpen}
        onOpenChange={setDialogOpen}
      />
    </>
  );
}
