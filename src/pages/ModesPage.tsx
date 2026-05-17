import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useNavigate } from "react-router-dom";
import {
  CaretRight,
  Copy,
  PencilSimple,
  Star,
  Trash,
  X,
} from "@phosphor-icons/react";
import { toast } from "sonner";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Label } from "@/components/ui/label";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
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

const LANGUAGES: { code: string; name: string; flag: string }[] = [
  { code: "en", name: "English", flag: "🇺🇸" },
  { code: "uk", name: "Ukrainian", flag: "🇺🇦" },
  { code: "fr", name: "French", flag: "🇫🇷" },
  { code: "de", name: "German", flag: "🇩🇪" },
  { code: "es", name: "Spanish", flag: "🇪🇸" },
  { code: "it", name: "Italian", flag: "🇮🇹" },
  { code: "pt", name: "Portuguese", flag: "🇵🇹" },
  { code: "ru", name: "Russian", flag: "🇷🇺" },
  { code: "zh", name: "Chinese", flag: "🇨🇳" },
  { code: "ja", name: "Japanese", flag: "🇯🇵" },
  { code: "ko", name: "Korean", flag: "🇰🇷" },
  { code: "ar", name: "Arabic", flag: "🇸🇦" },
  { code: "pl", name: "Polish", flag: "🇵🇱" },
  { code: "nl", name: "Dutch", flag: "🇳🇱" },
  { code: "tr", name: "Turkish", flag: "🇹🇷" },
  { code: "sv", name: "Swedish", flag: "🇸🇪" },
  { code: "da", name: "Danish", flag: "🇩🇰" },
  { code: "fi", name: "Finnish", flag: "🇫🇮" },
  { code: "nb", name: "Norwegian", flag: "🇳🇴" },
  { code: "cs", name: "Czech", flag: "🇨🇿" },
  { code: "hu", name: "Hungarian", flag: "🇭🇺" },
  { code: "ro", name: "Romanian", flag: "🇷🇴" },
  { code: "hi", name: "Hindi", flag: "🇮🇳" },
  { code: "vi", name: "Vietnamese", flag: "🇻🇳" },
  { code: "th", name: "Thai", flag: "🇹🇭" },
  { code: "id", name: "Indonesian", flag: "🇮🇩" },
  { code: "he", name: "Hebrew", flag: "🇮🇱" },
];

function langLabel(code: string): string {
  const entry = LANGUAGES.find((l) => l.code === code);
  return entry ? `${entry.flag} ${entry.name}` : code.toUpperCase();
}

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
  if (lang.kind === "exact") return lang.code.toUpperCase();
  return lang.codes.map((c) => c.toUpperCase()).join(", ");
}

function buildLanguage(
  langMode: "auto" | "restrict",
  codes: string[],
): ModeLanguage {
  if (langMode === "auto" || codes.length === 0) return { kind: "auto" };
  if (codes.length === 1) return { kind: "exact", code: codes[0] };
  return { kind: "hints", codes };
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

export function ModeEditor({
  mode,
  isNew,
  onClose,
  onPersist,
}: {
  mode: Mode;
  isNew: boolean;
  onClose: () => void;
  onPersist: (mode: Mode, wasNew: boolean) => void;
}) {
  const [draft, setDraft] = useState<Mode>(mode);
  const [creating, setCreating] = useState(false);
  const [promptOpen, setPromptOpen] = useState(
    !!mode.ai_cleanup.prompt_override,
  );

  // Language UI state — kept separate so chip list survives toggling to Auto.
  const [langMode, setLangMode] = useState<"auto" | "restrict">(
    mode.language.kind === "auto" ? "auto" : "restrict",
  );
  const [restrictCodes, setRestrictCodes] = useState<string[]>(() => {
    if (mode.language.kind === "exact") return [mode.language.code];
    if (mode.language.kind === "hints") return mode.language.codes;
    return ["en"];
  });

  const addLangCode = (code: string) => {
    if (!restrictCodes.includes(code)) setRestrictCodes([...restrictCodes, code]);
  };
  const removeLangCode = (code: string) => {
    const updated = restrictCodes.filter((c) => c !== code);
    setRestrictCodes(updated);
    if (updated.length === 0) setLangMode("auto");
  };

  const setName = (name: string) => setDraft((d) => ({ ...d, name }));
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
  const setUseTerms = (use_terms: boolean) =>
    setDraft((d) => ({ ...d, use_terms }));
  const setUseCorrections = (use_corrections: boolean) =>
    setDraft((d) => ({ ...d, use_corrections }));
  const setUseSnippets = (use_snippets: boolean) =>
    setDraft((d) => ({ ...d, use_snippets }));

  const normalized = useMemo<Mode>(
    () => ({
      ...draft,
      language: buildLanguage(langMode, restrictCodes),
      ai_cleanup: {
        ...draft.ai_cleanup,
        prompt_override: draft.ai_cleanup.prompt_override?.trim() || null,
      },
    }),
    [draft, langMode, restrictCodes],
  );

  const lastSavedRef = useRef<string>(JSON.stringify(normalized));
  const normalizedRef = useRef<Mode>(normalized);
  normalizedRef.current = normalized;
  const onPersistRef = useRef(onPersist);
  onPersistRef.current = onPersist;

  useEffect(() => {
    if (isNew) return;
    if (normalized.name.trim().length === 0) return;
    const serialized = JSON.stringify(normalized);
    if (serialized === lastSavedRef.current) return;

    const handle = setTimeout(() => {
      lastSavedRef.current = serialized;
      updateMode(normalized)
        .then(() => onPersistRef.current(normalized, false))
        .catch((e) => {
          toast.error("Couldn't save mode", { description: String(e) });
        });
    }, 450);

    return () => clearTimeout(handle);
  }, [normalized, isNew]);

  // Flush pending edits if the sheet closes inside the 450ms debounce window.
  useEffect(() => {
    if (isNew) return;
    return () => {
      const current = normalizedRef.current;
      if (current.name.trim().length === 0) return;
      const serialized = JSON.stringify(current);
      if (serialized === lastSavedRef.current) return;
      lastSavedRef.current = serialized;
      updateMode(current)
        .then(() => onPersistRef.current(current, false))
        .catch((e) => {
          toast.error("Couldn't save mode", { description: String(e) });
        });
    };
  }, [isNew]);

  const handleCreate = async () => {
    setCreating(true);
    try {
      await addMode(normalized);
      onPersistRef.current(normalized, true);
      onClose();
    } catch (e) {
      toast.error("Couldn't add mode", { description: String(e) });
    } finally {
      setCreating(false);
    }
  };

  return (
    <>
      <SheetHeader className="px-4 pt-4 pb-0">
        <SheetTitle className="text-[15px]">
          {isNew ? "New Mode" : "Edit Mode"}
        </SheetTitle>
      </SheetHeader>

      <div className="flex flex-col gap-4 px-4 pb-6 overflow-y-auto flex-1">
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

        <div className="flex flex-col gap-2">
          <Label className="text-[13px]">Spoken language</Label>
          <RadioGroup
            value={langMode}
            onValueChange={(v) => setLangMode(v as "auto" | "restrict")}
            className="gap-2"
          >
            <Label
              htmlFor={`lang-auto-${draft.id}`}
              className="flex items-start gap-2.5 cursor-pointer font-normal"
            >
              <RadioGroupItem
                id={`lang-auto-${draft.id}`}
                value="auto"
                className="mt-0.5"
              />
              <div className="flex flex-col gap-0.5">
                <span className="text-[13px]">Auto-detect all languages</span>
                <span className="text-xs text-muted-foreground">
                  Let the engine detect the spoken language without restrictions.
                </span>
              </div>
            </Label>
            <Label
              htmlFor={`lang-restrict-${draft.id}`}
              className="flex items-start gap-2.5 cursor-pointer font-normal"
            >
              <RadioGroupItem
                id={`lang-restrict-${draft.id}`}
                value="restrict"
                className="mt-0.5"
              />
              <div className="flex flex-col gap-1">
                <span className="text-[13px]">
                  Restrict detection to selected languages
                </span>
                <span className="text-xs text-muted-foreground">
                  Improve detection by limiting it to one or more expected languages.
                </span>
                {langMode === "restrict" && (
                  <div className="flex flex-wrap items-center gap-1.5 mt-1">
                    <Select
                      value=""
                      onValueChange={(v) => {
                        if (v) addLangCode(v);
                      }}
                    >
                      <SelectTrigger
                        size="sm"
                        className="w-auto rounded-full border-dashed px-2.5 py-1 h-auto text-xs font-medium text-muted-foreground shadow-none data-[placeholder]:text-muted-foreground"
                      >
                        <SelectValue placeholder="+ Add language" />
                      </SelectTrigger>
                      <SelectContent>
                        {LANGUAGES.filter(
                          (l) => !restrictCodes.includes(l.code),
                        ).map((l) => (
                          <SelectItem key={l.code} value={l.code}>
                            {l.flag} {l.name}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                    {restrictCodes.map((code) => (
                      <span
                        key={code}
                        className="inline-flex items-center gap-1 rounded-full bg-muted pl-2.5 pr-1.5 py-1 text-xs font-medium"
                      >
                        {langLabel(code)}
                        <button
                          type="button"
                          onClick={() => removeLangCode(code)}
                          className="inline-flex items-center justify-center rounded-full p-0.5 text-muted-foreground hover:text-foreground hover:bg-foreground/5 outline-none focus-visible:ring-[3px] focus-visible:ring-ring/50"
                          aria-label={`Remove ${langLabel(code)}`}
                        >
                          <X size={10} weight="bold" />
                        </button>
                      </span>
                    ))}
                  </div>
                )}
              </div>
            </Label>
          </RadioGroup>
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
              className="flex flex-col gap-2"
            >
              <CollapsibleTrigger className="flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground transition-colors w-fit">
                <CaretRight
                  size={10}
                  className={`transition-transform ${promptOpen ? "rotate-90" : ""}`}
                />
                Custom prompt
              </CollapsibleTrigger>
              <CollapsibleContent className="flex flex-col gap-2">
                <Textarea
                  className="resize-none min-h-[80px] leading-[1.5]"
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
          <div className="flex items-center gap-3 mt-3 mb-1">
            <span className="text-[10.5px] font-semibold uppercase tracking-[0.14em] text-muted-foreground/70">
              Augment
            </span>
            <div className="flex-1 h-px bg-border/60" />
          </div>
          <ToggleRow
            id="terms"
            label="Use terms"
            checked={draft.use_terms}
            onCheckedChange={setUseTerms}
          />
          <ToggleRow
            id="corrections"
            label="Use corrections"
            checked={draft.use_corrections}
            onCheckedChange={setUseCorrections}
          />
          <ToggleRow
            id="snippets"
            label="Use snippets"
            checked={draft.use_snippets}
            onCheckedChange={setUseSnippets}
          />
        </div>

      </div>

      {isNew && (
        <SheetFooter>
          <Button
            onClick={handleCreate}
            disabled={creating || !draft.name.trim()}
            className="w-full"
          >
            {creating ? "Creating…" : "Create mode"}
          </Button>
        </SheetFooter>
      )}
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

  const handlePersist = useCallback(
    (saved: Mode, wasNew: boolean) => {
      setSettings((s) => {
        if (!s) return s;
        if (wasNew) {
          return { ...s, modes: [...s.modes, saved] };
        }
        return {
          ...s,
          modes: s.modes.map((m) => (m.id === saved.id ? saved : m)),
        };
      });
    },
    [setSettings],
  );

  const handleAddMode = () => {
    const newMode: Mode = {
      id: `mode-${Date.now()}`,
      name: "",
      icon: null,
      language: { kind: "exact", code: "en" },
      translate: { kind: "off" },
      ai_cleanup: { enabled: false, prompt_override: null },
      use_terms: true,
      use_corrections: true,
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
        <SheetContent side="right" className="w-[440px] sm:max-w-[440px] flex flex-col gap-4">
          {editor && (
            <ModeEditor
              key={editor.mode.id}
              mode={editor.mode}
              isNew={editor.isNew}
              onClose={closeEditor}
              onPersist={handlePersist}
            />
          )}
        </SheetContent>
      </Sheet>
    </div>
  );
}
