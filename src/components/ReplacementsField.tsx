import { useEffect, useState } from "react";

import { zodResolver } from "@hookform/resolvers/zod";
import { useFieldArray, useForm } from "react-hook-form";
import * as z from "zod";

import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import {
  Form,
  FormControl,
  FormField,
  FormItem,
} from "@/components/ui/form";
import { Input } from "@/components/ui/input";

import { setReplacements as persistReplacements } from "../lib/api";
import type { Replacement } from "../lib/types";
import { CollapsibleCard } from "./CollapsibleCard";

const replacementsSchema = z.object({
  rows: z.array(z.object({ from: z.string(), to: z.string() })),
});

type ReplacementsFormValues = z.infer<typeof replacementsSchema>;

interface Props {
  initial: Replacement[];
  onSaved: (replacements: Replacement[]) => void;
  defaultOpen?: boolean;
}

export function ReplacementsField({
  initial,
  onSaved,
  defaultOpen = true,
}: Props) {
  const form = useForm<ReplacementsFormValues>({
    resolver: zodResolver(replacementsSchema),
    values: { rows: initial },
  });

  const { fields, append, remove } = useFieldArray({
    control: form.control,
    name: "rows",
  });

  const [saving, setSaving] = useState(false);
  const [savedOk, setSavedOk] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);

  useEffect(() => {
    if (!savedOk) return;
    const t = setTimeout(() => setSavedOk(false), 1500);
    return () => clearTimeout(t);
  }, [savedOk]);

  const onSubmit = async (values: ReplacementsFormValues) => {
    const cleaned = values.rows
      .map((r) => ({ from: r.from.trim(), to: r.to }))
      .filter((r) => r.from.length > 0);
    setSaving(true);
    setSaveError(null);
    try {
      await persistReplacements(cleaned);
      form.reset({ rows: cleaned });
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
      title="Voice Replacements"
      defaultOpen={defaultOpen}
      dirty={form.formState.isDirty}
      info='Spoken words on the left become the text on the right. Punctuation like ". / -" is spaced intelligently — saying "test dot ts" produces "test.ts".'
    >
      <Form {...form}>
        <form onSubmit={form.handleSubmit(onSubmit)}>
          <div className="mb-2.5 flex flex-col gap-1.5">
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
                <span className="select-none text-xs text-[var(--text-tertiary)]">
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
                  className="text-[var(--text-tertiary)] hover:text-[var(--danger)]"
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
              + Add
            </Button>
            <div className="flex-1" />
            <Button
              type="submit"
              size="sm"
              disabled={!form.formState.isDirty || saving}
            >
              {saving ? "Saving…" : "Save"}
            </Button>
          </div>
          {savedOk && (
            <Alert variant="success" className="mt-2">
              <AlertDescription>Saved</AlertDescription>
            </Alert>
          )}
          {saveError && (
            <Alert variant="destructive" className="mt-2">
              <AlertDescription>{saveError}</AlertDescription>
            </Alert>
          )}
        </form>
      </Form>
    </CollapsibleCard>
  );
}
