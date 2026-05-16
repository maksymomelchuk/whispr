import { useState } from "react";
import { Trash, Plus } from "@phosphor-icons/react";

import { Button } from "@/components/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";

import { ShortcutRecorder } from "../components/ShortcutRecorder";
import { useSettings } from "../context/SettingsContext";
import { formatShortcut, setHotkeyBindings } from "../lib/api";
import type { HotkeyBinding, Mode, Shortcut } from "../lib/types";

function hasConflict(bindings: HotkeyBinding[], index: number): boolean {
  const b = bindings[index];
  const key = `${b.shortcut.key}|${b.shortcut.modifiers.join(",")}`;
  return bindings.some(
    (other, i) =>
      i !== index &&
      `${other.shortcut.key}|${other.shortcut.modifiers.join(",")}` === key,
  );
}

function findConflict(bindings: HotkeyBinding[]): string | null {
  for (let i = 0; i < bindings.length; i++) {
    if (hasConflict(bindings, i)) {
      return "Two bindings use the same shortcut. Remove the conflict before saving.";
    }
  }
  return null;
}

interface RecorderTarget {
  modeId: string;
  bindingIndex: number | null; // null = new binding for this mode
  current: Shortcut;
}

export function HotkeysPage() {
  const { settings, setSettings } = useSettings();
  const [recorderTarget, setRecorderTarget] = useState<RecorderTarget | null>(
    null,
  );
  const [error, setError] = useState<string | null>(null);

  if (!settings) return null;

  const bindings = settings.hotkey_bindings;

  const persist = async (next: HotkeyBinding[]) => {
    const conflict = findConflict(next);
    if (conflict) {
      setError(conflict);
      return;
    }
    setError(null);
    try {
      await setHotkeyBindings(next);
      setSettings((s) => (s ? { ...s, hotkey_bindings: next } : s));
    } catch (e) {
      setError(String(e));
    }
  };

  const handleRemove = (index: number) => {
    persist(bindings.filter((_, i) => i !== index));
  };

  const handleRecordSave = async (shortcut: Shortcut) => {
    if (!recorderTarget) return;
    const { modeId, bindingIndex } = recorderTarget;
    setRecorderTarget(null);

    let next: HotkeyBinding[];
    if (bindingIndex === null) {
      next = [...bindings, { shortcut, mode_id: modeId }];
    } else {
      next = bindings.map((b, i) =>
        i === bindingIndex ? { ...b, shortcut } : b,
      );
    }
    await persist(next);
  };

  const modeBindings = (mode: Mode) =>
    bindings
      .map((b, i) => ({ binding: b, index: i }))
      .filter(({ binding }) => binding.mode_id === mode.id);

  const defaultShortcut: Shortcut = { key: "AltRight", modifiers: [] };

  return (
    <div className="p-6 flex flex-col gap-6">
      {error && (
        <p className="text-xs text-destructive bg-destructive/10 px-3 py-2 rounded-md">
          {error}
        </p>
      )}

      {settings.modes.map((mode) => {
        const rows = modeBindings(mode);
        return (
          <div key={mode.id} className="flex flex-col gap-2">
            <h3 className="text-sm font-semibold text-foreground">
              {mode.name}
            </h3>

            {rows.length === 0 && (
              <p className="text-xs text-muted-foreground">No hotkeys bound.</p>
            )}

            {rows.map(({ binding, index }) => {
              const conflict = hasConflict(bindings, index);
              return (
                <div
                  key={index}
                  className="flex items-center gap-2 rounded-[10px] border border-border bg-card px-4 py-2"
                >
                  <span
                    className={`flex-1 font-mono text-sm ${conflict ? "text-destructive" : "text-foreground"}`}
                  >
                    {formatShortcut(binding.shortcut)}
                    {conflict && (
                      <span className="ml-2 text-xs font-sans font-normal">
                        ⚠ conflict
                      </span>
                    )}
                  </span>
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() =>
                      setRecorderTarget({
                        modeId: mode.id,
                        bindingIndex: index,
                        current: binding.shortcut,
                      })
                    }
                  >
                    Re-record
                  </Button>
                  <Tooltip>
                    <TooltipTrigger asChild>
                      <Button
                        variant="ghost"
                        size="icon-sm"
                        aria-label="Remove binding"
                        onClick={() => handleRemove(index)}
                        className="text-muted-foreground/70 hover:text-destructive"
                      >
                        <Trash size={15} />
                      </Button>
                    </TooltipTrigger>
                    <TooltipContent>Remove</TooltipContent>
                  </Tooltip>
                </div>
              );
            })}

            <div>
              <Button
                variant="outline"
                size="sm"
                onClick={() =>
                  setRecorderTarget({
                    modeId: mode.id,
                    bindingIndex: null,
                    current: defaultShortcut,
                  })
                }
              >
                <Plus size={14} />
                Add hotkey
              </Button>
            </div>
          </div>
        );
      })}

      {recorderTarget && (
        <ShortcutRecorder
          open
          initial={recorderTarget.current}
          onSave={handleRecordSave}
          onCancel={() => setRecorderTarget(null)}
        />
      )}
    </div>
  );
}
