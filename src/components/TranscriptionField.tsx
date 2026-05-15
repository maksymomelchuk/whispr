import { useEffect, useState } from "react";
import { useForm } from "react-hook-form";

import { Button } from "@/components/ui/button";
import {
  Form,
  FormControl,
  FormField,
  FormItem,
  FormLabel,
} from "@/components/ui/form";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";

import { setDeepgramSettings as persistDeepgramSettings } from "../lib/api";
import type { DeepgramSettings } from "../lib/types";
import { CollapsibleCard } from "./CollapsibleCard";
import { InfoTip } from "./InfoTip";

interface Props {
  initial: DeepgramSettings;
  onSaved: (deepgram: DeepgramSettings) => void;
  defaultOpen?: boolean;
}

type BoolKey = {
  [K in keyof DeepgramSettings]: DeepgramSettings[K] extends boolean
    ? K
    : never;
}[keyof DeepgramSettings];

interface BoolOption {
  key: BoolKey;
  label: string;
  param: string;
  description: string;
}

const BOOL_OPTIONS: BoolOption[] = [
  {
    key: "smart_format",
    label: "Smart Format",
    param: "smart_format",
    description:
      "Improves readability by applying additional formatting. When enabled, punctuation and paragraph breaks will be applied as well as formatting of other entities, such as dates, times, and numbers.",
  },
  {
    key: "dictation",
    label: "Dictation",
    param: "dictation",
    description:
      "Automatically formats spoken commands for punctuation into their respective punctuation marks.",
  },
  {
    key: "numerals",
    label: "Numerals",
    param: "numerals",
    description:
      'Converts numbers from written format to numerical format (e.g., "nine hundred" becomes "900").',
  },
];

export function TranscriptionField({
  initial,
  onSaved,
  defaultOpen = false,
}: Props) {
  const form = useForm<DeepgramSettings>({ values: initial });

  const [saving, setSaving] = useState(false);
  const [savedOk, setSavedOk] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);

  useEffect(() => {
    if (!savedOk) return;
    const t = setTimeout(() => setSavedOk(false), 1500);
    return () => clearTimeout(t);
  }, [savedOk]);

  const keyterms = form.watch("keyterms");

  const updateKeyterm = (index: number, value: string) =>
    form.setValue(
      "keyterms",
      keyterms.map((k, i) => (i === index ? value : k)),
      { shouldDirty: true },
    );

  const removeKeyterm = (index: number) =>
    form.setValue(
      "keyterms",
      keyterms.filter((_, i) => i !== index),
      { shouldDirty: true },
    );

  const addKeyterm = () =>
    form.setValue("keyterms", [...keyterms, ""], { shouldDirty: true });

  const onSubmit = async (values: DeepgramSettings) => {
    const cleaned: DeepgramSettings = {
      ...values,
      language: values.language.trim() || "en",
      keyterms: values.keyterms.map((k) => k.trim()).filter((k) => k.length > 0),
    };
    setSaving(true);
    setSaveError(null);
    try {
      await persistDeepgramSettings(cleaned);
      onSaved(cleaned);
      setSavedOk(true);
    } catch (e) {
      setSaveError(String(e));
    } finally {
      setSaving(false);
    }
  };

  return (
    <CollapsibleCard
      title="Deepgram"
      defaultOpen={defaultOpen}
      dirty={form.formState.isDirty}
      info="Deepgram nova-3 options. Defaults are off; enable what you need."
    >
      <Form {...form}>
        <form onSubmit={form.handleSubmit(onSubmit)}>
          <FormField
            control={form.control}
            name="language"
            render={({ field }) => (
              <FormItem className="field-group">
                <div className="label-with-info" style={{ marginBottom: 4 }}>
                  <FormLabel className="field-label" style={{ margin: 0 }}>
                    Language
                  </FormLabel>
                  <InfoTip text="Language code (e.g. en, multi, es, de)." />
                </div>
                <FormControl>
                  <Input
                    {...field}
                    placeholder="en"
                    spellCheck={false}
                    autoComplete="off"
                  />
                </FormControl>
              </FormItem>
            )}
          />

          <div className="options-list">
            {BOOL_OPTIONS.map((opt) => (
              <FormField
                key={opt.key}
                control={form.control}
                name={opt.key}
                render={({ field }) => (
                  <FormItem>
                    <label
                      className="option-row"
                      htmlFor={`switch-${opt.key}`}
                    >
                      <div className="option-text">
                        <div className="option-label label-with-info">
                          {opt.label}
                          <InfoTip text={opt.description} />
                        </div>
                        <div className="option-param mono">
                          {opt.param}={String(field.value)}
                        </div>
                      </div>
                      <FormControl>
                        <Switch
                          id={`switch-${opt.key}`}
                          checked={field.value as boolean}
                          onCheckedChange={field.onChange}
                        />
                      </FormControl>
                    </label>
                  </FormItem>
                )}
              />
            ))}

            <div className="option-row keyterms-option">
              <div className="option-text">
                <div className="option-label label-with-info">
                  Keyterm Prompting
                  <InfoTip text="Boosts recognition of important words or phrases, like names, product terms, or jargon. Up to 100 keyterms per request." />
                </div>
                <div className="option-param mono">keyterm=TERM_OR_PHRASE</div>
                <div className="keyterms-list">
                  {keyterms.map((kt, i) => (
                    <div key={i} className="replacement-row">
                      <Input
                        className="flex-1 min-w-0"
                        value={kt}
                        placeholder="e.g. Deepgram"
                        spellCheck={false}
                        autoComplete="off"
                        onChange={(e) => updateKeyterm(i, e.target.value)}
                      />
                      <Button
                        type="button"
                        variant="ghost"
                        size="icon-sm"
                        aria-label="Remove"
                        onClick={() => removeKeyterm(i)}
                      >
                        ×
                      </Button>
                    </div>
                  ))}
                </div>
                <div className="row">
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    onClick={addKeyterm}
                  >
                    + Add keyterm
                  </Button>
                </div>
              </div>
            </div>
          </div>

          <div className="row replacements-actions save-row">
            <div className="spacer" />
            <Button type="submit" disabled={!form.formState.isDirty || saving}>
              {saving ? "Saving…" : "Save"}
            </Button>
          </div>
          {savedOk && <div className="status ok">Saved</div>}
          {saveError && <div className="status err">{saveError}</div>}
        </form>
      </Form>
    </CollapsibleCard>
  );
}
