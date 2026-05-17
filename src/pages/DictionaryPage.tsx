import { zodResolver } from "@hookform/resolvers/zod";
import { Plus } from "@phosphor-icons/react";
import { useState } from "react";
import { useFieldArray, useForm } from "react-hook-form";
import { toast } from "sonner";
import * as z from "zod";

import { SectionHeader } from "@/components/SectionHeader";
import { TermChipInput } from "@/components/TermChipInput";
import { Button } from "@/components/ui/button";
import { Form, FormControl, FormField, FormItem } from "@/components/ui/form";
import { Input } from "@/components/ui/input";

import { useSettings } from "../context/SettingsContext";
import { setCorrections as persistCorrections, setTerms as persistTerms } from "../lib/api";
import type { CorrectionEntry } from "../lib/types";

const correctionsSchema = z.object({
  rows: z.array(z.object({ from: z.string(), to: z.string() })),
});

type CorrectionsFormValues = z.infer<typeof correctionsSchema>;

type Tab = "terms" | "corrections";

export function DictionaryPage() {
  const { settings, setSettings } = useSettings();
  const [activeTab, setActiveTab] = useState<Tab>("terms");

  // Lift form state up so switching tabs doesn't discard unsaved changes.
  const [localTerms, setLocalTerms] = useState<string[]>(() => settings?.terms ?? []);
  const [termsDirty, setTermsDirty] = useState(false);

  if (!settings) return null;

  return (
    <div className="p-6 flex flex-col gap-4">
      <SectionHeader title="Dictionary" />

      {/* Segmented control */}
      <div className="flex gap-0 rounded-md border w-fit text-[12px] overflow-hidden">
        {(["terms", "corrections"] as Tab[]).map((tab) => (
          <button
            key={tab}
            type="button"
            onClick={() => setActiveTab(tab)}
            className={`px-4 py-1.5 capitalize transition-colors ${
              activeTab === tab
                ? "bg-secondary text-secondary-foreground font-medium"
                : "text-muted-foreground hover:text-foreground"
            }`}
          >
            {tab}
          </button>
        ))}
      </div>

      {activeTab === "terms" ? (
        <TermsTab
          terms={localTerms}
          dirty={termsDirty}
          onChange={(next) => { setLocalTerms(next); setTermsDirty(true); }}
          onSaved={(saved) => {
            setTermsDirty(false);
            setSettings((s) => (s ? { ...s, terms: saved } : s));
          }}
        />
      ) : (
        <CorrectionsTab
          settings={settings}
          setSettings={setSettings}
        />
      )}
    </div>
  );
}

function TermsTab({
  terms,
  dirty,
  onChange,
  onSaved,
}: {
  terms: string[];
  dirty: boolean;
  onChange: (next: string[]) => void;
  onSaved: (saved: string[]) => void;
}) {
  const [saving, setSaving] = useState(false);

  const handleSave = async () => {
    setSaving(true);
    try {
      await persistTerms(terms);
      onSaved(terms);
    } catch (e) {
      toast.error("Couldn't save terms", { description: String(e) });
    } finally {
      setSaving(false);
    }
  };

  const count = terms.length;

  return (
    <div className="flex flex-col gap-3">
      <p className="text-[12px] text-muted-foreground/85 max-w-prose">
        Words the recognizer should know exist. They bias the transcription
        engine so it picks your exact spelling — no replacement happens.
        {count > 0 && (
          <span className="ml-1 text-muted-foreground">
            ({count} {count === 1 ? "term" : "terms"})
          </span>
        )}
      </p>
      <TermChipInput value={terms} onChange={onChange} />
      <div className="flex justify-end">
        <Button size="sm" disabled={!dirty || saving} onClick={handleSave}>
          {saving ? "Saving…" : "Save"}
        </Button>
      </div>
    </div>
  );
}

function CorrectionsTab({
  settings,
  setSettings,
}: {
  settings: NonNullable<ReturnType<typeof useSettings>["settings"]>;
  setSettings: ReturnType<typeof useSettings>["setSettings"];
}) {
  const corrections = settings.corrections ?? [];

  const form = useForm<CorrectionsFormValues>({
    resolver: zodResolver(correctionsSchema),
    values: { rows: corrections },
  });

  const { fields, append, remove } = useFieldArray({
    control: form.control,
    name: "rows",
  });

  const [saving, setSaving] = useState(false);

  const onSubmit = async (values: CorrectionsFormValues) => {
    const cleaned = values.rows
      .map((r) => ({ from: r.from.trim(), to: r.to }))
      .filter((r) => r.from.length > 0);
    setSaving(true);
    try {
      await persistCorrections(cleaned);
      form.reset({ rows: cleaned });
      setSettings((s) =>
        s ? { ...s, corrections: cleaned as CorrectionEntry[] } : s,
      );
    } catch (e) {
      toast.error("Couldn't save corrections", { description: String(e) });
    } finally {
      setSaving(false);
    }
  };

  const entryCount = fields.length;
  const dirty = form.formState.isDirty;

  return (
    <div className="flex flex-col gap-3">
      <p className="-mt-1 text-[12px] text-muted-foreground/85 max-w-prose">
        Post-transcription find-and-replace rules. Applied after cleanup and
        snippets. The spoken form on the left is never sent to the STT engine —
        only real vocabulary hints go there (see Terms).
        {entryCount > 0 && (
          <span className="ml-1 text-muted-foreground">
            ({entryCount} {entryCount === 1 ? "entry" : "entries"})
          </span>
        )}
      </p>

      <Form {...form}>
        <form
          onSubmit={form.handleSubmit(onSubmit)}
          className="flex flex-col gap-2"
        >
          <div className="flex flex-col gap-1.5">
            {fields.map(({ id }, i) => (
              <div key={id} className="flex items-center gap-1.5">
                <FormField
                  control={form.control}
                  name={`rows.${i}.from`}
                  render={({ field }) => (
                    <FormItem className="min-w-0 flex-1">
                      <FormControl>
                        <Input
                          {...field}
                          placeholder="spoken"
                          spellCheck={false}
                          autoComplete="off"
                        />
                      </FormControl>
                    </FormItem>
                  )}
                />
                <span className="select-none text-xs text-muted-foreground/70">
                  →
                </span>
                <FormField
                  control={form.control}
                  name={`rows.${i}.to`}
                  render={({ field }) => (
                    <FormItem className="min-w-0 flex-1">
                      <FormControl>
                        <Input
                          {...field}
                          placeholder="text"
                          spellCheck={false}
                          autoComplete="off"
                        />
                      </FormControl>
                    </FormItem>
                  )}
                />
                <Button
                  type="button"
                  variant="ghost"
                  size="icon-sm"
                  aria-label="Remove"
                  onClick={() => remove(i)}
                  className="text-muted-foreground/70 hover:text-destructive"
                >
                  ×
                </Button>
              </div>
            ))}
          </div>
          <div className="flex items-center">
            <Button
              type="button"
              variant="outline"
              size="sm"
              onClick={() => append({ from: "", to: "" })}
            >
              <Plus size={13} />
              Add
            </Button>
            <div className="flex-1" />
            <Button type="submit" size="sm" disabled={!dirty || saving}>
              {saving ? "Saving…" : "Save"}
            </Button>
          </div>
        </form>
      </Form>
    </div>
  );
}
