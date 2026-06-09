import { TrashIcon } from "@phosphor-icons/react";
import { useCallback, useEffect, useState } from "react";

import { EmptyRowCard } from "@/components/EmptyRowCard";
import { Keycap, ShortcutKeycaps } from "@/components/Keycap";
import { RowCard } from "@/components/RowCard";
import { SectionHeader } from "@/components/SectionHeader";
import { Alert } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Separator } from "@/components/ui/separator";
import { Switch } from "@/components/ui/switch";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";

import { useSettings } from "../context/SettingsContext";
import { useFlash } from "../hooks/useFlash";
import { usePtt } from "../hooks/usePtt";
import { setHotkeyBindings, setShortcutCapturePaused } from "../lib/api";
import {
  collectModifiers,
  hasConflict,
  isModifierCode,
  MOD_LABEL,
  shortcutKey,
  shortcutsEqual,
} from "../lib/shortcut";
import type { HotkeyBinding, Mode, Shortcut } from "../lib/types";
import {
  isPasteLatestBinding,
  isRecoverLatestBinding,
  pasteLatestBinding,
  pttBinding,
  pttModeId,
  recoverLatestBinding,
} from "../lib/types";

const CONFLICT_MESSAGE =
  "Two bindings use the same shortcut. Remove the conflict before saving.";

const DEFAULT_SHORTCUT: Shortcut = { key: "AltRight", modifiers: [] };
const PASTE_LATEST_TARGET_ID = "__paste_latest__";
const RECOVER_LATEST_TARGET_ID = "__recover_latest__";

type RecorderTargetKind =
  | { kind: "mode"; modeId: string }
  | { kind: "paste_latest" }
  | { kind: "recover_latest" };

interface RecorderTarget {
  target: RecorderTargetKind;
  bindingIndex: number | null;
  current: Shortcut;
}

function targetId(target: RecorderTargetKind): string {
  if (target.kind === "mode") return target.modeId;
  if (target.kind === "paste_latest") return PASTE_LATEST_TARGET_ID;
  return RECOVER_LATEST_TARGET_ID;
}

function makeBindingForTarget(
  target: RecorderTargetKind,
  shortcut: Shortcut,
): HotkeyBinding {
  if (target.kind === "mode") {
    return pttBinding(shortcut, target.modeId);
  }
  if (target.kind === "recover_latest") {
    return recoverLatestBinding(shortcut);
  }
  return pasteLatestBinding(shortcut);
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
              <TrashIcon size={15} />
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

  // Pause/resume must fire exactly once per mount, independent of the
  // keydown listener below. Coupling them to handleKeyDown's identity made
  // every parent re-render re-emit interleaved pause/resume IPC calls, and an
  // in-flight `true` could outlive the unmount's `false` — leaving global
  // capture stuck paused until an app restart.
  useEffect(() => {
    setShortcutCapturePaused(true).catch((e) =>
      console.error("failed to pause shortcut capture", e),
    );
    return () => {
      setShortcutCapturePaused(false).catch((e) =>
        console.error("failed to resume shortcut capture", e),
      );
    };
  }, []);

  useEffect(() => {
    window.addEventListener("keydown", handleKeyDown, { capture: true });
    return () => {
      window.removeEventListener("keydown", handleKeyDown, { capture: true });
    };
  }, [handleKeyDown]);

  const doubleTapChanged = isDoubleTap !== (initial.is_double_tap ?? false);
  const hasChanges = captured !== null || doubleTapChanged;
  const effective: Shortcut = {
    ...(captured ?? initial),
    is_double_tap: isDoubleTap,
  };

  return (
    <RowCard tone="accent" interactive={false} className="shadow-sm">
      <div className="flex flex-1 min-h-8 min-w-0 items-center gap-3 flex-wrap">
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
          <Switch checked={isDoubleTap} onCheckedChange={setIsDoubleTap} />
          Double-tap
        </label>
      </div>

      <div className="flex items-center gap-1 shrink-0">
        <Button variant="ghost" size="xs" onClick={onCancel}>
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
          <Keycap tone="phantom">{MOD_LABEL.Alt}</Keycap>
          <Keycap tone="phantom">{MOD_LABEL.Meta}</Keycap>
          <Keycap tone="phantom">K</Keycap>
        </div>
      }
      action="Add hotkey"
      onClick={onAdd}
      className="py-3"
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

  const bindings = settings.hotkey_bindings;

  const persist = async (
    next: HotkeyBinding[],
    flashSig?: { targetId: string; shortcut: Shortcut },
  ) => {
    const hasAnyConflict = next.some((_, i) => hasConflict(next, i));
    if (hasAnyConflict) {
      setError(CONFLICT_MESSAGE);
      return;
    }
    setError(null);
    try {
      await setHotkeyBindings(next);
      setSettings((s) => ({ ...s, hotkey_bindings: next }));
      if (flashSig) {
        flash(rowFlashId(flashSig.targetId, flashSig.shortcut));
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
    const { target, bindingIndex } = recorderTarget;
    setRecorderTarget(null);

    let next: HotkeyBinding[];
    if (bindingIndex === null) {
      next = [...bindings, makeBindingForTarget(target, shortcut)];
    } else {
      next = bindings.map((b, i) =>
        i === bindingIndex ? { ...b, shortcut } : b,
      );
    }
    await persist(next, { targetId: targetId(target), shortcut });
  };

  const modeBindings = (mode: Mode) =>
    bindings
      .map((b, i) => ({ binding: b, index: i }))
      .filter(({ binding }) => pttModeId(binding) === mode.id);

  const pasteLatestBindings = bindings
    .map((b, i) => ({ binding: b, index: i }))
    .filter(({ binding }) => isPasteLatestBinding(binding));

  const recoverLatestBindings = bindings
    .map((b, i) => ({ binding: b, index: i }))
    .filter(({ binding }) => isRecoverLatestBinding(binding));

  const rowFlashId = (targetIdValue: string, shortcut: Shortcut) =>
    `${targetIdValue}|${shortcutKey(shortcut)}`;

  return (
    <div className="p-6 flex flex-col gap-6">
      {error && (
        <Alert variant="destructive" className="font-medium">
          {error}
        </Alert>
      )}

      <div className="flex flex-col gap-5">
        <p className="font-mono text-eyebrow uppercase text-foreground">
          Dictation Modes
        </p>
        {settings.modes.length === 0 && (
          <p className="text-sm text-muted-foreground">
            No profiles yet. Create one in Profiles to bind a hotkey.
          </p>
        )}
        <div className="flex flex-col gap-8">
          {settings.modes.map((mode, modeIdx) => {
            const rows = modeBindings(mode);
            const isRecordingForThisMode =
              recorderTarget?.target.kind === "mode" &&
              recorderTarget.target.modeId === mode.id;
            const recordingExistingIndex =
              isRecordingForThisMode && recorderTarget?.bindingIndex !== null
                ? recorderTarget?.bindingIndex
                : null;
            const recordingNewBinding =
              isRecordingForThisMode && recorderTarget?.bindingIndex === null;
            const startRecording = (
              bindingIndex: number | null,
              current: Shortcut,
            ) =>
              setRecorderTarget({
                target: { kind: "mode", modeId: mode.id },
                bindingIndex,
                current,
              });

            return (
              <section key={mode.id} className="flex flex-col gap-2.5">
                <SectionHeader index={modeIdx} title={mode.name} />

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
                          onEdit={() => startRecording(index, binding.shortcut)}
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
      </div>

      <Separator />

      <PasteLatestSection
        rows={pasteLatestBindings}
        recorderTarget={recorderTarget}
        setRecorderTarget={setRecorderTarget}
        bindings={bindings}
        onSave={handleRecordSave}
        onRemove={handleRemove}
        isFlashing={isFlashing}
        rowFlashId={rowFlashId}
      />

      <Separator />

      <RecoverLatestSection
        rows={recoverLatestBindings}
        recorderTarget={recorderTarget}
        setRecorderTarget={setRecorderTarget}
        bindings={bindings}
        onSave={handleRecordSave}
        onRemove={handleRemove}
        isFlashing={isFlashing}
        rowFlashId={rowFlashId}
      />
    </div>
  );
}

interface PasteLatestSectionProps {
  rows: { binding: HotkeyBinding; index: number }[];
  recorderTarget: RecorderTarget | null;
  setRecorderTarget: (target: RecorderTarget | null) => void;
  bindings: HotkeyBinding[];
  onSave: (shortcut: Shortcut) => void;
  onRemove: (index: number) => void;
  isFlashing: (id: string) => boolean;
  rowFlashId: (targetId: string, shortcut: Shortcut) => string;
}

function PasteLatestSection({
  rows,
  recorderTarget,
  setRecorderTarget,
  bindings,
  onSave,
  onRemove,
  isFlashing,
  rowFlashId,
}: PasteLatestSectionProps) {
  const isRecordingHere = recorderTarget?.target.kind === "paste_latest";
  const recordingExistingIndex =
    isRecordingHere && recorderTarget?.bindingIndex !== null
      ? recorderTarget?.bindingIndex
      : null;
  const recordingNewBinding =
    isRecordingHere && recorderTarget?.bindingIndex === null;

  const startRecording = (bindingIndex: number | null, current: Shortcut) =>
    setRecorderTarget({
      target: { kind: "paste_latest" },
      bindingIndex,
      current,
    });

  return (
    <section className="flex flex-col gap-2.5">
      <SectionHeader title="Paste Latest Transcription" />

      {rows.length === 0 ? (
        recordingNewBinding ? (
          <RecordingRow
            initial={recorderTarget!.current}
            onSave={onSave}
            onCancel={() => setRecorderTarget(null)}
          />
        ) : (
          <EmptyModeCard onAdd={() => startRecording(null, DEFAULT_SHORTCUT)} />
        )
      ) : (
        <div className="flex flex-col gap-2">
          {rows.map(({ binding, index: bindingIndex }) => {
            if (recordingExistingIndex === bindingIndex) {
              return (
                <RecordingRow
                  key={bindingIndex}
                  initial={recorderTarget!.current}
                  onSave={onSave}
                  onCancel={() => setRecorderTarget(null)}
                />
              );
            }
            const flashing = isFlashing(
              rowFlashId(PASTE_LATEST_TARGET_ID, binding.shortcut),
            );
            return (
              <BindingRow
                key={bindingIndex}
                binding={binding}
                conflict={hasConflict(bindings, bindingIndex)}
                armed={false}
                flashing={flashing}
                onEdit={() => startRecording(bindingIndex, binding.shortcut)}
                onRemove={() => onRemove(bindingIndex)}
              />
            );
          })}
        </div>
      )}
    </section>
  );
}

interface RecoverLatestSectionProps {
  rows: { binding: HotkeyBinding; index: number }[];
  recorderTarget: RecorderTarget | null;
  setRecorderTarget: (target: RecorderTarget | null) => void;
  bindings: HotkeyBinding[];
  onSave: (shortcut: Shortcut) => void;
  onRemove: (index: number) => void;
  isFlashing: (id: string) => boolean;
  rowFlashId: (targetId: string, shortcut: Shortcut) => string;
}

function RecoverLatestSection({
  rows,
  recorderTarget,
  setRecorderTarget,
  bindings,
  onSave,
  onRemove,
  isFlashing,
  rowFlashId,
}: RecoverLatestSectionProps) {
  const isRecordingHere = recorderTarget?.target.kind === "recover_latest";
  const recordingExistingIndex =
    isRecordingHere && recorderTarget?.bindingIndex !== null
      ? recorderTarget?.bindingIndex
      : null;
  const recordingNewBinding =
    isRecordingHere && recorderTarget?.bindingIndex === null;

  const startRecording = (bindingIndex: number | null, current: Shortcut) =>
    setRecorderTarget({
      target: { kind: "recover_latest" },
      bindingIndex,
      current,
    });

  return (
    <section className="flex flex-col gap-2.5">
      <SectionHeader title="Recover Latest Transcription" />

      {rows.length === 0 ? (
        recordingNewBinding ? (
          <RecordingRow
            initial={recorderTarget!.current}
            onSave={onSave}
            onCancel={() => setRecorderTarget(null)}
          />
        ) : (
          <EmptyModeCard onAdd={() => startRecording(null, DEFAULT_SHORTCUT)} />
        )
      ) : (
        <div className="flex flex-col gap-2">
          {rows.map(({ binding, index: bindingIndex }) => {
            if (recordingExistingIndex === bindingIndex) {
              return (
                <RecordingRow
                  key={bindingIndex}
                  initial={recorderTarget!.current}
                  onSave={onSave}
                  onCancel={() => setRecorderTarget(null)}
                />
              );
            }
            const flashing = isFlashing(
              rowFlashId(RECOVER_LATEST_TARGET_ID, binding.shortcut),
            );
            return (
              <BindingRow
                key={bindingIndex}
                binding={binding}
                conflict={hasConflict(bindings, bindingIndex)}
                armed={false}
                flashing={flashing}
                onEdit={() => startRecording(bindingIndex, binding.shortcut)}
                onRemove={() => onRemove(bindingIndex)}
              />
            );
          })}
        </div>
      )}
    </section>
  );
}
