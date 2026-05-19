import {
  CaretRightIcon,
  CopyIcon,
  PencilSimpleIcon,
  StarIcon,
  TrashIcon,
  WarningCircleIcon,
} from "@phosphor-icons/react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useNavigate } from "react-router-dom";
import { toast } from "sonner";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
import { Input } from "@/components/ui/input";
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
  Sheet,
  SheetContent,
  SheetFooter,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import { Textarea } from "@/components/ui/textarea";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";

import { Chip } from "../components/Chip";
import { RowCard } from "../components/RowCard";
import { SectionHeader } from "../components/SectionHeader";
import { ToggleRow } from "../components/ToggleRow";
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
import type {
  AssemblyAiModel,
  GroqModel,
  HotkeyBinding,
  Mode,
  ModeLanguage,
  NamedCorrectionSet,
  NamedTermSet,
  ProviderModel,
} from "../lib/types";
import { providerModelLanguageCodes } from "../lib/types";

const PROVIDER_OPTIONS: { value: ProviderModel["provider"]; label: string }[] =
  [
    { value: "deepgram", label: "Deepgram" },
    { value: "groq", label: "Groq" },
    { value: "assembly_ai", label: "AssemblyAI" },
  ];

const GROQ_MODEL_OPTIONS: { value: GroqModel; label: string }[] = [
  { value: "whisper_large_v3_turbo", label: "Whisper Large v3-turbo" },
  { value: "whisper_large_v3", label: "Whisper Large v3" },
];

const ASSEMBLYAI_MODEL_OPTIONS: { value: AssemblyAiModel; label: string }[] = [
  { value: "universal_pro_streaming", label: "Universal-3 Pro" },
  { value: "universal_streaming_english", label: "Universal English" },
  {
    value: "universal_streaming_multilingual",
    label: "Universal Multilingual",
  },
  { value: "whisper_streaming", label: "Whisper Streaming" },
];

function defaultProviderModel(
  provider: ProviderModel["provider"],
): ProviderModel {
  if (provider === "groq")
    return { provider: "groq", model: "whisper_large_v3_turbo" };
  if (provider === "assembly_ai")
    return { provider: "assembly_ai", model: "universal_pro_streaming" };
  return { provider: "deepgram" };
}

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
  return (
    APPLE_TRANSLATE_LANGUAGES.find(([c]) => c === code)?.[1] ??
    code.toUpperCase()
  );
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
  missingProviderKey,
  onEdit,
  onDuplicate,
  onDelete,
  onSetDefault,
}: {
  mode: Mode;
  isDefault: boolean;
  isLast: boolean;
  bindings: HotkeyBinding[];
  missingProviderKey: boolean;
  onEdit: () => void;
  onDuplicate: () => void;
  onDelete: () => void;
  onSetDefault: () => void;
}) {
  const deleteDisabled = isLast || isDefault;
  let deleteTooltip: string | null = null;
  if (isLast) deleteTooltip = "Cannot delete the only remaining profile";
  else if (isDefault) deleteTooltip = "Set a different default before deleting";

  return (
    <RowCard>
      <div className="flex flex-1 min-w-0 flex-col gap-0.5">
        <div className="flex items-center gap-2 flex-wrap">
          <span className="text-sm font-semibold text-foreground">
            {mode.name}
          </span>
          {isDefault && <Badge className="text-[10px]">Default</Badge>}
          {mode.ai_cleanup.enabled && (
            <Badge variant="neutral" className="text-[10px]">
              Cleanup
            </Badge>
          )}
          {missingProviderKey && (
            <Tooltip>
              <TooltipTrigger asChild>
                <span className="inline-flex items-center text-amber-500">
                  <WarningCircleIcon size={14} weight="fill" />
                </span>
              </TooltipTrigger>
              <TooltipContent>API key missing for this provider</TooltipContent>
            </Tooltip>
          )}
        </div>
        <span className="text-xs text-muted-foreground">
          {languageSummary(mode.language)}
          {mode.translate.kind !== "off" && <> · {translateSummary(mode)}</>}
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
                <StarIcon size={15} />
              </Button>
            </TooltipTrigger>
            <TooltipContent>Set as default</TooltipContent>
          </Tooltip>
        )}
        {isDefault && (
          <Button
            variant="ghost"
            size="icon-sm"
            aria-label="Default profile"
            disabled
            className="opacity-100 text-primary"
          >
            <StarIcon size={15} weight="fill" />
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
              <PencilSimpleIcon size={15} />
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
              <CopyIcon size={15} />
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
                <TrashIcon size={15} />
              </Button>
            </span>
          </TooltipTrigger>
          {deleteTooltip && <TooltipContent>{deleteTooltip}</TooltipContent>}
        </Tooltip>
      </div>
    </RowCard>
  );
}

function SetMultiSelect({
  label,
  emptyHint,
  addPlaceholder,
  available,
  selectedIds,
  onAdd,
  onRemove,
}: {
  label: string;
  emptyHint: string;
  addPlaceholder: string;
  available: { id: string; name: string }[];
  selectedIds: string[];
  onAdd: (id: string) => void;
  onRemove: (id: string) => void;
}) {
  const byId = new Map(available.map((s) => [s.id, s]));
  const selected = selectedIds
    .map((id) => byId.get(id))
    .filter((s): s is { id: string; name: string } => !!s);
  const unselected = available.filter((s) => !selectedIds.includes(s.id));

  return (
    <div className="flex flex-col gap-1.5">
      <span className="text-[13px] text-foreground">{label}</span>
      {available.length === 0 ? (
        <p className="text-xs text-muted-foreground">{emptyHint}</p>
      ) : (
        <div className="min-h-[40px] flex flex-wrap gap-1 items-center p-2 rounded-lg bg-card border border-border shadow-xs">
          {selected.map((s) => (
            <Chip key={s.id} label={s.name} onRemove={() => onRemove(s.id)} />
          ))}
          {unselected.length > 0 && (
            <Select value="" onValueChange={(id) => id && onAdd(id)}>
              <SelectTrigger
                size="sm"
                className="h-7 w-auto border-0 shadow-none bg-transparent px-2 text-muted-foreground hover:text-foreground dark:bg-transparent dark:hover:bg-transparent"
              >
                <SelectValue placeholder={addPlaceholder} />
              </SelectTrigger>
              <SelectContent>
                {unselected.map((s) => (
                  <SelectItem key={s.id} value={s.id}>
                    {s.name}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          )}
        </div>
      )}
    </div>
  );
}

export function ModeEditor({
  mode,
  isNew,
  onClose,
  onPersist,
  availableTermSets = [],
  correctionSets = [],
  cleanupCredentialConfigured = true,
}: {
  mode: Mode;
  isNew: boolean;
  onClose: () => void;
  onPersist: (mode: Mode, wasNew: boolean) => void;
  availableTermSets?: NamedTermSet[];
  correctionSets?: NamedCorrectionSet[];
  cleanupCredentialConfigured?: boolean;
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
    if (!restrictCodes.includes(code))
      setRestrictCodes([...restrictCodes, code]);
  };
  const removeLangCode = (code: string) => {
    const updated = restrictCodes.filter((c) => c !== code);
    setRestrictCodes(updated);
    if (updated.length === 0) setLangMode("auto");
  };

  const allowedLanguageCodes = providerModelLanguageCodes(draft.provider_model);

  const setProviderModel = (pm: ProviderModel) =>
    setDraft((d) => ({ ...d, provider_model: pm }));

  const setProvider = (provider: ProviderModel["provider"]) =>
    setProviderModel(defaultProviderModel(provider));

  const setGroqModel = (model: GroqModel) =>
    setProviderModel({ provider: "groq", model });

  const setAssemblyAiModel = (model: AssemblyAiModel) =>
    setProviderModel({ provider: "assembly_ai", model });

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
        value === "off" ? { kind: "off" } : { kind: "apple", target: value },
    }));
  const toggleTermSet = (id: string, checked: boolean) =>
    setDraft((d) => ({
      ...d,
      term_set_ids: checked
        ? [...d.term_set_ids, id]
        : d.term_set_ids.filter((tsid) => tsid !== id),
    }));
  const setUseSnippets = (use_snippets: boolean) =>
    setDraft((d) => ({ ...d, use_snippets }));
  const toggleCorrectionSet = (setId: string, on: boolean) =>
    setDraft((d) => ({
      ...d,
      correction_set_ids: on
        ? [...d.correction_set_ids, setId]
        : d.correction_set_ids.filter((id) => id !== setId),
    }));

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
          toast.error("Couldn't save profile", { description: String(e) });
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
          toast.error("Couldn't save profile", { description: String(e) });
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
      toast.error("Couldn't add profile", { description: String(e) });
    } finally {
      setCreating(false);
    }
  };

  return (
    <>
      <SheetHeader className="px-4 pt-4 pb-0">
        <SheetTitle>{isNew ? "New Profile" : "Edit Profile"}</SheetTitle>
      </SheetHeader>

      <div className="flex flex-col gap-4 px-4 pb-10 overflow-y-auto flex-1 min-h-0 [scrollbar-gutter:stable] [&::-webkit-scrollbar]:w-1.5 [&::-webkit-scrollbar-thumb]:rounded-full [&::-webkit-scrollbar-thumb]:bg-border [&::-webkit-scrollbar-thumb:hover]:bg-muted-foreground/40">
        <div className="flex flex-col gap-1.5">
          <Label htmlFor="mode-name" className="text-[13px]">
            Name
          </Label>
          <Input
            id="mode-name"
            value={draft.name}
            onChange={(e) => setName(e.target.value)}
            autoFocus
            placeholder="Profile name"
          />
        </div>

        <div className="flex flex-col gap-1.5">
          <Label className="text-[13px]">Provider</Label>
          <Select
            value={draft.provider_model.provider}
            onValueChange={(v) => setProvider(v as ProviderModel["provider"])}
          >
            <SelectTrigger size="sm" className="w-full">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {PROVIDER_OPTIONS.map((opt) => (
                <SelectItem key={opt.value} value={opt.value}>
                  {opt.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>

        {draft.provider_model.provider === "groq" && (
          <div className="flex flex-col gap-1.5">
            <Label className="text-[13px]">Model</Label>
            <Select
              value={draft.provider_model.model}
              onValueChange={(v) => setGroqModel(v as GroqModel)}
            >
              <SelectTrigger size="sm" className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {GROQ_MODEL_OPTIONS.map((opt) => (
                  <SelectItem key={opt.value} value={opt.value}>
                    {opt.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
        )}

        {draft.provider_model.provider === "assembly_ai" && (
          <div className="flex flex-col gap-1.5">
            <Label className="text-[13px]">Model</Label>
            <Select
              value={draft.provider_model.model}
              onValueChange={(v) => setAssemblyAiModel(v as AssemblyAiModel)}
            >
              <SelectTrigger size="sm" className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {ASSEMBLYAI_MODEL_OPTIONS.map((opt) => (
                  <SelectItem key={opt.value} value={opt.value}>
                    {opt.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
        )}

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
                  Let the engine detect the spoken language without
                  restrictions.
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
                  Improve detection by limiting it to one or more expected
                  languages.
                </span>
                {langMode === "restrict" && (
                  <div className="flex flex-wrap items-center gap-1.5 mt-1">
                    <Select
                      value=""
                      onValueChange={(v) => {
                        if (v) addLangCode(v);
                      }}
                    >
                      <SelectTrigger size="sm" className="w-auto">
                        <SelectValue placeholder="+ Add language" />
                      </SelectTrigger>
                      <SelectContent>
                        {LANGUAGES.filter((l) => {
                          if (restrictCodes.includes(l.code)) return false;
                          if (
                            allowedLanguageCodes !== null &&
                            !allowedLanguageCodes.includes(l.code)
                          )
                            return false;
                          return true;
                        }).map((l) => (
                          <SelectItem key={l.code} value={l.code}>
                            {l.flag} {l.name}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                    {restrictCodes.map((code) => (
                      <Chip
                        key={code}
                        label={langLabel(code)}
                        onRemove={() => removeLangCode(code)}
                      />
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
            value={
              draft.translate.kind === "apple" ? draft.translate.target : "off"
            }
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
            <p className="text-help text-muted-foreground">
              Engine: Apple Translate (on-device)
            </p>
          )}
        </div>

        <div className="flex flex-col gap-2 pt-1">
          <ToggleRow
            id="cleanup"
            label="AI cleanup"
            info={
              !cleanupCredentialConfigured && !draft.ai_cleanup.enabled
                ? "Set Anthropic credentials in Providers to enable cleanup."
                : undefined
            }
            checked={draft.ai_cleanup.enabled}
            onCheckedChange={setCleanup}
            disabled={
              !cleanupCredentialConfigured && !draft.ai_cleanup.enabled
            }
          />
          {draft.ai_cleanup.enabled && (
            <Collapsible
              open={promptOpen}
              onOpenChange={setPromptOpen}
              className="flex flex-col gap-2"
            >
              <CollapsibleTrigger className="flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground transition-colors w-fit">
                <CaretRightIcon
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
                <p className="text-help text-muted-foreground leading-relaxed">
                  The text inside{" "}
                  <code className="font-mono">&lt;transcript&gt;</code> tags
                  will be your dictation. Your prompt is responsible for
                  treating it as data to transform, not instructions to execute.
                </p>
              </CollapsibleContent>
            </Collapsible>
          )}
          <div className="flex items-center gap-3 mt-3 mb-1">
            <span className="text-eyebrow uppercase text-muted-foreground/70">
              Augment
            </span>
            <div className="flex-1 h-px bg-border/60" />
          </div>
          <SetMultiSelect
            label="Term sets"
            emptyHint="No term sets — create one in the Terms page."
            addPlaceholder="+ Add term set"
            available={availableTermSets}
            selectedIds={draft.term_set_ids}
            onAdd={(id) => toggleTermSet(id, true)}
            onRemove={(id) => toggleTermSet(id, false)}
          />
          <SetMultiSelect
            label="Correction sets"
            emptyHint="No correction sets — create one in the Corrections page."
            addPlaceholder="+ Add correction set"
            available={correctionSets}
            selectedIds={draft.correction_set_ids}
            onAdd={(id) => toggleCorrectionSet(id, true)}
            onRemove={(id) => toggleCorrectionSet(id, false)}
          />
          <div className="mt-2 pt-3 border-t border-border/60">
            <ToggleRow
              id="snippets"
              label="Use snippets"
              checked={draft.use_snippets}
              onCheckedChange={setUseSnippets}
            />
          </div>
        </div>
      </div>

      {isNew && (
        <SheetFooter>
          <Button
            onClick={handleCreate}
            disabled={creating || !draft.name.trim()}
            className="w-full"
          >
            {creating ? "Creating…" : "Create profile"}
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

  const openEditor = (mode: Mode, isNew = false) => setEditor({ mode, isNew });
  const closeEditor = () => setEditor(null);

  const handlePersist = useCallback(
    (saved: Mode, wasNew: boolean) => {
      setSettings((s) => {
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
      term_set_ids: [],
      correction_set_ids: [],
      use_snippets: true,
      provider_model: { provider: "deepgram" },
    };
    openEditor(newMode, true);
  };

  const handleDelete = async (id: string) => {
    try {
      await deleteMode(id);
      setSettings((s) => ({ ...s, modes: s.modes.filter((m) => m.id !== id) }));
    } catch (e) {
      console.error(e);
    }
  };

  const handleDuplicate = async (id: string) => {
    try {
      await duplicateMode(id);
      const updated = await getSettings();
      setSettings(() => updated);
    } catch (e) {
      console.error(e);
    }
  };

  const handleSetDefault = async (id: string) => {
    try {
      await setDefaultMode(id);
      setSettings((s) => ({ ...s, default_mode_id: id }));
    } catch (e) {
      console.error(e);
    }
  };

  return (
    <div className="p-6 flex flex-col gap-8">
      <SectionHeader title="Profiles" />
      <div className="flex flex-col gap-2">
        {settings.modes.map((mode) => {
          const provider = mode.provider_model.provider;
          const missingProviderKey =
            (provider === "deepgram" && !settings.deepgram_api_key_configured) ||
            (provider === "groq" && !settings.groq_api_key_configured) ||
            (provider === "assembly_ai" &&
              !settings.assemblyai_api_key_configured);
          return (
            <ModeRow
              key={mode.id}
              mode={mode}
              isDefault={mode.id === settings.default_mode_id}
              isLast={settings.modes.length === 1}
              bindings={settings.hotkey_bindings.filter(
                (b) => b.mode_id === mode.id,
              )}
              missingProviderKey={missingProviderKey}
              onEdit={() => openEditor(mode)}
              onDuplicate={() => handleDuplicate(mode.id)}
              onDelete={() => handleDelete(mode.id)}
              onSetDefault={() => handleSetDefault(mode.id)}
            />
          );
        })}
      </div>

      <div className="flex items-center gap-3">
        <Button variant="outline" size="sm" onClick={handleAddMode}>
          + Add profile
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

      <Sheet
        open={editor !== null}
        onOpenChange={(open) => !open && closeEditor()}
      >
        <SheetContent side="right" className="flex flex-col gap-4">
          {editor && (
            <ModeEditor
              key={editor.mode.id}
              mode={editor.mode}
              isNew={editor.isNew}
              onClose={closeEditor}
              onPersist={handlePersist}
              availableTermSets={settings.term_sets ?? []}
              correctionSets={settings.correction_sets ?? []}
              cleanupCredentialConfigured={
                settings.ai_cleanup_auth_mode === "api_key"
                  ? settings.ai_cleanup_key_configured
                  : settings.ai_cleanup_oauth_token_configured
              }
            />
          )}
        </SheetContent>
      </Sheet>
    </div>
  );
}
