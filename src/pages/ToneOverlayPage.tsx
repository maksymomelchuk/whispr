import { PencilSimpleIcon, XIcon } from "@phosphor-icons/react";
import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useState } from "react";
import { toast } from "sonner";

import { SectionCard } from "@/components/SectionCard";
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
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Textarea } from "@/components/ui/textarea";

import { useSettings } from "../context/SettingsContext";
import { usePersistedToggle } from "../hooks/usePersistedToggle";
import {
  clearToneAppOverride,
  getAppsSeenInHistory,
  setToneOverlayEnabled as persistToneOverlay,
  setToneAppCustomPrompt,
  setToneAppOverride,
} from "../lib/api";
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
    <div className="flex size-6 shrink-0 items-center justify-center rounded-[5px] bg-muted text-[11px] font-medium text-muted-foreground">
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
    let unlisten: (() => void) | null = null;
    listen("stats-updated", () => loadSeenApps())
      .then((u) => {
        unlisten = u;
      })
      .catch((e) => console.error("stats-updated listen failed", e));
    return () => {
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
    } catch (e) {
      toast.error("Couldn't update tone override", { description: String(e) });
    }
  };

  const removeOverride = async (bundleId: string) => {
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
      setSeenApps((prev) =>
        prev.map((a) =>
          a.bundle_id === bundleId
            ? { ...a, tone_override: null, custom_prompt: null }
            : a,
        ),
      );
    } catch (e) {
      toast.error("Couldn't remove tone override", { description: String(e) });
    }
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
    <div className="p-6 flex flex-col gap-8">
      <SectionCard title="Tone of voice">
        <div className="flex flex-col gap-3">
          <ToggleRow
            id="tone-overlay-enabled"
            label="Adapt tone to app"
            info="Matches formatting to the app you dictate into — email gets formal punctuation, messaging stays casual, code honors spoken casing cues like 'underscore' and 'dot'. Presets adjust punctuation, capitalization, line breaks, and (for Technical) identifier casing; a custom prompt can do more."
            checked={settings.ai_cleanup_tone_overlay_enabled}
            onCheckedChange={toneOverlay.toggle}
          />
          {settings.ai_cleanup_tone_overlay_enabled && (
            <div className="flex flex-col gap-2 pt-1">
              <div className="flex items-center justify-between gap-2">
                <p className="text-xs font-medium text-foreground">
                  Per-app overrides
                </p>
                {candidateApps.length > 0 && (
                  <div className="flex items-center gap-2">
                    <Select value="" onValueChange={handleAddOverride}>
                      <SelectTrigger className="h-7 w-40 text-xs">
                        <SelectValue placeholder="+ Add app override" />
                      </SelectTrigger>
                      <SelectContent>
                        {candidateApps.map((app) => (
                          <SelectItem key={app.bundle_id} value={app.bundle_id}>
                            <span className="flex items-center gap-2">
                              <AppIcon
                                icon={app.icon_data_url}
                                name={app.app_name}
                              />
                              {app.app_name}
                            </span>
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                    <span className="size-6 shrink-0" aria-hidden />
                    <span className="size-6 shrink-0" aria-hidden />
                  </div>
                )}
              </div>
              {overriddenApps.length === 0 ? (
                <p className="text-xs text-muted-foreground">
                  Every app uses its automatic tone. Add an override to
                  customize one.
                </p>
              ) : (
                <div className="flex flex-col">
                  {overriddenApps.map((app) => {
                    const isCustom = app.custom_prompt !== null;
                    return (
                      <div
                        key={app.bundle_id}
                        className="flex items-center gap-2 py-1.5"
                      >
                        <AppIcon icon={app.icon_data_url} name={app.app_name} />
                        <span className="min-w-0 truncate text-[13px] text-foreground">
                          {app.app_name}
                        </span>
                        <Select
                          value={
                            isCustom
                              ? "custom"
                              : (app.tone_override ?? "neutral")
                          }
                          onValueChange={(v) => {
                            if (v === "custom") {
                              setEditing({
                                bundleId: app.bundle_id,
                                appName: app.app_name,
                                text: app.custom_prompt ?? "",
                              });
                            } else {
                              void applyPreset(app.bundle_id, v as TonePreset);
                            }
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
                        {isCustom ? (
                          <button
                            type="button"
                            aria-label={`Edit ${app.app_name} custom prompt`}
                            onClick={() =>
                              setEditing({
                                bundleId: app.bundle_id,
                                appName: app.app_name,
                                text: app.custom_prompt ?? "",
                              })
                            }
                            className="shrink-0 rounded p-1 text-muted-foreground transition-colors hover:text-foreground"
                          >
                            <PencilSimpleIcon className="size-4" />
                          </button>
                        ) : (
                          <span className="size-6 shrink-0" aria-hidden />
                        )}
                        <button
                          type="button"
                          aria-label={`Remove ${app.app_name} override`}
                          onClick={() => removeOverride(app.bundle_id)}
                          className="shrink-0 rounded p-1 text-muted-foreground transition-colors hover:text-foreground"
                        >
                          <XIcon className="size-4" />
                        </button>
                      </div>
                    );
                  })}
                </div>
              )}
            </div>
          )}
        </div>
      </SectionCard>

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
    </div>
  );
}
