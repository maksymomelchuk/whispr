import { zodResolver } from "@hookform/resolvers/zod";
import { useEffect, useState } from "react";
import { useForm } from "react-hook-form";
import { toast } from "sonner";
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
import { cn } from "@/lib/utils";

import type { ApiKeyValidation } from "../lib/types";
import { InfoTip } from "./InfoTip";

const schema = z.object({ value: z.string().min(1, "Required") });
type Values = z.infer<typeof schema>;

interface Props {
  label: string;
  info?: string;
  placeholder?: string;
  isConfigured: boolean;
  persist: (value: string) => Promise<void>;
  validate?: (value: string) => Promise<ApiKeyValidation>;
  onConfiguredChange: (configured: boolean) => void;
  className?: string;
}

export function CredentialField({
  label,
  info,
  placeholder,
  isConfigured,
  persist,
  validate,
  onConfiguredChange,
  className,
}: Props) {
  const [editing, setEditing] = useState(!isConfigured);
  const [saving, setSaving] = useState(false);
  const [validating, setValidating] = useState(false);

  const form = useForm<Values>({
    resolver: zodResolver(schema),
    defaultValues: { value: "" },
  });

  useEffect(() => {
    if (!isConfigured) setEditing(true);
  }, [isConfigured]);

  const cancel = () => {
    form.reset({ value: "" });
    form.clearErrors("value");
    setEditing(false);
  };

  const onSubmit = form.handleSubmit(async ({ value }) => {
    const trimmed = value.trim();
    if (!trimmed) return;
    setSaving(true);
    form.clearErrors("value");
    try {
      await persist(trimmed);
      onConfiguredChange(true);
      form.reset({ value: "" });
      setEditing(false);
    } catch (e) {
      toast.error(`Couldn't save ${label}`, { description: String(e) });
    } finally {
      setSaving(false);
    }
  });

  const handleRemove = async () => {
    setSaving(true);
    try {
      await persist("");
      onConfiguredChange(false);
      form.reset({ value: "" });
      setEditing(true);
    } catch (e) {
      toast.error(`Couldn't remove ${label}`, { description: String(e) });
    } finally {
      setSaving(false);
    }
  };

  const handleBlur = async (val: string) => {
    if (!validate) return;
    const trimmed = val.trim();
    if (!trimmed) return;
    setValidating(true);
    form.clearErrors("value");
    try {
      const result = await validate(trimmed);
      if (result.kind === "invalid") {
        form.setError("value", {
          message: "Key was rejected by the provider.",
        });
      } else if (result.kind === "error") {
        form.setError("value", {
          message: `Couldn't validate: ${result.message}`,
        });
      }
    } catch (e) {
      form.setError("value", {
        message: `Couldn't validate: ${String(e)}`,
      });
    } finally {
      setValidating(false);
    }
  };

  return (
    <div className={cn("flex flex-col gap-[6px]", className)}>
      <div className="inline-flex items-center gap-2">
        <span className="text-form-label text-muted-foreground">{label}</span>
        {info && <InfoTip text={info} />}
      </div>

      {isConfigured && !editing ? (
        <div className="flex items-center gap-2">
          <div
            aria-label="Saved, hidden"
            className={cn(
              "flex h-9 flex-1 items-center rounded-md border border-input bg-background/40 px-3",
              "select-none font-mono text-[13px] tracking-[0.45em] text-muted-foreground/55",
            )}
          >
            ••••••••••••
          </div>
          <Button
            type="button"
            variant="outline"
            onClick={() => setEditing(true)}
            disabled={saving}
          >
            Replace
          </Button>
          <Button
            type="button"
            variant="ghost"
            onClick={handleRemove}
            disabled={saving}
            className="text-muted-foreground hover:text-destructive"
          >
            Remove
          </Button>
        </div>
      ) : (
        <Form {...form}>
          <form
            onSubmit={onSubmit}
            onKeyDown={(e) => {
              if (e.key === "Escape" && isConfigured) {
                e.preventDefault();
                cancel();
              }
            }}
          >
            <FormField
              control={form.control}
              name="value"
              render={({ field }) => (
                <FormItem>
                  <FormControl>
                    <div className="flex items-center gap-2">
                      <Input
                        {...field}
                        type="password"
                        autoFocus={isConfigured}
                        placeholder={placeholder ?? ""}
                        spellCheck={false}
                        autoComplete="off"
                        onChange={(e) => {
                          field.onChange(e);
                          form.clearErrors("value");
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
                          variant="ghost"
                          onClick={cancel}
                          disabled={saving}
                        >
                          Cancel
                        </Button>
                      )}
                    </div>
                  </FormControl>
                  {validating ? (
                    <p className="mt-1.5 text-help text-muted-foreground">
                      Checking key…
                    </p>
                  ) : (
                    <FormMessage className="mt-1.5 text-help" />
                  )}
                </FormItem>
              )}
            />
          </form>
        </Form>
      )}
    </div>
  );
}
