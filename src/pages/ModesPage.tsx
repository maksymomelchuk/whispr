import { useState } from "react";
import { useNavigate } from "react-router-dom";
import {
  CaretRight,
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
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
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
  formatShortcut,
  getSettings,
  setDefaultMode,
  updateMode,
} from "../lib/api";
import type { HotkeyBinding, Mode, ModeLanguage } from "../lib/types";
import { ToggleRow } from "../components/ToggleRow";

// Languages Apple Translate supports (code → display name).
const APPLE_TRANSLATE_LANGUAGES: [string, string][] = [
  ["ar", "Arabic"],
  ["zh", "Chinese (Simplified)"],
  ["zh-TW", "Chinese (Traditional)"],
  ["nl", "Dutch"],
  ["en", "English"],
  ["fr", "French"],
  ["de", "German"],
  ["id", "Indonesian"],
  ["it", "Italian"],
  ["ja", "Japanese"],
  ["ko", "Korean"],
  ["pl", "Polish"],
  ["pt", "Portuguese"],
  ["ru", "Russian"],
  ["es", "Spanish"],
  ["th", "Thai"],
  ["tr", "Turkish"],
  ["uk", "Ukrainian"],
  ["vi", "Vietnamese"],
];

function translateLanguageName(code: string): string {
  return APPLE_TRANSLATE_LANGUAGES.find(([c]) => c === code)?.[1] ?? code.toUpperCase();
}

function languageSummary(lang: ModeLanguage): string {
  if (lang.kind === "auto") return "Auto-detect";
  return lang.code.toUpperCase();
}

function translateSummary(mode: Mode): string {
  if (mode.translate.kind === "off") return "Off";
  return `→ ${translateLanguageName(mode.translate.target)}`;
}

function ModeRow({
  mode,
  isDefault,
  isLast,
  bindings,
  onEdit,
  onDuplicate,
  onDelete,
  onSetDefault,
}: {
  mode: Mode;
  isDefault: boolean;
  isLast: boolean;
  bindings: HotkeyBinding[];
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
          {bindings.length > 0 && (
            <> · {bindings.map((b) => formatShortcut(b.shortcut)).join(", ")}</>
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
  const [promptOpen, setPromptOpen] = useState(
    !!mode.ai_cleanup.prompt_override,
  );

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
  const setPromptOverride = (value: string) =>
    setDraft((d) => ({
      ...d,
      ai_cleanup: {
        ...d.ai_cleanup,
        prompt_override: value || null,
      },
    }));
  const setTranslate = (value: string) =>
    setDraft((d) => ({
      ...d,
      translate:
        value === "off"
          ? { kind: "off" }
          : { kind: "apple", target: value },
    }));
  const setUseDictionary = (use_dictionary: boolean) =>
    setDraft((d) => ({ ...d, use_dictionary }));
  const setUseSnippets = (use_snippets: boolean) =>
    setDraft((d) => ({ ...d, use_snippets }));

  const handleSave = async () => {
    setSaving(true);
    setError(null);
    const normalized: Mode = {
      ...draft,
      ai_cleanup: {
        ...draft.ai_cleanup,
        prompt_override: draft.ai_cleanup.prompt_override?.trim() || null,
      },
    };
    try {
      if (isNew) {
        await addMode(normalized);
      } else {
        await updateMode(normalized);
      }
      onSaved(normalized);
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

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
          <Label className="text-[13px]">Translate to</Label>
          <Select
            value={draft.translate.kind === "apple" ? draft.translate.target : "off"}
            onValueChange={setTranslate}
          >
            <SelectTrigger size="sm" className="w-full">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="off">Off</SelectItem>
              {APPLE_TRANSLATE_LANGUAGES.map(([code, name]) => (
                <SelectItem key={code} value={code}>
                  {name}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          {draft.translate.kind === "apple" && (
            <p className="text-[11px] text-muted-foreground">
              Engine: Apple Translate (on-device)
            </p>
          )}
        </div>

        <div className="flex flex-col gap-2 pt-1">
          <ToggleRow
            id="cleanup"
            label="AI cleanup"
            checked={draft.ai_cleanup.enabled}
            onCheckedChange={setCleanup}
          />
          {draft.ai_cleanup.enabled && (
            <Collapsible
              open={promptOpen}
              onOpenChange={setPromptOpen}
              className="ml-[52px] flex flex-col gap-2"
            >
              <CollapsibleTrigger className="flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground transition-colors w-fit">
                <CaretRight
                  size={10}
                  className={`transition-transform ${promptOpen ? "rotate-90" : ""}`}
                />
                Custom prompt
              </CollapsibleTrigger>
              <CollapsibleContent className="flex flex-col gap-2">
                <textarea
                  className="w-full resize-none rounded-md border border-input bg-transparent px-3 py-2 text-sm placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring min-h-[80px]"
                  placeholder="Leave empty to use the default cleanup prompt."
                  value={draft.ai_cleanup.prompt_override ?? ""}
                  onChange={(e) => setPromptOverride(e.target.value)}
                  spellCheck={false}
                />
                <p className="text-[11px] text-muted-foreground leading-relaxed">
                  The text inside <code className="font-mono">&lt;transcript&gt;</code> tags will be your dictation. Your prompt is responsible for treating it as data to transform, not instructions to execute.
                </p>
              </CollapsibleContent>
            </Collapsible>
          )}
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
  const navigate = useNavigate();
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
            bindings={settings.hotkey_bindings.filter(
              (b) => b.mode_id === mode.id,
            )}
            onEdit={() => openEditor(mode)}
            onDuplicate={() => handleDuplicate(mode.id)}
            onDelete={() => handleDelete(mode.id)}
            onSetDefault={() => handleSetDefault(mode.id)}
          />
        ))}
      </div>

      <div className="flex items-center gap-3">
        <Button variant="outline" size="sm" onClick={handleAddMode}>
          + Add mode
        </Button>
        <Button
          variant="ghost"
          size="sm"
          className="text-muted-foreground"
          onClick={() => navigate("/hotkeys")}
        >
          Manage hotkeys →
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
