import { ArrowSquareOutIcon } from "@phosphor-icons/react";
import { useEffect, useState } from "react";

import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import type { EngineDescriptor } from "../lib/speechModelCatalog";

const MASKED_PLACEHOLDER = "••••••••••••";

interface Props {
  descriptor: EngineDescriptor;
  isConfigured: boolean;
  onConfiguredChange: (configured: boolean) => void;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  children?: React.ReactNode;
}

export function ProviderSetupDialog({
  descriptor,
  isConfigured,
  onConfiguredChange,
  open,
  onOpenChange,
  children,
}: Props) {
  const [keyValue, setKeyValue] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (open) {
      setKeyValue("");
      setError(null);
      setSaving(false);
    }
  }, [open]);

  const handleSave = async () => {
    const trimmed = keyValue.trim();
    setError(null);
    setSaving(true);
    try {
      const result = await descriptor.validate(trimmed);
      if (result.kind === "invalid") {
        setError("Key was rejected by the provider.");
        return;
      }
      if (result.kind === "error") {
        setError(`Couldn't validate: ${result.message}`);
        return;
      }
      await descriptor.persist(trimmed);
      onConfiguredChange(true);
      onOpenChange(false);
    } catch (e) {
      setError(`Couldn't save: ${String(e)}`);
    } finally {
      setSaving(false);
    }
  };

  const handleDisconnect = async () => {
    setSaving(true);
    try {
      await descriptor.persist("");
      onConfiguredChange(false);
      onOpenChange(false);
    } catch (e) {
      setError(`Couldn't disconnect: ${String(e)}`);
    } finally {
      setSaving(false);
    }
  };

  const { logo: Logo, metadata } = descriptor;
  const isSaveDisabled = keyValue.trim() === "" || saving;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-md" showCloseButton>
        <DialogHeader>
          <div className="flex items-center gap-3 mb-1">
            <Logo className="h-8 w-8 shrink-0 rounded-md" />
            <DialogTitle className="text-base">{descriptor.name}</DialogTitle>
          </div>
          <DialogDescription>{descriptor.description}</DialogDescription>
          <div className="flex flex-wrap gap-x-3 gap-y-1 pt-1 text-xs text-muted-foreground">
            <span>{metadata.languages}</span>
            <span>·</span>
            <span>Streaming: {metadata.streaming}</span>
            <span>·</span>
            <span>Diarization: {metadata.diarization}</span>
          </div>
        </DialogHeader>

        <div className="flex flex-col gap-3">
          {children}

          <div className="flex flex-col gap-1.5">
            <label
              htmlFor="provider-api-key"
              className="text-xs font-medium text-muted-foreground"
            >
              API Key
            </label>
            <Input
              id="provider-api-key"
              type="password"
              placeholder={isConfigured ? MASKED_PLACEHOLDER : descriptor.keyPlaceholder}
              value={keyValue}
              onChange={(e) => {
                setKeyValue(e.target.value);
                setError(null);
              }}
              disabled={saving}
              spellCheck={false}
              autoComplete="off"
              aria-label="API Key"
            />
          </div>

          <a
            href={descriptor.helpUrl}
            target="_blank"
            rel="noopener noreferrer"
            className="inline-flex items-center gap-1 text-xs text-primary hover:underline w-fit"
          >
            Get your {descriptor.name} API key here
            <ArrowSquareOutIcon size={12} />
          </a>

          {error && (
            <p className="text-xs text-destructive" role="alert">
              {error}
            </p>
          )}
        </div>

        <DialogFooter className="flex items-center justify-between sm:justify-between gap-2">
          <div className="flex gap-2">
            <DialogClose asChild>
              <Button type="button" variant="outline" size="sm" disabled={saving}>
                Cancel
              </Button>
            </DialogClose>
            <Button
              type="button"
              size="sm"
              disabled={isSaveDisabled}
              onClick={handleSave}
            >
              {saving ? "Saving…" : "Save"}
            </Button>
          </div>
          {isConfigured && (
            <Button
              type="button"
              variant="ghost"
              size="sm"
              disabled={saving}
              onClick={handleDisconnect}
              className="text-muted-foreground hover:text-destructive"
            >
              Disconnect
            </Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
