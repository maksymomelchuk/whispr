import { useState } from "react";

import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";
import type { EngineDescriptor } from "../lib/speechModelCatalog";
import { ProviderSetupDialog } from "./ProviderSetupDialog";

interface Props {
  descriptor: EngineDescriptor;
  isConfigured: boolean;
  onConfiguredChange: (configured: boolean) => void;
}

export function ProviderCard({ descriptor, isConfigured, onConfiguredChange }: Props) {
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
        <div className="flex flex-1 flex-col gap-0.5 min-w-0">
          <span className="text-sm font-medium leading-tight">{descriptor.name}</span>
          <Badge variant={isConfigured ? "accent" : "neutral"} className="w-fit">
            {isConfigured ? "Configured" : "Setup"}
          </Badge>
        </div>
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
