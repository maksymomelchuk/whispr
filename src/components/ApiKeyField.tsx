import { useEffect, useState } from "react";

import { zodResolver } from "@hookform/resolvers/zod";
import { useForm } from "react-hook-form";
import * as z from "zod";

import { Button } from "@/components/ui/button";
import {
  Form,
  FormControl,
  FormField,
  FormItem,
  FormMessage,
} from "@/components/ui/form";
import { Input } from "@/components/ui/input";

import type { ApiKeyValidation } from "../lib/types";
import { InfoTip } from "./InfoTip";

const apiKeySchema = z.object({
  apiKey: z.string().min(1, "API key is required"),
});

type ApiKeyFormValues = z.infer<typeof apiKeySchema>;

interface Props {
  title: string;
  info: string;
  placeholder?: string;
  isConfigured: boolean;
  persist: (apiKey: string) => Promise<void>;
  validate?: (apiKey: string) => Promise<ApiKeyValidation>;
  onSaved: (configured: boolean) => void;
}

export function ApiKeyField({
  title,
  info,
  placeholder,
  isConfigured,
  persist,
  validate,
  onSaved,
}: Props) {
  const form = useForm<ApiKeyFormValues>({
    resolver: zodResolver(apiKeySchema),
    defaultValues: { apiKey: "" },
  });

  const [saving, setSaving] = useState(false);
  const [savedOk, setSavedOk] = useState(false);
  const [validating, setValidating] = useState(false);

  useEffect(() => {
    if (!savedOk) return;
    const t = setTimeout(() => setSavedOk(false), 1500);
    return () => clearTimeout(t);
  }, [savedOk]);

  const onSubmit = async (values: ApiKeyFormValues) => {
    setSaving(true);
    try {
      const trimmed = values.apiKey.trim();
      await persist(trimmed);
      form.reset();
      onSaved(trimmed.length > 0);
      setSavedOk(true);
    } catch (e) {
      form.setError("apiKey", { message: String(e) });
    } finally {
      setSaving(false);
    }
  };

  const handleClear = async () => {
    setSaving(true);
    form.clearErrors("apiKey");
    try {
      await persist("");
      form.reset();
      onSaved(false);
      setSavedOk(true);
    } catch (e) {
      form.setError("apiKey", { message: String(e) });
    } finally {
      setSaving(false);
    }
  };

  const handleBlur = async (value: string) => {
    if (!validate) return;
    const trimmed = value.trim();
    if (!trimmed) return;
    setValidating(true);
    form.clearErrors("apiKey");
    try {
      const result = await validate(trimmed);
      if (result.kind === "invalid") {
        form.setError("apiKey", { message: "Invalid API key" });
      } else if (result.kind === "error") {
        form.setError("apiKey", {
          message: `Could not validate key: ${result.message}`,
        });
      }
    } catch (e) {
      form.setError("apiKey", {
        message: `Could not validate key: ${String(e)}`,
      });
    } finally {
      setValidating(false);
    }
  };

  const inputPlaceholder = isConfigured
    ? "Enter new key to replace…"
    : (placeholder ?? "");

  return (
    <section className="card">
      <div className="card-title-row">
        <h2 style={{ margin: 0 }}>{title}</h2>
        <InfoTip text={info} />
        {isConfigured ? (
          <span className="status ok">Configured</span>
        ) : (
          <span className="status err">Not set</span>
        )}
      </div>
      <Form {...form}>
        <form onSubmit={form.handleSubmit(onSubmit)}>
          <FormField
            control={form.control}
            name="apiKey"
            render={({ field }) => (
              <FormItem>
                <FormControl>
                  <div className="row">
                    <Input
                      {...field}
                      type="password"
                      placeholder={inputPlaceholder}
                      spellCheck={false}
                      autoComplete="off"
                      onChange={(e) => {
                        field.onChange(e);
                        form.clearErrors("apiKey");
                      }}
                      onBlur={(e) => {
                        field.onBlur();
                        handleBlur(e.target.value);
                      }}
                    />
                    <Button
                      type="submit"
                      disabled={!form.formState.isDirty || saving}
                    >
                      {saving ? "Saving…" : "Save"}
                    </Button>
                    {isConfigured && (
                      <Button
                        type="button"
                        variant="outline"
                        onClick={handleClear}
                        disabled={saving}
                      >
                        Clear
                      </Button>
                    )}
                  </div>
                </FormControl>
                {validating ? (
                  <div className="status">Checking key…</div>
                ) : (
                  <FormMessage />
                )}
              </FormItem>
            )}
          />
        </form>
      </Form>
      {savedOk && <div className="status ok">Saved</div>}
    </section>
  );
}
