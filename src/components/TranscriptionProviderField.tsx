import { useEffect, useState } from "react";

import {
  setDeepgramApiKey as persistDeepgramApiKey,
  setGroqApiKey as persistGroqApiKey,
  setGroqSettings as persistGroqSettings,
  setTranscriptionProvider as persistProvider,
  validateDeepgramApiKey,
  validateGroqApiKey,
} from "../lib/api";
import type {
  DeepgramSettings,
  GroqModel,
  GroqSettings,
  TranscriptionProvider,
} from "../lib/types";
import { ApiKeyField } from "./ApiKeyField";
import { TranscriptionField } from "./TranscriptionField";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";

interface Props {
  provider: TranscriptionProvider;
  deepgram: DeepgramSettings;
  groq: GroqSettings;
  deepgramApiKeyConfigured: boolean;
  groqApiKeyConfigured: boolean;
  onProviderChange: (provider: TranscriptionProvider) => void;
  onDeepgramSaved: (deepgram: DeepgramSettings) => void;
  onGroqSaved: (groq: GroqSettings) => void;
  onDeepgramApiKeyConfiguredChange: (configured: boolean) => void;
  onGroqApiKeyConfiguredChange: (configured: boolean) => void;
}

const PROVIDER_OPTIONS: { value: TranscriptionProvider; label: string }[] = [
  { value: "deepgram", label: "Deepgram" },
  { value: "groq", label: "Groq" },
];

const GROQ_MODEL_OPTIONS: { value: GroqModel; label: string }[] = [
  { value: "whisper_large_v3_turbo", label: "Whisper Large v3-turbo" },
  { value: "whisper_large_v3", label: "Whisper Large v3" },
];

export function TranscriptionProviderField({
  provider,
  deepgram,
  groq,
  deepgramApiKeyConfigured,
  groqApiKeyConfigured,
  onProviderChange,
  onDeepgramSaved,
  onGroqSaved,
  onDeepgramApiKeyConfiguredChange,
  onGroqApiKeyConfiguredChange,
}: Props) {
  const [providerSaveError, setProviderSaveError] = useState<string | null>(
    null,
  );

  const handleProviderChange = async (next: TranscriptionProvider) => {
    if (next === provider) return;
    const previous = provider;
    onProviderChange(next);
    setProviderSaveError(null);
    try {
      await persistProvider(next);
    } catch (e) {
      onProviderChange(previous);
      setProviderSaveError(String(e));
    }
  };

  return (
    <>
      <section className="card">
        <div className="field">
          <div className="label-with-info" style={{ marginBottom: 4 }}>
            <label
              className="field-label"
              style={{ margin: 0 }}
              htmlFor="transcription-provider"
            >
              Transcription provider
            </label>
            <Tooltip>
              <TooltipTrigger asChild>
                <span
                  className="inline-flex items-center justify-center w-3.5 h-3.5 rounded-full border border-[var(--border-strong)] text-[10px] font-semibold leading-none text-[var(--text-tertiary)] bg-[var(--bg-elevated)] cursor-help select-none outline-none"
                  aria-label="Provider used for the next dictation. Each provider keeps its own API key and options."
                  tabIndex={0}
                  onClick={(e) => { e.preventDefault(); e.stopPropagation(); }}
                  onMouseDown={(e) => e.preventDefault()}
                >
                  ?
                </span>
              </TooltipTrigger>
              <TooltipContent>Provider used for the next dictation. Each provider keeps its own API key and options.</TooltipContent>
            </Tooltip>
          </div>
          <select
            id="transcription-provider"
            className="mic-select"
            value={provider}
            onChange={(e) =>
              handleProviderChange(e.target.value as TranscriptionProvider)
            }
          >
            {PROVIDER_OPTIONS.map((opt) => (
              <option key={opt.value} value={opt.value}>
                {opt.label}
              </option>
            ))}
          </select>
        </div>
        {providerSaveError && (
          <div className="status err">{providerSaveError}</div>
        )}
      </section>

      {provider === "deepgram" && (
        <>
          <ApiKeyField
            title="Deepgram API Key"
            info="Required to transcribe audio with Deepgram. Paste your key from console.deepgram.com."
            placeholder="dg_..."
            isConfigured={deepgramApiKeyConfigured}
            persist={persistDeepgramApiKey}
            validate={validateDeepgramApiKey}
            onSaved={onDeepgramApiKeyConfiguredChange}
          />
          <TranscriptionField
            initial={deepgram}
            defaultOpen
            onSaved={onDeepgramSaved}
          />
        </>
      )}

      {provider === "groq" && (
        <>
          <ApiKeyField
            title="Groq API Key"
            info="Required to transcribe audio with Groq. Create one at console.groq.com."
            placeholder="gsk_..."
            isConfigured={groqApiKeyConfigured}
            persist={persistGroqApiKey}
            validate={validateGroqApiKey}
            onSaved={onGroqApiKeyConfiguredChange}
          />
          <GroqOptions initial={groq} onSaved={onGroqSaved} />
        </>
      )}
    </>
  );
}

interface GroqOptionsProps {
  initial: GroqSettings;
  onSaved: (groq: GroqSettings) => void;
}

type SaveStatus = "idle" | "saving" | "saved" | "error";

function GroqOptions({ initial, onSaved }: GroqOptionsProps) {
  const [state, setState] = useState<GroqSettings>(initial);
  const [status, setStatus] = useState<SaveStatus>("idle");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setState(initial);
  }, [initial]);

  useEffect(() => {
    if (status !== "saved") return;
    const t = setTimeout(() => setStatus("idle"), 1500);
    return () => clearTimeout(t);
  }, [status]);

  const dirty =
    state.model !== initial.model || state.language !== initial.language;

  const handleSave = async () => {
    const cleaned: GroqSettings = {
      ...state,
      language: state.language.trim() || "en",
    };
    setStatus("saving");
    setError(null);
    try {
      await persistGroqSettings(cleaned);
      setState(cleaned);
      onSaved(cleaned);
      setStatus("saved");
    } catch (e) {
      setStatus("error");
      setError(String(e));
    }
  };

  return (
    <section className="card">
      <div className="card-title-row">
        <h2 style={{ margin: 0 }}>Groq</h2>
        <Tooltip>
          <TooltipTrigger asChild>
            <span
              className="inline-flex items-center justify-center w-3.5 h-3.5 rounded-full border border-[var(--border-strong)] text-[10px] font-semibold leading-none text-[var(--text-tertiary)] bg-[var(--bg-elevated)] cursor-help select-none outline-none"
              aria-label="Whisper Large via Groq's transcription endpoint."
              tabIndex={0}
              onClick={(e) => { e.preventDefault(); e.stopPropagation(); }}
              onMouseDown={(e) => e.preventDefault()}
            >
              ?
            </span>
          </TooltipTrigger>
          <TooltipContent>Whisper Large via Groq's transcription endpoint.</TooltipContent>
        </Tooltip>
      </div>
      <div className="field-group">
        <div className="label-with-info" style={{ marginBottom: 4 }}>
          <label
            className="field-label"
            style={{ margin: 0 }}
            htmlFor="groq-model"
          >
            Model
          </label>
          <Tooltip>
            <TooltipTrigger asChild>
              <span
                className="inline-flex items-center justify-center w-3.5 h-3.5 rounded-full border border-[var(--border-strong)] text-[10px] font-semibold leading-none text-[var(--text-tertiary)] bg-[var(--bg-elevated)] cursor-help select-none outline-none"
                aria-label="v3-turbo is the cheapest and fastest. v3 is slightly more accurate."
                tabIndex={0}
                onClick={(e) => { e.preventDefault(); e.stopPropagation(); }}
                onMouseDown={(e) => e.preventDefault()}
              >
                ?
              </span>
            </TooltipTrigger>
            <TooltipContent>v3-turbo is the cheapest and fastest. v3 is slightly more accurate.</TooltipContent>
          </Tooltip>
        </div>
        <select
          id="groq-model"
          className="mic-select"
          value={state.model}
          onChange={(e) =>
            setState((s) => ({ ...s, model: e.target.value as GroqModel }))
          }
        >
          {GROQ_MODEL_OPTIONS.map((opt) => (
            <option key={opt.value} value={opt.value}>
              {opt.label}
            </option>
          ))}
        </select>
      </div>
      <div className="field-group">
        <div className="label-with-info" style={{ marginBottom: 4 }}>
          <label
            className="field-label"
            style={{ margin: 0 }}
            htmlFor="groq-language"
          >
            Language
          </label>
          <Tooltip>
            <TooltipTrigger asChild>
              <span
                className="inline-flex items-center justify-center w-3.5 h-3.5 rounded-full border border-[var(--border-strong)] text-[10px] font-semibold leading-none text-[var(--text-tertiary)] bg-[var(--bg-elevated)] cursor-help select-none outline-none"
                aria-label="ISO-639-1 language code (e.g. en, fr, es). Defaults to en."
                tabIndex={0}
                onClick={(e) => { e.preventDefault(); e.stopPropagation(); }}
                onMouseDown={(e) => e.preventDefault()}
              >
                ?
              </span>
            </TooltipTrigger>
            <TooltipContent>ISO-639-1 language code (e.g. en, fr, es). Defaults to en.</TooltipContent>
          </Tooltip>
        </div>
        <input
          id="groq-language"
          type="text"
          value={state.language}
          placeholder="en"
          spellCheck={false}
          autoComplete="off"
          onChange={(e) =>
            setState((s) => ({ ...s, language: e.target.value }))
          }
        />
      </div>
      <div className="row replacements-actions save-row">
        <div className="spacer" />
        <button
          className="primary"
          onClick={handleSave}
          disabled={!dirty || status === "saving"}
        >
          {status === "saving" ? "Saving…" : "Save"}
        </button>
      </div>
      {status === "saved" && <div className="status ok">Saved</div>}
      {status === "error" && <div className="status err">{error}</div>}
    </section>
  );
}
