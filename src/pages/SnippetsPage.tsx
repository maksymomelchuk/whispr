import { useRef, useState } from "react";
import { PencilSimple, Trash } from "@phosphor-icons/react";

import { Alert, AlertDescription } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
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

import { useSettings } from "../context/SettingsContext";
import { getSettings, setSnippets } from "../lib/api";
import type { Snippet } from "../lib/types";

const PLACEHOLDERS = [
  { label: "{{DATE}}", description: "Today's date (YYYY-MM-DD)" },
  { label: "{{TIME}}", description: "Current time (HH:MM)" },
  { label: "{{CLIPBOARD}}", description: "Current clipboard text" },
];

function SnippetRow({
  snippet,
  onEdit,
  onDelete,
}: {
  snippet: Snippet;
  onEdit: () => void;
  onDelete: () => void;
}) {
  const usedPlaceholders = PLACEHOLDERS.filter((p) =>
    snippet.expansion.includes(p.label)
  );
  return (
    <div className="flex items-center gap-3 rounded-[10px] border border-border bg-card px-4 py-3">
      <div className="flex flex-1 min-w-0 flex-col gap-0.5">
        <div className="flex items-center gap-2 flex-wrap">
          <span className="font-mono text-sm font-semibold text-foreground">
            {snippet.trigger}
          </span>
          <span className="text-muted-foreground text-xs">→</span>
          <span className="text-xs text-muted-foreground truncate max-w-[200px]">
            {snippet.expansion}
          </span>
        </div>
        {usedPlaceholders.length > 0 && (
          <div className="flex gap-1 flex-wrap mt-0.5">
            {usedPlaceholders.map((p) => (
              <Badge key={p.label} variant="neutral" className="text-[10px]">
                {p.label}
              </Badge>
            ))}
          </div>
        )}
      </div>
      <div className="flex items-center gap-1">
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              type="button"
              variant="ghost"
              size="icon-sm"
              aria-label="Edit"
              onClick={onEdit}
            >
              <PencilSimple size={14} />
            </Button>
          </TooltipTrigger>
          <TooltipContent>Edit</TooltipContent>
        </Tooltip>
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              type="button"
              variant="ghost"
              size="icon-sm"
              aria-label="Delete"
              onClick={onDelete}
              className="text-muted-foreground/70 hover:text-destructive"
            >
              <Trash size={14} />
            </Button>
          </TooltipTrigger>
          <TooltipContent>Delete</TooltipContent>
        </Tooltip>
      </div>
    </div>
  );
}

type EditorState = { snippet: Snippet; isNew: boolean } | null;

export function SnippetsPage() {
  const { settings, setSettings } = useSettings();
  const [editor, setEditor] = useState<EditorState>(null);
  const [draft, setDraft] = useState<Snippet>({
    id: "",
    trigger: "",
    expansion: "",
  });
  const [saveError, setSaveError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const expansionRef = useRef<HTMLTextAreaElement>(null);

  if (!settings) return null;

  const snippets = settings.snippets;

  function openNew() {
    const newSnippet: Snippet = {
      id: `snippet-${Date.now()}`,
      trigger: "",
      expansion: "",
    };
    setDraft(newSnippet);
    setEditor({ snippet: newSnippet, isNew: true });
    setSaveError(null);
  }

  function openEdit(snippet: Snippet) {
    setDraft({ ...snippet });
    setEditor({ snippet, isNew: false });
    setSaveError(null);
  }

  function closeEditor() {
    setEditor(null);
    setSaveError(null);
  }

  async function persistSnippets(updated: Snippet[]): Promise<boolean> {
    setSaving(true);
    try {
      await setSnippets(updated);
      const fresh = await getSettings();
      setSettings((prev) => (prev ? { ...prev, snippets: fresh.snippets } : prev));
      return true;
    } catch (e) {
      setSaveError(String(e));
      return false;
    } finally {
      setSaving(false);
    }
  }

  async function handleDelete(id: string) {
    await persistSnippets(snippets.filter((s) => s.id !== id));
  }

  async function handleSave() {
    if (!draft.trigger.trim()) {
      setSaveError("Trigger cannot be empty.");
      return;
    }
    const trimmed: Snippet = { ...draft, trigger: draft.trigger.trim() };
    const updated = editor?.isNew
      ? [...snippets, trimmed]
      : snippets.map((s) => (s.id === trimmed.id ? trimmed : s));
    setSaveError(null);
    if (await persistSnippets(updated)) {
      closeEditor();
    }
  }

  function insertPlaceholder(placeholder: string) {
    const el = expansionRef.current;
    if (!el) {
      setDraft((d) => ({ ...d, expansion: d.expansion + placeholder }));
      return;
    }
    const start = el.selectionStart ?? el.value.length;
    const end = el.selectionEnd ?? el.value.length;
    const newValue =
      el.value.slice(0, start) + placeholder + el.value.slice(end);
    setDraft((d) => ({ ...d, expansion: newValue }));
    requestAnimationFrame(() => {
      el.focus();
      el.setSelectionRange(start + placeholder.length, start + placeholder.length);
    });
  }

  return (
    <div className="mx-auto max-w-2xl px-6 py-6 space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-lg font-semibold">Snippets</h1>
          <p className="text-sm text-muted-foreground mt-0.5">
            Triggers in your dictation expand to their text after cleanup. Per-mode on/off in Mode settings.
          </p>
        </div>
        <Button size="sm" onClick={openNew}>
          + Add
        </Button>
      </div>

      {snippets.length === 0 ? (
        <div className="rounded-[10px] border border-dashed border-border px-4 py-8 text-center text-sm text-muted-foreground">
          No snippets yet. Add one to get started.
        </div>
      ) : (
        <div className="flex flex-col gap-2">
          {snippets.map((snippet) => (
            <SnippetRow
              key={snippet.id}
              snippet={snippet}
              onEdit={() => openEdit(snippet)}
              onDelete={() => handleDelete(snippet.id)}
            />
          ))}
        </div>
      )}

      {saveError && !editor && (
        <Alert variant="destructive">
          <AlertDescription>{saveError}</AlertDescription>
        </Alert>
      )}

      <Sheet open={editor !== null} onOpenChange={(open) => !open && closeEditor()}>
        <SheetContent className="flex flex-col gap-0 overflow-y-auto">
          <SheetHeader className="pb-4">
            <SheetTitle>
              {editor?.isNew ? "New Snippet" : "Edit Snippet"}
            </SheetTitle>
          </SheetHeader>

          <div className="flex flex-col gap-4 flex-1">
            <div className="space-y-1.5">
              <Label htmlFor="snippet-trigger">Trigger</Label>
              <Input
                id="snippet-trigger"
                value={draft.trigger}
                onChange={(e) =>
                  setDraft((d) => ({ ...d, trigger: e.target.value }))
                }
                placeholder="e.g. [date] or my email"
                spellCheck={false}
                autoComplete="off"
              />
              <p className="text-xs text-muted-foreground">
                Exact, case-sensitive match. All occurrences in the transcript are replaced.
              </p>
            </div>

            <div className="space-y-1.5">
              <Label htmlFor="snippet-expansion">Expansion</Label>
              <Textarea
                id="snippet-expansion"
                ref={expansionRef}
                value={draft.expansion}
                onChange={(e) =>
                  setDraft((d) => ({ ...d, expansion: e.target.value }))
                }
                placeholder="e.g. user@example.com or Today is {{DATE}}"
                spellCheck={false}
                autoComplete="off"
                rows={3}
                className="resize-none font-mono text-sm"
              />
              <div className="space-y-1.5">
                <p className="text-xs text-muted-foreground">
                  Insert a placeholder:
                </p>
                <div className="flex flex-wrap gap-1.5">
                  {PLACEHOLDERS.map((p) => (
                    <Tooltip key={p.label}>
                      <TooltipTrigger asChild>
                        <Button
                          type="button"
                          variant="outline"
                          size="sm"
                          className="font-mono text-xs h-7 px-2"
                          onClick={() => insertPlaceholder(p.label)}
                        >
                          {p.label}
                        </Button>
                      </TooltipTrigger>
                      <TooltipContent>{p.description}</TooltipContent>
                    </Tooltip>
                  ))}
                </div>
              </div>
            </div>

            {saveError && (
              <Alert variant="destructive">
                <AlertDescription>{saveError}</AlertDescription>
              </Alert>
            )}
          </div>

          <SheetFooter className="pt-4">
            <Button variant="outline" onClick={closeEditor} disabled={saving}>
              Cancel
            </Button>
            <Button onClick={handleSave} disabled={saving}>
              {saving ? "Saving…" : "Save"}
            </Button>
          </SheetFooter>
        </SheetContent>
      </Sheet>
    </div>
  );
}
