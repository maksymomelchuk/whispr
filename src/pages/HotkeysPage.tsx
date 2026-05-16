import { Plus, Trash } from "@phosphor-icons/react";
import { useCallback, useEffect, useState } from "react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";

import { useSettings } from "../context/SettingsContext";
import { usePtt } from "../hooks/usePtt";
import { setHotkeyBindings, setShortcutCapturePaused } from "../lib/api";
import type { HotkeyBinding, Mode, Shortcut } from "../lib/types";

const CONFLICT_MESSAGE =
  "Two bindings use the same shortcut. Remove the conflict before saving.";

const DEFAULT_SHORTCUT: Shortcut = { key: "AltRight", modifiers: [] };
const FLASH_MS = 700;

const MOD_LABEL: Record<string, string> = {
  Meta: "⌘",
  Control: "⌃",
  Alt: "⌥",
  Shift: "⇧",
};

const KEY_LABEL: Record<string, string> = {
  AltRight: "Right ⌥",
  AltLeft: "Left ⌥",
  MetaRight: "Right ⌘",
  MetaLeft: "Left ⌘",
  ControlRight: "Right ⌃",
  ControlLeft: "Left ⌃",
  ShiftRight: "Right ⇧",
  ShiftLeft: "Left ⇧",
  Space: "Space",
  Escape: "Esc",
  Tab: "Tab",
  Enter: "Return",
  Backspace: "Del",
  ArrowUp: "↑",
  ArrowDown: "↓",
  ArrowLeft: "←",
  ArrowRight: "→",
};

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

function displayKey(code: string): string {
  if (KEY_LABEL[code]) return KEY_LABEL[code];
  const k = code.match(/^Key([A-Z])$/);
  if (k) return k[1];
  const d = code.match(/^Digit(\d)$/);
  if (d) return d[1];
  if (/^F\d{1,2}$/.test(code)) return code;
  return code;
}

function shortcutKey(shortcut: Shortcut): string {
  return `${shortcut.key}|${shortcut.modifiers.join(",")}|${shortcut.is_double_tap ?? false}`;
}

function shortcutsEqual(a: Shortcut, b: Shortcut): boolean {
  if (a.key !== b.key) return false;
  if ((a.is_double_tap ?? false) !== (b.is_double_tap ?? false)) return false;
  if (a.modifiers.length !== b.modifiers.length) return false;
  const aSet = new Set(a.modifiers);
  return b.modifiers.every((m) => aSet.has(m));
}

function hasConflict(bindings: HotkeyBinding[], index: number): boolean {
  const key = shortcutKey(bindings[index].shortcut);
  return bindings.some(
    (other, i) => i !== index && shortcutKey(other.shortcut) === key,
  );
}

interface RecorderTarget {
  modeId: string;
  bindingIndex: number | null;
  current: Shortcut;
}

function Keycap({
  children,
  tone = "neutral",
}: {
  children: React.ReactNode;
  tone?: "neutral" | "destructive" | "phantom" | "accent";
}) {
  const base =
    "inline-flex items-center justify-center h-7 min-w-7 px-1.5 rounded-md " +
    "font-mono text-[12px] font-medium leading-none tracking-tight border " +
    "transition-[border-color,background-color,color] duration-150";
  const tones = {
    neutral:
      "border-border/80 bg-background text-foreground " +
      "shadow-[inset_0_-1px_0_0_hsl(var(--border)/0.7),0_1px_0_0_hsl(var(--border)/0.35)] " +
      "group-hover:border-ring/40",
    accent:
      "border-ring/40 bg-ring/10 text-foreground " +
      "shadow-[inset_0_-1px_0_0_hsl(var(--ring)/0.35)]",
    destructive:
      "border-destructive/45 bg-destructive/[0.06] text-destructive " +
      "shadow-[inset_0_-1px_0_0_hsl(var(--destructive)/0.35)]",
    phantom:
      "border-border/60 bg-transparent text-muted-foreground/60",
  };
  return <kbd className={`${base} ${tones[tone]}`}>{children}</kbd>;
}

function ShortcutKeycaps({
  shortcut,
  tone = "neutral",
}: {
  shortcut: Shortcut;
  tone?: "neutral" | "destructive" | "accent";
}) {
  return (
    <div className="flex items-center gap-1.5 flex-wrap">
      {shortcut.modifiers.map((m, i) => (
        <Keycap key={`mod-${i}`} tone={tone}>
          {MOD_LABEL[m] ?? m}
        </Keycap>
      ))}
      <Keycap tone={tone}>{displayKey(shortcut.key)}</Keycap>
    </div>
  );
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
  return (
    <div
      className={
        "group relative flex items-center gap-3 rounded-[10px] border bg-card pl-3 pr-2 py-2.5 " +
        "shadow-xs transition-[border-color,box-shadow,background-color,outline-color] duration-150 " +
        "outline outline-2 outline-offset-0 " +
        (flashing
          ? "outline-ring/45 "
          : "outline-transparent motion-safe:duration-[600ms] ") +
        (conflict
          ? "border-destructive/45 bg-destructive/[0.04] hover:border-destructive/65"
          : armed
            ? "border-ring/60 bg-ring/[0.04]"
            : "border-border hover:border-ring/55 hover:shadow-sm")
      }
    >
      {armed && (
        <span className="absolute -left-3 top-1/2 -translate-y-1/2">
          <ArmedDot />
        </span>
      )}
      <div className="flex flex-1 min-w-0 items-center gap-2.5 flex-wrap">
        <ShortcutKeycaps
          shortcut={binding.shortcut}
          tone={conflict ? "destructive" : armed ? "accent" : "neutral"}
        />
        {binding.shortcut.is_double_tap && (
          <Badge
            variant="neutral"
            className="text-[10px] font-semibold uppercase tracking-[0.06em] bg-primary/12 text-primary border-transparent"
          >
            Double-tap
          </Badge>
        )}
        {conflict && (
          <span className="text-[11px] font-medium text-destructive/85">
            Conflict
          </span>
        )}
      </div>

      <div className="flex items-center gap-0.5 shrink-0 opacity-65 group-hover:opacity-100 transition-opacity">
        <Button
          variant="ghost"
          size="sm"
          className="h-7 px-2 text-[12px] text-muted-foreground hover:text-foreground"
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
              className="text-muted-foreground/70 hover:text-destructive"
            >
              <Trash size={15} />
            </Button>
          </TooltipTrigger>
          <TooltipContent>Remove</TooltipContent>
        </Tooltip>
      </div>
    </div>
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
    <div
      className={
        "relative flex items-center gap-3 rounded-[10px] " +
        "border border-ring/55 bg-ring/[0.04] shadow-sm pl-3 pr-2 py-2.5 " +
        "ring-2 ring-ring/15 transition-colors"
      }
    >
      <div className="flex flex-1 min-w-0 items-center gap-3 flex-wrap">
        {captured ? (
          <ShortcutKeycaps shortcut={captured} tone="accent" />
        ) : (
          <span className="flex items-center gap-2 text-[12.5px] font-medium text-muted-foreground">
            <span className="relative inline-flex size-2">
              <span className="motion-safe:animate-ping absolute inset-0 rounded-full bg-destructive/70" />
              <span className="relative inline-flex size-2 rounded-full bg-destructive" />
            </span>
            Listening… press your shortcut
          </span>
        )}
        <label className="ml-1 flex items-center gap-1.5 text-[11px] text-muted-foreground cursor-pointer select-none">
          <Switch
            size="sm"
            checked={isDoubleTap}
            onCheckedChange={setIsDoubleTap}
          />
          Double-tap
        </label>
      </div>

      <div className="flex items-center gap-1 shrink-0">
        <Button
          variant="ghost"
          size="sm"
          className="h-7 px-2 text-[12px]"
          onClick={onCancel}
        >
          Cancel
        </Button>
        <Button
          size="sm"
          className="h-7 px-3 text-[12px]"
          disabled={!hasChanges}
          onClick={() => onSave(effective)}
        >
          Save
        </Button>
      </div>
    </div>
  );
}

function EmptyModeCard({ onAdd }: { onAdd: () => void }) {
  return (
    <button
      type="button"
      onClick={onAdd}
      className={
        "group flex items-center justify-between gap-3 rounded-[10px] " +
        "border border-dashed border-border/80 bg-card/30 pl-3 pr-4 py-2.5 " +
        "hover:border-ring/55 hover:bg-card hover:shadow-xs " +
        "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50 " +
        "transition-[border-color,background-color,box-shadow] duration-150"
      }
    >
      <div className="flex items-center gap-1.5">
        <Keycap tone="phantom">⌥</Keycap>
        <Keycap tone="phantom">⌘</Keycap>
        <Keycap tone="phantom">K</Keycap>
      </div>
      <span className="flex items-center gap-1.5 text-[12.5px] font-medium text-muted-foreground/80 group-hover:text-foreground transition-colors">
        <Plus size={13} />
        Add hotkey
      </span>
    </button>
  );
}

export function HotkeysPage() {
  const { settings, setSettings } = useSettings();
  const { activeShortcut } = usePtt();
  const [recorderTarget, setRecorderTarget] = useState<RecorderTarget | null>(
    null,
  );
  const [error, setError] = useState<string | null>(null);
  const [flashId, setFlashId] = useState<string | null>(null);

  useEffect(() => {
    if (!flashId) return;
    const t = setTimeout(() => setFlashId(null), FLASH_MS);
    return () => clearTimeout(t);
  }, [flashId]);

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
        setFlashId(`${flashSig.modeId}|${shortcutKey(flashSig.shortcut)}`);
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
    <div className="p-6 flex flex-col gap-7">
      {error && (
        <p
          role="alert"
          className="rounded-md border border-destructive/40 bg-destructive/[0.06] px-3 py-2 text-[12.5px] font-medium text-destructive"
        >
          {error}
        </p>
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
            <div className="flex items-baseline gap-3 pb-1.5 border-b border-border/40">
              <span className="font-mono text-[10.5px] font-semibold uppercase tracking-[0.18em] text-muted-foreground/55 tabular-nums">
                {String(modeIdx + 1).padStart(2, "0")}
              </span>
              <h3 className="text-[14px] font-semibold text-foreground tracking-[-0.005em]">
                {mode.name}
              </h3>
              {isDefault && (
                <Badge className="text-[9.5px] font-semibold uppercase tracking-[0.08em] px-1.5 py-0">
                  Default
                </Badge>
              )}
            </div>

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
              <>
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
                    const flashing =
                      flashId ===
                      rowFlashId(mode.id, binding.shortcut);
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
              </>
            )}
          </section>
        );
      })}
    </div>
  );
}
