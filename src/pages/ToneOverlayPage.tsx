import { PencilSimpleIcon, PlusIcon, XIcon } from "@phosphor-icons/react";
import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useState } from "react";
import { toast } from "sonner";

import { ListRow, RowActionButton } from "@/components/ListRow";
import { ListSurface } from "@/components/ListSurface";
import { RowCard } from "@/components/RowCard";
import { SectionHeader } from "@/components/SectionHeader";
import { ToggleRow } from "@/components/ToggleRow";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Textarea } from "@/components/ui/textarea";

import { useSettings } from "../context/SettingsContext";
import { useFlash } from "../hooks/useFlash";
import { usePersistedToggle } from "../hooks/usePersistedToggle";
import {
  clearToneAppOverride,
  getAppsSeenInHistory,
  setToneOverlayEnabled as persistToneOverlay,
  setToneAppCustomPrompt,
  setToneAppOverride,
} from "../lib/api";
import { toastUndo } from "../lib/toastUndo";
import type { AppToneInfo, TonePreset } from "../lib/types";

const OVERRIDE_OPTIONS: { value: TonePreset | "custom"; label: string }[] = [
  { value: "casual", label: "Casual" },
  { value: "formal", label: "Formal" },
  { value: "technical_casing", label: "Technical" },
  { value: "neutral", label: "Neutral" },
  { value: "custom", label: "Custom…" },
];

function AppIcon({ icon, name }: { icon: string | null; name: string }) {
  if (icon) {
    return (
      <img
        src={icon}
        alt=""
        aria-hidden
        className="size-6 shrink-0 rounded-[5px]"
      />
    );
  }
  return (
    <div
      aria-hidden
      className="flex size-6 shrink-0 items-center justify-center rounded-[5px] bg-muted text-[11px] font-medium text-muted-foreground"
    >
      {name.charAt(0).toUpperCase()}
    </div>
  );
}

type CustomDraft = { bundleId: string; appName: string; text: string };

export function ToneOverlayPage() {
  const { settings, setSettings } = useSettings();

  const toneOverlay = usePersistedToggle(
    settings.ai_cleanup_tone_overlay_enabled,
    persistToneOverlay,
    (next) =>
      setSettings((s) => ({ ...s, ai_cleanup_tone_overlay_enabled: next })),
  );

  const [seenApps, setSeenApps] = useState<AppToneInfo[]>([]);
  const [editing, setEditing] = useState<CustomDraft | null>(null);
  const [pickerOpen, setPickerOpen] = useState(false);
  const { flash, isFlashing } = useFlash();

  const loadSeenApps = useCallback(async () => {
    try {
      setSeenApps(await getAppsSeenInHistory());
    } catch {
      // non-fatal: list stays empty
    }
  }, []);

  useEffect(() => {
    if (settings.ai_cleanup_tone_overlay_enabled) loadSeenApps();
  }, [settings.ai_cleanup_tone_overlay_enabled, loadSeenApps]);

  useEffect(() => {
    if (!settings.ai_cleanup_tone_overlay_enabled) return;
    let cancelled = false;
    let unlisten: (() => void) | null = null;
    listen("stats-updated", () => loadSeenApps())
      .then((u) => {
        if (cancelled) u();
        else unlisten = u;
      })
      .catch((e) => console.error("stats-updated listen failed", e));
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [settings.ai_cleanup_tone_overlay_enabled, loadSeenApps]);

  const applyPreset = async (bundleId: string, preset: TonePreset) => {
    try {
      await setToneAppOverride(bundleId, preset);
      setSettings((s) => {
        const customs = { ...s.tone_app_custom_prompts };
        delete customs[bundleId];
        return {
          ...s,
          tone_app_overrides: { ...s.tone_app_overrides, [bundleId]: preset },
          tone_app_custom_prompts: customs,
        };
      });
      setSeenApps((prev) =>
        prev.map((a) =>
          a.bundle_id === bundleId
            ? { ...a, tone_override: preset, custom_prompt: null }
            : a,
        ),
      );
      flash(bundleId);
    } catch (e) {
      toast.error("Couldn't update tone override", { description: String(e) });
    }
  };

  const removeOverride = (bundleId: string) => {
    const app = seenApps.find((a) => a.bundle_id === bundleId);
    if (!app) return;

    setSeenApps((prev) =>
      prev.map((a) =>
        a.bundle_id === bundleId
          ? { ...a, tone_override: null, custom_prompt: null }
          : a,
      ),
    );

    toastUndo(
      `Removed ${app.app_name} override`,
      async () => {
        try {
          await clearToneAppOverride(bundleId);
          setSettings((s) => {
            const overrides = { ...s.tone_app_overrides };
            const customs = { ...s.tone_app_custom_prompts };
            delete overrides[bundleId];
            delete customs[bundleId];
            return {
              ...s,
              tone_app_overrides: overrides,
              tone_app_custom_prompts: customs,
            };
          });
        } catch (e) {
          toast.error("Couldn't remove tone override", {
            description: String(e),
          });
          setSeenApps((prev) =>
            prev.map((a) =>
              a.bundle_id === bundleId
                ? {
                    ...a,
                    tone_override: app.tone_override,
                    custom_prompt: app.custom_prompt,
                  }
                : a,
            ),
          );
        }
      },
      () => {
        setSeenApps((prev) =>
          prev.map((a) =>
            a.bundle_id === bundleId
              ? {
                  ...a,
                  tone_override: app.tone_override,
                  custom_prompt: app.custom_prompt,
                }
              : a,
          ),
        );
      },
    );
  };

  const saveCustom = async () => {
    if (!editing) return;
    const text = editing.text.trim();
    if (!text) return;
    try {
      await setToneAppCustomPrompt(editing.bundleId, text);
      setSettings((s) => {
        const overrides = { ...s.tone_app_overrides };
        delete overrides[editing.bundleId];
        return {
          ...s,
          tone_app_overrides: overrides,
          tone_app_custom_prompts: {
            ...s.tone_app_custom_prompts,
            [editing.bundleId]: text,
          },
        };
      });
      setSeenApps((prev) =>
        prev.map((a) =>
          a.bundle_id === editing.bundleId
            ? { ...a, tone_override: null, custom_prompt: text }
            : a,
        ),
      );
      flash(editing.bundleId);
      setEditing(null);
    } catch (e) {
      toast.error("Couldn't save custom tone", { description: String(e) });
    }
  };

  const handleAddOverride = (bundleId: string) => {
    const app = seenApps.find((a) => a.bundle_id === bundleId);
    if (!app) return;
    void applyPreset(bundleId, app.tone_preset);
  };

  const overriddenApps = seenApps.filter(
    (a) => a.tone_override !== null || a.custom_prompt !== null,
  );
  const candidateApps = seenApps.filter(
    (a) => a.tone_override === null && a.custom_prompt === null,
  );

  return (
    <ListSurface
      title="Tone of voice"
      description="Adapt formatting to the app you dictate into. Email gets formal punctuation, messaging stays casual, code honors spoken casing cues."
    >
      <RowCard interactive={false}>
        <ToggleRow
          id="tone-overlay-enabled"
          label="Adapt tone to app"
          info="Matches formatting to the app you dictate into — email gets formal punctuation, messaging stays casual, code honors spoken casing cues like 'underscore' and 'dot'. Presets adjust punctuation, capitalization, line breaks, and (for Technical) identifier casing; a custom prompt can do more."
          checked={settings.ai_cleanup_tone_overlay_enabled}
          onCheckedChange={toneOverlay.toggle}
          className="flex-1"
        />
      </RowCard>

      {settings.ai_cleanup_tone_overlay_enabled && (
        <div className="flex flex-col gap-2">
          <SectionHeader
            title="Per-app overrides"
            control={
              candidateApps.length > 0 ? (
                <Button
                  variant="outline"
                  size="xs"
                  onClick={() => setPickerOpen(true)}
                >
                  <PlusIcon />
                  Add app override
                </Button>
              ) : undefined
            }
          />
          {overriddenApps.length === 0 ? (
            <p className="text-xs text-muted-foreground">
              Every app uses its automatic tone. Add an override to customize
              one.
            </p>
          ) : (
            overriddenApps.map((app) => (
              <ToneAppRow
                key={app.bundle_id}
                app={app}
                flashing={isFlashing(app.bundle_id)}
                onApplyPreset={(preset) => applyPreset(app.bundle_id, preset)}
                onEditCustom={() =>
                  setEditing({
                    bundleId: app.bundle_id,
                    appName: app.app_name,
                    text: app.custom_prompt ?? "",
                  })
                }
                onRemove={() => removeOverride(app.bundle_id)}
              />
            ))
          )}
        </div>
      )}

      <AppPickerDialog
        open={pickerOpen}
        onOpenChange={setPickerOpen}
        candidates={candidateApps}
        onSelect={handleAddOverride}
      />

      <Dialog
        open={editing !== null}
        onOpenChange={(open) => {
          if (!open) setEditing(null);
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Custom tone for {editing?.appName}</DialogTitle>
            <DialogDescription>
              This instruction is sent to the cleanup model whenever you dictate
              into {editing?.appName}. Unlike the presets, a custom prompt can
              change wording — the formatting-only guarantee does not apply.
            </DialogDescription>
          </DialogHeader>
          <Textarea
            value={editing?.text ?? ""}
            onChange={(e) =>
              setEditing((cur) =>
                cur ? { ...cur, text: e.target.value } : cur,
              )
            }
            placeholder="e.g. Keep it terse and all-lowercase; never add exclamation marks."
            rows={4}
          />
          <DialogFooter>
            <Button variant="outline" onClick={() => setEditing(null)}>
              Cancel
            </Button>
            <Button onClick={saveCustom} disabled={!editing?.text.trim()}>
              Save
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </ListSurface>
  );
}

function AppPickerDialog({
  open,
  onOpenChange,
  candidates,
  onSelect,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  candidates: AppToneInfo[];
  onSelect: (bundleId: string) => void;
}) {
  const [query, setQuery] = useState("");
  const filtered = candidates.filter((a) =>
    a.app_name.toLowerCase().includes(query.toLowerCase()),
  );

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Add app override</DialogTitle>
        </DialogHeader>
        <Input
          placeholder="Search apps…"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          autoFocus
        />
        <div className="flex max-h-64 flex-col gap-0.5 overflow-y-auto">
          {filtered.map((app) => (
            <button
              key={app.bundle_id}
              className="flex items-center gap-2 rounded-md px-2 py-1.5 text-left text-sm hover:bg-accent"
              onClick={() => {
                onSelect(app.bundle_id);
                onOpenChange(false);
              }}
            >
              <AppIcon icon={app.icon_data_url} name={app.app_name} />
              {app.app_name}
            </button>
          ))}
          {filtered.length === 0 && (
            <p className="px-2 py-1.5 text-xs text-muted-foreground">
              No apps found.
            </p>
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
}

function ToneAppRow({
  app,
  flashing,
  onApplyPreset,
  onEditCustom,
  onRemove,
}: {
  app: AppToneInfo;
  flashing?: boolean;
  onApplyPreset: (preset: TonePreset) => void;
  onEditCustom: () => void;
  onRemove: () => void;
}) {
  const isCustom = app.custom_prompt !== null;

  return (
    <ListRow
      flashing={flashing}
      label={
        <>
          <AppIcon icon={app.icon_data_url} name={app.app_name} />
          <span className="min-w-0 truncate text-[13px] text-foreground">
            {app.app_name}
          </span>
        </>
      }
      meta={
        <Select
          value={isCustom ? "custom" : (app.tone_override ?? "neutral")}
          onValueChange={(v) => {
            if (v === "custom") onEditCustom();
            else onApplyPreset(v as TonePreset);
          }}
        >
          <SelectTrigger className="ml-auto h-7 w-40 shrink-0 text-xs">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {OVERRIDE_OPTIONS.map((opt) => (
              <SelectItem key={opt.value} value={opt.value}>
                {opt.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      }
      actions={
        <>
          {isCustom && (
            <RowActionButton
              icon={<PencilSimpleIcon size={14} />}
              label={`Edit ${app.app_name} custom prompt`}
              onClick={onEditCustom}
            />
          )}
          <RowActionButton
            icon={<XIcon size={14} />}
            label={`Remove ${app.app_name} override`}
            onClick={onRemove}
          />
        </>
      }
    />
  );
}
