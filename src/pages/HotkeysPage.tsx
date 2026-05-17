import { Trash } from "@phosphor-icons/react";
import { useCallback, useEffect, useState } from "react";

import { Alert } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { EmptyRowCard } from "@/components/EmptyRowCard";
import { Keycap, ShortcutKeycaps } from "@/components/Keycap";
import { RowCard } from "@/components/RowCard";
import { SectionHeader } from "@/components/SectionHeader";

import { useSettings } from "../context/SettingsContext";
import { useFlash } from "../hooks/useFlash";
import { usePtt } from "../hooks/usePtt";
import { setHotkeyBindings, setShortcutCapturePaused } from "../lib/api";
import {
  collectModifiers,
  hasConflict,
  isModifierCode,
  shortcutKey,
  shortcutsEqual,
} from "../lib/shortcut";
import type { HotkeyBinding, Mode, Shortcut } from "../lib/types";

const CONFLICT_MESSAGE =
  "Two bindings use the same shortcut. Remove the conflict before saving.";

const DEFAULT_SHORTCUT: Shortcut = { key: "AltRight", modifiers: [] };

interface RecorderTarget {
  modeId: string;
  bindingIndex: number | null;
  current: Shortcut;
}

function ArmedDot() {
  return (
    <span
      aria-label="Currently held"
      className="relative inline-flex size-1.5 shrink-0"
    >
      <span className="motion-safe:animate-ping absolute inset-0 rounded-full bg-ring/60" />
      <span className="relative inline-flex size-1.5 rounded-full bg-ring" />
    </span>
  );
}

function BindingRow({
  binding,
  conflict,
  armed,
  flashing,
  onEdit,
  onRemove,
}: {
  binding: HotkeyBinding;
  conflict: boolean;
  armed: boolean;
  flashing: boolean;
  onEdit: () => void;
  onRemove: () => void;
}) {
  const tone = conflict ? "destructive" : armed ? "accent" : "neutral";
  return (
    <RowCard tone={tone} flashing={flashing}>
      <div className="flex flex-1 min-w-0 items-center gap-2.5 flex-wrap">
        {armed && <ArmedDot />}
        <ShortcutKeycaps shortcut={binding.shortcut} tone={tone} />
        {binding.shortcut.is_double_tap && (
          <Badge variant="accent" className="text-eyebrow uppercase">
            Double-tap
          </Badge>
        )}
        {conflict && (
          <span className="text-form-label text-destructive/85">Conflict</span>
        )}
      </div>

      <div className="flex items-center gap-0.5 shrink-0 transform-gpu opacity-65 group-hover:opacity-100 transition-opacity">
        <Button
          variant="ghost"
          size="xs"
          className="text-muted-foreground hover:text-foreground"
          onClick={onEdit}
        >
          Re-record
        </Button>

        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant="ghost"
              size="icon-sm"
              aria-label="Remove binding"
              onClick={onRemove}
              className="transition-colors text-muted-foreground/70 hover:text-destructive"
            >
              <Trash size={15} />
            </Button>
          </TooltipTrigger>
          <TooltipContent>Remove</TooltipContent>
        </Tooltip>
      </div>
    </RowCard>
  );
}

function RecordingRow({
  initial,
  onSave,
  onCancel,
}: {
  initial: Shortcut;
  onSave: (shortcut: Shortcut) => void;
  onCancel: () => void;
}) {
  const [captured, setCaptured] = useState<Shortcut | null>(null);
  const [isDoubleTap, setIsDoubleTap] = useState<boolean>(
    initial.is_double_tap ?? false,
  );

  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        onCancel();
        return;
      }
      e.preventDefault();
      e.stopPropagation();
      const code = e.code;
      if (isModifierCode(code)) {
        setCaptured({ key: code, modifiers: [] });
        return;
      }
      setCaptured({ key: code, modifiers: collectModifiers(e) });
    },
    [onCancel],
  );

  useEffect(() => {
    setShortcutCapturePaused(true).catch(() => {});
    window.addEventListener("keydown", handleKeyDown, { capture: true });
    return () => {
      window.removeEventListener("keydown", handleKeyDown, { capture: true });
      setShortcutCapturePaused(false).catch(() => {});
    };
  }, [handleKeyDown]);

  const doubleTapChanged = isDoubleTap !== (initial.is_double_tap ?? false);
  const hasChanges = captured !== null || doubleTapChanged;
  const effective: Shortcut = {
    ...(captured ?? initial),
    is_double_tap: isDoubleTap,
  };

  return (
    <RowCard
      tone="accent"
      interactive={false}
      className="shadow-sm"
    >
      <div className="flex flex-1 min-w-0 items-center gap-3 flex-wrap">
        {captured ? (
          <ShortcutKeycaps shortcut={captured} tone="accent" />
        ) : (
          <Tooltip delayDuration={0}>
            <TooltipTrigger asChild>
              <span className="flex items-center gap-2 text-xs font-medium text-muted-foreground">
                <span className="relative inline-flex size-2">
                  <span className="motion-safe:animate-ping absolute inset-0 rounded-full bg-destructive/70" />
                  <span className="relative inline-flex size-2 rounded-full bg-destructive" />
                </span>
                Listening… press your shortcut
              </span>
            </TooltipTrigger>
            <TooltipContent>Press Esc to cancel</TooltipContent>
          </Tooltip>
        )}
        <label className="ml-1 flex items-center gap-1.5 text-help text-muted-foreground cursor-pointer select-none">
          <Switch
            checked={isDoubleTap}
            onCheckedChange={setIsDoubleTap}
          />
          Double-tap
        </label>
      </div>

      <div className="flex items-center gap-1 shrink-0">
        <Button
          variant="ghost"
          size="xs"
          onClick={onCancel}
        >
          Cancel
        </Button>
        <Button
          size="xs"
          disabled={!hasChanges}
          onClick={() => onSave(effective)}
        >
          Save
        </Button>
      </div>
    </RowCard>
  );
}

function EmptyModeCard({ onAdd }: { onAdd: () => void }) {
  return (
    <EmptyRowCard
      preview={
        <div className="flex items-center gap-1.5">
          <Keycap tone="phantom">⌥</Keycap>
          <Keycap tone="phantom">⌘</Keycap>
          <Keycap tone="phantom">K</Keycap>
        </div>
      }
      action="Add hotkey"
      onClick={onAdd}
    />
  );
}

export function HotkeysPage() {
  const { settings, setSettings } = useSettings();
  const { activeShortcut } = usePtt();
  const [recorderTarget, setRecorderTarget] = useState<RecorderTarget | null>(
    null,
  );
  const [error, setError] = useState<string | null>(null);
  const { flash, isFlashing } = useFlash();

  if (!settings) return null;

  const bindings = settings.hotkey_bindings;

  const persist = async (
    next: HotkeyBinding[],
    flashSig?: { modeId: string; shortcut: Shortcut },
  ) => {
    const hasAnyConflict = next.some((_, i) => hasConflict(next, i));
    if (hasAnyConflict) {
      setError(CONFLICT_MESSAGE);
      return;
    }
    setError(null);
    try {
      await setHotkeyBindings(next);
      setSettings((s) => (s ? { ...s, hotkey_bindings: next } : s));
      if (flashSig) {
        flash(rowFlashId(flashSig.modeId, flashSig.shortcut));
      }
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
    await persist(next, { modeId, shortcut });
  };

  const modeBindings = (mode: Mode) =>
    bindings
      .map((b, i) => ({ binding: b, index: i }))
      .filter(({ binding }) => binding.mode_id === mode.id);

  const rowFlashId = (modeId: string, shortcut: Shortcut) =>
    `${modeId}|${shortcutKey(shortcut)}`;

  return (
    <div className="p-6 flex flex-col gap-8">
      {error && (
        <Alert variant="destructive" className="font-medium">
          {error}
        </Alert>
      )}

      {settings.modes.map((mode, modeIdx) => {
        const rows = modeBindings(mode);
        const isDefault = mode.id === settings.default_mode_id;
        const isRecordingForThisMode = recorderTarget?.modeId === mode.id;
        const recordingExistingIndex =
          isRecordingForThisMode && recorderTarget?.bindingIndex !== null
            ? recorderTarget?.bindingIndex
            : null;
        const recordingNewBinding =
          isRecordingForThisMode && recorderTarget?.bindingIndex === null;
        const startRecording = (
          bindingIndex: number | null,
          current: Shortcut,
        ) => setRecorderTarget({ modeId: mode.id, bindingIndex, current });

        return (
          <section key={mode.id} className="flex flex-col gap-2.5">
            <SectionHeader
              index={modeIdx}
              title={mode.name}
              isDefault={isDefault}
            />

            {rows.length === 0 ? (
              recordingNewBinding ? (
                <RecordingRow
                  initial={recorderTarget!.current}
                  onSave={handleRecordSave}
                  onCancel={() => setRecorderTarget(null)}
                />
              ) : (
                <EmptyModeCard
                  onAdd={() => startRecording(null, DEFAULT_SHORTCUT)}
                />
              )
            ) : (
              <div className="flex flex-col gap-2">
                {rows.map(({ binding, index }) => {
                  if (recordingExistingIndex === index) {
                    return (
                      <RecordingRow
                        key={index}
                        initial={recorderTarget!.current}
                        onSave={handleRecordSave}
                        onCancel={() => setRecorderTarget(null)}
                      />
                    );
                  }
                  const flashing = isFlashing(
                    rowFlashId(mode.id, binding.shortcut),
                  );
                  const armed =
                    !!activeShortcut &&
                    shortcutsEqual(activeShortcut, binding.shortcut);
                  return (
                    <BindingRow
                      key={index}
                      binding={binding}
                      conflict={hasConflict(bindings, index)}
                      armed={armed}
                      flashing={flashing}
                      onEdit={() =>
                        startRecording(index, binding.shortcut)
                      }
                      onRemove={() => handleRemove(index)}
                    />
                  );
                })}
                {recordingNewBinding && (
                  <RecordingRow
                    initial={recorderTarget!.current}
                    onSave={handleRecordSave}
                    onCancel={() => setRecorderTarget(null)}
                  />
                )}
              </div>
            )}
          </section>
        );
      })}
    </div>
  );
}
