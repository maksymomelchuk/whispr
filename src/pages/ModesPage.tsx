import { useState } from "react";
import {
  Copy,
  PencilSimple,
  Star,
  Trash,
} from "@phosphor-icons/react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Sheet,
  SheetContent,
  SheetFooter,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";

import { useSettings } from "../context/SettingsContext";
import {
  addMode,
  deleteMode,
  duplicateMode,
  getSettings,
  setDefaultMode,
  updateMode,
} from "../lib/api";
import type { Mode, ModeLanguage } from "../lib/types";
import { ToggleRow } from "../components/ToggleRow";

function languageSummary(lang: ModeLanguage): string {
  if (lang.kind === "auto") return "Auto-detect";
  return lang.code.toUpperCase();
}

function translateSummary(mode: Mode): string {
  if (mode.translate.kind === "off") return "Off";
  return `→ ${mode.translate.target.toUpperCase()}`;
}

function ModeRow({
  mode,
  isDefault,
  isLast,
  onEdit,
  onDuplicate,
  onDelete,
  onSetDefault,
}: {
  mode: Mode;
  isDefault: boolean;
  isLast: boolean;
  onEdit: () => void;
  onDuplicate: () => void;
  onDelete: () => void;
  onSetDefault: () => void;
}) {
  const deleteDisabled = isLast || isDefault;
  let deleteTooltip: string | null = null;
  if (isLast) deleteTooltip = "Cannot delete the only remaining mode";
  else if (isDefault) deleteTooltip = "Set a different default before deleting";

  return (
    <div className="flex items-center gap-3 rounded-[10px] border border-border bg-card px-4 py-3">
      <div className="flex flex-1 min-w-0 flex-col gap-0.5">
        <div className="flex items-center gap-2 flex-wrap">
          <span className="text-sm font-semibold text-foreground">
            {mode.name}
          </span>
          {isDefault && (
            <Badge className="text-[10px]">Default</Badge>
          )}
          {mode.ai_cleanup.enabled && (
            <Badge variant="neutral" className="text-[10px]">
              Cleanup
            </Badge>
          )}
        </div>
        <span className="text-xs text-muted-foreground">
          {languageSummary(mode.language)}
          {mode.translate.kind !== "off" && (
            <> · {translateSummary(mode)}</>
          )}
        </span>
      </div>

      <div className="flex items-center gap-1 shrink-0">
        {!isDefault && (
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                variant="ghost"
                size="icon-sm"
                aria-label="Set as default"
                onClick={onSetDefault}
              >
                <Star size={15} />
              </Button>
            </TooltipTrigger>
            <TooltipContent>Set as default</TooltipContent>
          </Tooltip>
        )}
        {isDefault && (
          <Button
            variant="ghost"
            size="icon-sm"
            aria-label="Default mode"
            disabled
            className="opacity-100 text-amber-500"
          >
            <Star size={15} weight="fill" />
          </Button>
        )}

        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant="ghost"
              size="icon-sm"
              aria-label="Edit"
              onClick={onEdit}
            >
              <PencilSimple size={15} />
            </Button>
          </TooltipTrigger>
          <TooltipContent>Edit</TooltipContent>
        </Tooltip>

        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant="ghost"
              size="icon-sm"
              aria-label="Duplicate"
              onClick={onDuplicate}
            >
              <Copy size={15} />
            </Button>
          </TooltipTrigger>
          <TooltipContent>Duplicate</TooltipContent>
        </Tooltip>

        <Tooltip>
          <TooltipTrigger asChild>
            <span>
              <Button
                variant="ghost"
                size="icon-sm"
                aria-label="Delete"
                disabled={deleteDisabled}
                onClick={onDelete}
                className="text-muted-foreground/70 hover:text-destructive"
              >
                <Trash size={15} />
              </Button>
            </span>
          </TooltipTrigger>
          {deleteTooltip && (
            <TooltipContent>{deleteTooltip}</TooltipContent>
          )}
        </Tooltip>
      </div>
    </div>
  );
}

function ModeEditor({
  mode,
  isNew,
  onClose,
  onSaved,
}: {
  mode: Mode;
  isNew: boolean;
  onClose: () => void;
  onSaved: (mode: Mode) => void;
}) {
  const [draft, setDraft] = useState<Mode>(mode);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const langKind = draft.language.kind;
  const langCode = draft.language.kind === "exact" ? draft.language.code : "en";

  const setName = (name: string) => setDraft((d) => ({ ...d, name }));
  const setLangKind = (kind: "auto" | "exact") =>
    setDraft((d) => ({
      ...d,
      language: kind === "auto" ? { kind: "auto" } : { kind: "exact", code: langCode },
    }));
  const setLangCode = (code: string) =>
    setDraft((d) => ({ ...d, language: { kind: "exact", code } }));
  const setCleanup = (enabled: boolean) =>
    setDraft((d) => ({
      ...d,
      ai_cleanup: { ...d.ai_cleanup, enabled },
    }));
  const setUseDictionary = (use_dictionary: boolean) =>
    setDraft((d) => ({ ...d, use_dictionary }));
  const setUseSnippets = (use_snippets: boolean) =>
    setDraft((d) => ({ ...d, use_snippets }));

  const handleSave = async () => {
    setSaving(true);
    setError(null);
    try {
      if (isNew) {
        await addMode(draft);
      } else {
        await updateMode(draft);
      }
      onSaved(draft);
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  const translateLabel =
    draft.translate.kind === "apple"
      ? `→ ${draft.translate.target.toUpperCase()} (coming soon)`
      : "(coming soon)";

  return (
    <>
      <SheetHeader>
        <SheetTitle>{isNew ? "New Mode" : "Edit Mode"}</SheetTitle>
      </SheetHeader>

      <div className="flex flex-col gap-4 px-4 overflow-y-auto flex-1">
        <div className="flex flex-col gap-1.5">
          <Label htmlFor="mode-name" className="text-[13px]">
            Name
          </Label>
          <Input
            id="mode-name"
            value={draft.name}
            onChange={(e) => setName(e.target.value)}
            autoFocus
            placeholder="Mode name"
          />
        </div>

        <div className="flex flex-col gap-1.5">
          <Label className="text-[13px]">Spoken language</Label>
          <div className="flex gap-2">
            <Select value={langKind} onValueChange={setLangKind}>
              <SelectTrigger size="sm" className="w-36">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="auto">Auto-detect</SelectItem>
                <SelectItem value="exact">Exact code</SelectItem>
              </SelectContent>
            </Select>
            {langKind === "exact" && (
              <Input
                className="h-8 w-24 text-sm"
                value={langCode}
                onChange={(e) => setLangCode(e.target.value.toLowerCase())}
                placeholder="en"
                maxLength={8}
                spellCheck={false}
                autoComplete="off"
              />
            )}
          </div>
        </div>

        <div className="flex flex-col gap-1.5">
          <Label className="text-[13px] text-muted-foreground">
            Translate to
          </Label>
          <Input
            className="h-8 text-sm text-muted-foreground"
            value={translateLabel}
            disabled
            readOnly
          />
        </div>

        <div className="flex flex-col gap-2 pt-1">
          <ToggleRow
            id="cleanup"
            label="AI cleanup"
            checked={draft.ai_cleanup.enabled}
            onCheckedChange={setCleanup}
          />
          <ToggleRow
            id="dictionary"
            label="Use dictionary"
            checked={draft.use_dictionary}
            onCheckedChange={setUseDictionary}
          />
          <ToggleRow
            id="snippets"
            label="Use snippets"
            checked={draft.use_snippets}
            onCheckedChange={setUseSnippets}
          />
        </div>

        {error && (
          <p className="text-xs text-destructive">{error}</p>
        )}
      </div>

      <SheetFooter>
        <Button variant="outline" onClick={onClose} disabled={saving}>
          Cancel
        </Button>
        <Button onClick={handleSave} disabled={saving || !draft.name.trim()}>
          {saving ? "Saving…" : "Save"}
        </Button>
      </SheetFooter>
    </>
  );
}

type EditorState = { mode: Mode; isNew: boolean };

export function ModesPage() {
  const { settings, setSettings } = useSettings();
  const [editor, setEditor] = useState<EditorState | null>(null);

  if (!settings) return null;

  const openEditor = (mode: Mode, isNew = false) => setEditor({ mode, isNew });
  const closeEditor = () => setEditor(null);

  const handleSaved = (saved: Mode) => {
    const wasNew = editor?.isNew ?? false;
    setSettings((s) => {
      if (!s) return s;
      if (wasNew) {
        return { ...s, modes: [...s.modes, saved] };
      }
      return { ...s, modes: s.modes.map((m) => (m.id === saved.id ? saved : m)) };
    });
    closeEditor();
  };

  const handleAddMode = () => {
    const newMode: Mode = {
      id: `mode-${Date.now()}`,
      name: "",
      icon: null,
      language: { kind: "exact", code: "en" },
      translate: { kind: "off" },
      ai_cleanup: { enabled: false, prompt_override: null },
      use_dictionary: true,
      use_snippets: true,
    };
    openEditor(newMode, true);
  };

  const handleDelete = async (id: string) => {
    try {
      await deleteMode(id);
      setSettings((s) =>
        s ? { ...s, modes: s.modes.filter((m) => m.id !== id) } : s,
      );
    } catch (e) {
      console.error(e);
    }
  };

  const handleDuplicate = async (id: string) => {
    try {
      await duplicateMode(id);
      const updated = await getSettings();
      setSettings(updated);
    } catch (e) {
      console.error(e);
    }
  };

  const handleSetDefault = async (id: string) => {
    try {
      await setDefaultMode(id);
      setSettings((s) => (s ? { ...s, default_mode_id: id } : s));
    } catch (e) {
      console.error(e);
    }
  };

  return (
    <div className="p-6 flex flex-col gap-4">
      <div className="flex flex-col gap-2">
        {settings.modes.map((mode) => (
          <ModeRow
            key={mode.id}
            mode={mode}
            isDefault={mode.id === settings.default_mode_id}
            isLast={settings.modes.length === 1}
            onEdit={() => openEditor(mode)}
            onDuplicate={() => handleDuplicate(mode.id)}
            onDelete={() => handleDelete(mode.id)}
            onSetDefault={() => handleSetDefault(mode.id)}
          />
        ))}
      </div>

      <div>
        <Button variant="outline" size="sm" onClick={handleAddMode}>
          + Add mode
        </Button>
      </div>

      <Sheet open={editor !== null} onOpenChange={(open) => !open && closeEditor()}>
        <SheetContent side="right" className="w-80 sm:max-w-80 flex flex-col gap-4">
          {editor && (
            <ModeEditor
              key={editor.mode.id}
              mode={editor.mode}
              isNew={editor.isNew}
              onClose={closeEditor}
              onSaved={handleSaved}
            />
          )}
        </SheetContent>
      </Sheet>
    </div>
  );
}
