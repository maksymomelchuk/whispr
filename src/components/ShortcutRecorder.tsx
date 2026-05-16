import { useCallback, useEffect, useState } from "react";

import { formatShortcut, setShortcutCapturePaused } from "../lib/api";
import type { Shortcut } from "../lib/types";
import { Button } from "./ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "./ui/dialog";

interface Props {
  open: boolean;
  initial: Shortcut;
  onSave: (shortcut: Shortcut) => void;
  onCancel: () => void;
}

const MODIFIER_CODES = new Set([
  "MetaLeft",
  "MetaRight",
  "ControlLeft",
  "ControlRight",
  "AltLeft",
  "AltRight",
  "ShiftLeft",
  "ShiftRight",
]);

function isModifierCode(code: string): boolean {
  return MODIFIER_CODES.has(code);
}

function collectModifiers(e: KeyboardEvent): string[] {
  const mods: string[] = [];
  if (e.metaKey) mods.push("Meta");
  if (e.ctrlKey) mods.push("Control");
  if (e.altKey) mods.push("Alt");
  if (e.shiftKey) mods.push("Shift");
  return mods;
}

export function ShortcutRecorder({ open, initial, onSave, onCancel }: Props) {
  const [captured, setCaptured] = useState<Shortcut | null>(null);

  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        onCancel();
        return;
      }
      e.preventDefault();
      e.stopPropagation();

      const modifiers = collectModifiers(e);
      const code = e.code;

      if (isModifierCode(code)) {
        setCaptured({ key: code, modifiers: [] });
        return;
      }

      setCaptured({ key: code, modifiers });
    },
    [onCancel],
  );

  useEffect(() => {
    if (!open) {
      setCaptured(null);
      return;
    }
    // Suppress the OS-level CGEventTap PTT match so pressing the current
    // shortcut here gets captured as a new binding instead of triggering
    // dictation.
    setShortcutCapturePaused(true).catch(() => {});
    window.addEventListener("keydown", handleKeyDown, { capture: true });
    return () => {
      window.removeEventListener("keydown", handleKeyDown, { capture: true });
      setShortcutCapturePaused(false).catch(() => {});
    };
  }, [open, handleKeyDown]);

  const hasChanges = captured !== null;

  return (
    <Dialog open={open} onOpenChange={(next) => !next && onCancel()}>
      <DialogContent showCloseButton={false}>
        <DialogHeader>
          <DialogTitle>Record shortcut</DialogTitle>
          <DialogDescription>
            Press the key combination you want to hold for dictation.
            <br />
            Esc to cancel.
          </DialogDescription>
        </DialogHeader>
        <div className="mt-3 mb-2 px-3 py-5 bg-muted border border-dashed border-input rounded-lg text-center font-mono text-sm text-foreground min-h-6">
          {captured ? formatShortcut(captured) : "Listening…"}
        </div>
        <p className="m-0 text-[11px] text-muted-foreground text-center">
          Current:{" "}
          <span className="font-mono text-xs">{formatShortcut(initial)}</span>
        </p>
        <DialogFooter className="mt-4">
          <Button variant="outline" onClick={onCancel}>
            Cancel
          </Button>
          <Button
            disabled={!hasChanges || !captured}
            onClick={() => captured && onSave(captured)}
          >
            Save
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
