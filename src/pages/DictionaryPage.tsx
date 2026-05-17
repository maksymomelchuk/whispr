import { zodResolver } from "@hookform/resolvers/zod";
import { Plus } from "@phosphor-icons/react";
import { useState } from "react";
import { useFieldArray, useForm } from "react-hook-form";
import { toast } from "sonner";
import * as z from "zod";

import { SectionHeader } from "@/components/SectionHeader";
import { Button } from "@/components/ui/button";
import { Form, FormControl, FormField, FormItem } from "@/components/ui/form";
import { Input } from "@/components/ui/input";

import { useSettings } from "../context/SettingsContext";
import { setDictionary as persistDictionary } from "../lib/api";
import type { DictionaryEntry } from "../lib/types";

const dictionarySchema = z.object({
  rows: z.array(z.object({ from: z.string(), to: z.string() })),
});

type DictionaryFormValues = z.infer<typeof dictionarySchema>;

export function DictionaryPage() {
  const { settings, setSettings } = useSettings();
  const dictionary = settings?.dictionary ?? [];

  const form = useForm<DictionaryFormValues>({
    resolver: zodResolver(dictionarySchema),
    values: { rows: dictionary },
  });

  const { fields, append, remove } = useFieldArray({
    control: form.control,
    name: "rows",
  });

  const [saving, setSaving] = useState(false);

  if (!settings) return null;

  const onSubmit = async (values: DictionaryFormValues) => {
    const cleaned = values.rows
      .map((r) => ({ from: r.from.trim(), to: r.to }))
      .filter((r) => r.from.length > 0);
    setSaving(true);
    try {
      await persistDictionary(cleaned);
      form.reset({ rows: cleaned });
      setSettings((s) =>
        s ? { ...s, dictionary: cleaned as DictionaryEntry[] } : s,
      );
    } catch (e) {
      toast.error("Couldn't save dictionary", { description: String(e) });
    } finally {
      setSaving(false);
    }
  };

  const entryCount = fields.length;
  const dirty = form.formState.isDirty;

  return (
    <div className="p-6 flex flex-col gap-4">
      <SectionHeader
        title="Dictionary"
        trailing={
          entryCount > 0
            ? `${entryCount} ${entryCount === 1 ? "entry" : "entries"}`
            : undefined
        }
      />
      <p className="-mt-1 text-[12px] text-muted-foreground/85 max-w-prose">
        Spoken words on the left become the text on the right. Whole-word,
        case-insensitive. Punctuation like{" "}
        <code className="font-mono text-[11.5px]">. / -</code> is spaced
        intelligently — saying <em>“test dot ts”</em> produces{" "}
        <code className="font-mono text-[11.5px]">test.ts</code>. Entries also
        bias the transcriber, so recognition improves on the words you add.
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
