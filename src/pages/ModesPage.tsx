import {
  CaretRightIcon,
  CopyIcon,
  PencilSimpleIcon,
  StarIcon,
  TrashIcon,
  WarningCircleIcon,
} from "@phosphor-icons/react";
import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Link, useNavigate } from "react-router-dom";
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
  getLocalModelStatuses,
  getSettings,
  setDefaultMode,
  updateMode,
} from "../lib/api";
import type {
  AiProviderId,
  AssemblyAiModel,
  GroqModel,
  HotkeyBinding,
  LocalModelStatus,
  LocalWhisperModel,
  Mode,
  ModeLanguage,
  NamedCorrectionSet,
  NamedTermSet,
  OpenAiTranscribeModel,
  ProviderModel,
} from "../lib/types";
import { providerModelLanguageCodes, pttModeId } from "../lib/types";

const PROVIDER_OPTIONS: { value: ProviderModel["provider"]; label: string }[] =
  [
    { value: "deepgram", label: "Deepgram" },
    { value: "groq", label: "Groq" },
    { value: "assembly_ai", label: "AssemblyAI" },
    { value: "open_ai", label: "OpenAI" },
    { value: "local", label: "Local" },
  ];

const LOCAL_MODEL_OPTIONS: { value: LocalWhisperModel; label: string }[] = [
  { value: "large_v3_turbo", label: "Whisper Large v3 Turbo" },
  { value: "large_v3", label: "Whisper Large v3" },
  { value: "parakeet", label: "Parakeet TDT" },
];

const GROQ_MODEL_OPTIONS: { value: GroqModel; label: string }[] = [
  { value: "whisper_large_v3_turbo", label: "Whisper Large v3-turbo" },
  { value: "whisper_large_v3", label: "Whisper Large v3" },
];

const OPENAI_TRANSCRIBE_MODEL_OPTIONS: {
  value: OpenAiTranscribeModel;
  label: string;
}[] = [
  { value: "gpt4o_transcribe", label: "GPT-4o Transcribe" },
  { value: "gpt4o_mini_transcribe", label: "GPT-4o mini Transcribe" },
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
  if (provider === "open_ai")
    return { provider: "open_ai", model: "gpt4o_transcribe" };
  if (provider === "local")
    return { provider: "local", model: "large_v3_turbo" };
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

const CLEANUP_PROVIDER_OPTIONS: { value: AiProviderId; label: string }[] = [
  { value: "anthropic", label: "Anthropic" },
  { value: "openai", label: "OpenAI" },
  { value: "google", label: "Google Gemini" },
  { value: "groq", label: "Groq" },
  { value: "deepseek", label: "DeepSeek" },
  { value: "cerebras", label: "Cerebras" },
  { value: "openrouter", label: "OpenRouter" },
  { value: "custom", label: "Custom" },
];

type CleanupModelOption = {
  value: string;
  label: string;
  recommended?: boolean;
};

const CLEANUP_MODEL_OPTIONS: Record<
  Exclude<AiProviderId, "custom">,
  CleanupModelOption[]
> = {
  anthropic: [
    { value: "claude-haiku-4-5", label: "Claude Haiku 4.5", recommended: true },
    { value: "claude-sonnet-4-6", label: "Claude Sonnet 4.6" },
    { value: "claude-opus-4-8", label: "Claude Opus 4.8" },
  ],
  openai: [
    { value: "gpt-5.4-mini", label: "GPT-5.4 mini", recommended: true },
    { value: "gpt-5.4-nano", label: "GPT-5.4 nano" },
    { value: "gpt-5-mini", label: "GPT-5 mini" },
    { value: "gpt-5-nano", label: "GPT-5 nano" },
    { value: "gpt-5.4", label: "GPT-5.4" },
    { value: "gpt-5.5", label: "GPT-5.5" },
  ],
  google: [
    { value: "gemini-2.5-flash", label: "Gemini 2.5 Flash", recommended: true },
    { value: "gemini-3.5-flash", label: "Gemini 3.5 Flash" },
    { value: "gemini-3.1-flash-lite", label: "Gemini 3.1 Flash-Lite" },
    { value: "gemini-2.5-pro", label: "Gemini 2.5 Pro" },
    { value: "gemini-3.1-pro-preview", label: "Gemini 3.1 Pro (preview)" },
  ],
  groq: [
    {
      value: "llama-3.1-8b-instant",
      label: "Llama 3.1 8B",
      recommended: true,
    },
    { value: "llama-3.3-70b-versatile", label: "Llama 3.3 70B" },
    { value: "openai/gpt-oss-20b", label: "GPT-OSS 20B" },
    { value: "openai/gpt-oss-120b", label: "GPT-OSS 120B" },
  ],
  deepseek: [
    {
      value: "deepseek-v4-flash",
      label: "DeepSeek V4 Flash",
      recommended: true,
    },
    { value: "deepseek-v4-pro", label: "DeepSeek V4 Pro" },
  ],
  cerebras: [
    { value: "llama-3.3-70b", label: "Llama 3.3 70B", recommended: true },
    { value: "llama3.1-8b", label: "Llama 3.1 8B" },
    { value: "gpt-oss-120b", label: "GPT-OSS 120B" },
    { value: "qwen-3-235b-a22b-instruct-2507", label: "Qwen 3 235B" },
  ],
  openrouter: [
    {
      value: "anthropic/claude-haiku-4.5",
      label: "Claude Haiku 4.5",
      recommended: true,
    },
    { value: "openai/gpt-5-mini", label: "GPT-5 mini" },
    { value: "openai/gpt-5-nano", label: "GPT-5 nano" },
    { value: "google/gemini-2.5-flash", label: "Gemini 2.5 Flash" },
    { value: "google/gemini-3.5-flash", label: "Gemini 3.5 Flash" },
    { value: "meta-llama/llama-3.3-70b-instruct", label: "Llama 3.3 70B" },
  ],
};

function langLabel(code: string): string {
  const entry = LANGUAGES.find((l) => l.code === code);
  return entry ? `${entry.flag} ${entry.name}` : code.toUpperCase();
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
  configuredProviders,
  customProviderModel = "",
}: {
  mode: Mode;
  isNew: boolean;
  onClose: () => void;
  onPersist: (mode: Mode, wasNew: boolean) => void;
  availableTermSets?: NamedTermSet[];
  correctionSets?: NamedCorrectionSet[];
  configuredProviders?: AiProviderId[];
  customProviderModel?: string;
}) {
  const [draft, setDraft] = useState<Mode>(mode);
  const [creating, setCreating] = useState(false);
  const [promptOpen, setPromptOpen] = useState(
    !!mode.ai_cleanup.prompt_override,
  );
  const [localStatuses, setLocalStatuses] = useState<LocalModelStatus[] | null>(
    null,
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

  const setOpenAiModel = (model: OpenAiTranscribeModel) =>
    setProviderModel({ provider: "open_ai", model });

  const setLocalModel = (model: LocalWhisperModel) =>
    setProviderModel({ provider: "local", model });

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

  const setCleanupProvider = (provider: AiProviderId) => {
    const defaultModel =
      provider === "custom" ? "" : CLEANUP_MODEL_OPTIONS[provider][0].value;
    setDraft((d) => ({
      ...d,
      ai_cleanup: { ...d.ai_cleanup, provider, model: defaultModel },
    }));
  };

  const setCleanupModel = (model: string) =>
    setDraft((d) => ({ ...d, ai_cleanup: { ...d.ai_cleanup, model } }));

  const cleanupProvider = draft.ai_cleanup.provider;
  const cleanupProviderConfigured = (configuredProviders ?? []).includes(
    cleanupProvider,
  );

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

  useEffect(() => {
    getLocalModelStatuses()
      .then(setLocalStatuses)
      .catch(() => setLocalStatuses([]));
  }, []);

  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;

    const attach = async () => {
      const fn = await listen<LocalWhisperModel>(
        "model-download-complete",
        (e) => {
          const model = e.payload;
          setLocalStatuses(
            (prev) =>
              prev?.map((s) =>
                s.model === model ? { ...s, downloaded: true } : s,
              ) ?? prev,
          );
        },
      );
      if (cancelled) {
        fn();
        return;
      }
      unlisten = fn;
    };

    attach();
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

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

      <div className="flex flex-col gap-4 px-4 pb-10 overflow-y-scroll flex-1 min-h-0 [&::-webkit-scrollbar]:w-1.5 [&::-webkit-scrollbar-thumb]:rounded-full [&::-webkit-scrollbar-thumb]:bg-border [&::-webkit-scrollbar-thumb:hover]:bg-muted-foreground/40">
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

        {draft.provider_model.provider === "open_ai" && (
          <div className="flex flex-col gap-1.5">
            <Label className="text-[13px]">Model</Label>
            <Select
              value={draft.provider_model.model}
              onValueChange={(v) => setOpenAiModel(v as OpenAiTranscribeModel)}
            >
              <SelectTrigger size="sm" className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {OPENAI_TRANSCRIBE_MODEL_OPTIONS.map((opt) => (
                  <SelectItem key={opt.value} value={opt.value}>
                    {opt.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
        )}

        {draft.provider_model.provider === "local" && (
          <div className="flex flex-col gap-1.5">
            <Label className="text-[13px]">Model</Label>
            <Select
              value={draft.provider_model.model}
              onValueChange={(v) => setLocalModel(v as LocalWhisperModel)}
            >
              <SelectTrigger size="sm" className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {LOCAL_MODEL_OPTIONS.map((opt) => {
                  const downloaded =
                    localStatuses?.find((s) => s.model === opt.value)
                      ?.downloaded ?? false;
                  return (
                    <SelectItem
                      key={opt.value}
                      value={opt.value}
                      disabled={!downloaded}
                    >
                      {opt.label}
                    </SelectItem>
                  );
                })}
              </SelectContent>
            </Select>
            {localStatuses !== null &&
              localStatuses.some((s) => !s.downloaded) && (
                <p className="text-help text-muted-foreground">
                  Download models in{" "}
                  <Link
                    to="/speech-models"
                    className="underline hover:text-foreground transition-colors"
                  >
                    Speech models
                  </Link>
                  .
                </p>
              )}
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

        <div className="flex flex-col gap-2 pt-1">
          <ToggleRow
            id="cleanup"
            label="AI cleanup"
            info={
              !cleanupProviderConfigured && !draft.ai_cleanup.enabled
                ? "Set up a provider in AI Providers to enable cleanup."
                : undefined
            }
            checked={draft.ai_cleanup.enabled}
            onCheckedChange={setCleanup}
            disabled={!cleanupProviderConfigured && !draft.ai_cleanup.enabled}
          />
          {draft.ai_cleanup.enabled && (
            <>
              <div className="flex flex-col gap-2">
                <div className="flex flex-col gap-1.5">
                  <span className="text-xs text-muted-foreground/70">
                    Provider
                  </span>
                  <Select
                    value={draft.ai_cleanup.provider}
                    onValueChange={(v) => setCleanupProvider(v as AiProviderId)}
                  >
                    <SelectTrigger size="sm">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      {CLEANUP_PROVIDER_OPTIONS.map((opt) => (
                        <SelectItem key={opt.value} value={opt.value}>
                          {opt.label}
                          {!(configuredProviders ?? []).includes(opt.value) &&
                            " (needs setup)"}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </div>
                {cleanupProvider !== "custom" ? (
                  <div className="flex flex-col gap-1.5">
                    <span className="text-xs text-muted-foreground/70">
                      Model
                    </span>
                    <Select
                      value={draft.ai_cleanup.model}
                      onValueChange={setCleanupModel}
                    >
                      <SelectTrigger size="sm">
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent>
                        {CLEANUP_MODEL_OPTIONS[cleanupProvider].map((opt) => (
                          <SelectItem key={opt.value} value={opt.value}>
                            {opt.label}
                            {opt.recommended && (
                              <span className="ml-1.5 text-muted-foreground/70">
                                (recommended)
                              </span>
                            )}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  </div>
                ) : (
                  <div className="flex flex-col gap-1.5">
                    <span className="text-xs text-muted-foreground/70">
                      Model
                    </span>
                    <p className="text-xs text-muted-foreground">
                      {customProviderModel || "(blank — single-model server)"}
                    </p>
                    <p className="text-xs text-muted-foreground/60">
                      Configured on the Custom card in AI Providers.
                    </p>
                  </div>
                )}
              </div>
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
                </CollapsibleContent>
              </Collapsible>
            </>
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
      ai_cleanup: {
        enabled: false,
        prompt_override: null,
        provider: "anthropic",
        model: "claude-haiku-4-5",
      },
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
            (provider === "deepgram" &&
              !settings.deepgram_api_key_configured) ||
            (provider === "groq" && !settings.groq_api_key_configured) ||
            (provider === "assembly_ai" &&
              !settings.assemblyai_api_key_configured) ||
            (provider === "open_ai" && !settings.openai_api_key_configured);
          return (
            <ModeRow
              key={mode.id}
              mode={mode}
              isDefault={mode.id === settings.default_mode_id}
              isLast={settings.modes.length === 1}
              bindings={settings.hotkey_bindings.filter(
                (b) => pttModeId(b) === mode.id,
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
              configuredProviders={(() => {
                const anthropicConfigured =
                  settings.ai_cleanup_key_configured ||
                  settings.ai_cleanup_oauth_token_configured;
                return [
                  ...settings.configured_providers,
                  ...(anthropicConfigured &&
                  !settings.configured_providers.includes("anthropic")
                    ? (["anthropic"] as AiProviderId[])
                    : []),
                  ...(settings.custom_provider_configured
                    ? (["custom"] as AiProviderId[])
                    : []),
                ];
              })()}
              customProviderModel={settings.custom_provider_model}
            />
          )}
        </SheetContent>
      </Sheet>
    </div>
  );
}
