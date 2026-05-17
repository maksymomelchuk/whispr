import { TrashIcon } from "@phosphor-icons/react";
import { useEffect, useRef, useState } from "react";

import { EmptyRowCard } from "@/components/EmptyRowCard";
import { RowCard } from "@/components/RowCard";
import { SectionHeader } from "@/components/SectionHeader";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";

import { useSettings } from "../context/SettingsContext";
import { useFlash } from "../hooks/useFlash";
import { getSettings, setSnippets } from "../lib/api";
import type { Snippet } from "../lib/types";

const PLACEHOLDERS = [
  { label: "{{DATE}}", description: "Today's date (YYYY-MM-DD)" },
  { label: "{{TIME}}", description: "Current time (HH:MM)" },
  { label: "{{CLIPBOARD}}", description: "Current clipboard text" },
];

type Draft = { trigger: string; expansion: string };

function SnippetRow({
  snippet,
  flashing,
  onEdit,
  onDelete,
}: {
  snippet: Snippet;
  flashing: boolean;
  onEdit: () => void;
  onDelete: () => void;
}) {
  const usedPlaceholders = PLACEHOLDERS.filter((p) =>
    snippet.expansion.includes(p.label),
  );
  return (
    <RowCard flashing={flashing}>
      <div className="flex flex-1 min-w-0 items-center gap-3">
        <span className="font-mono text-[13px] font-semibold text-foreground truncate max-w-[40%]">
          {snippet.trigger || (
            <span className="text-muted-foreground/60 italic">(empty)</span>
          )}
        </span>
        <span className="text-muted-foreground/60 text-help shrink-0">→</span>
        <span className="flex-1 truncate text-xs text-muted-foreground">
          {snippet.expansion || (
            <span className="italic text-muted-foreground/60">(empty)</span>
          )}
        </span>
        {usedPlaceholders.length > 0 && (
          <div className="hidden md:flex gap-1 flex-wrap shrink-0">
            {usedPlaceholders.map((p) => (
              <Badge
                key={p.label}
                variant="neutral"
                className="font-mono text-[10px] tracking-tight"
              >
                {p.label}
              </Badge>
            ))}
          </div>
        )}
      </div>

      <div className="flex items-center gap-0.5 shrink-0 transform-gpu opacity-65 group-hover:opacity-100 transition-opacity">
        <Button
          variant="ghost"
          size="xs"
          className="text-muted-foreground hover:text-foreground "
          onClick={onEdit}
        >
          Edit
        </Button>
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant="ghost"
              size="icon-sm"
              aria-label="Delete snippet"
              onClick={onDelete}
              className="transition-colors text-muted-foreground/70 hover:text-destructive"
            >
              <TrashIcon size={15} />
            </Button>
          </TooltipTrigger>
          <TooltipContent>Delete</TooltipContent>
        </Tooltip>
      </div>
    </RowCard>
  );
}

function EditorRow({
  draft,
  onChange,
  onSave,
  onCancel,
  saving,
  error,
}: {
  draft: Draft;
  onChange: (next: Draft) => void;
  onSave: () => void;
  onCancel: () => void;
  saving: boolean;
  error: string | null;
}) {
  const triggerRef = useRef<HTMLInputElement>(null);
  const expansionRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    triggerRef.current?.focus();
  }, []);

  function insertPlaceholder(placeholder: string) {
    const el = expansionRef.current;
    if (!el) {
      onChange({ ...draft, expansion: draft.expansion + placeholder });
      return;
    }
    const start = el.selectionStart ?? el.value.length;
    const end = el.selectionEnd ?? el.value.length;
    const newValue =
      el.value.slice(0, start) + placeholder + el.value.slice(end);
    onChange({ ...draft, expansion: newValue });
    requestAnimationFrame(() => {
      el.focus();
      el.setSelectionRange(
        start + placeholder.length,
        start + placeholder.length,
      );
    });
  }

  function handleKeyDown(e: React.KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      onCancel();
    }
    if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
      e.preventDefault();
      onSave();
    }
  }

  return (
    <RowCard
      tone="accent"
      interactive={false}
      className="shadow-sm ring-2 ring-ring/15 items-stretch flex-col gap-2.5 py-3 pr-3"
      onKeyDown={handleKeyDown}
    >
      <div className="flex items-start gap-2.5">
        <Input
          ref={triggerRef}
          value={draft.trigger}
          onChange={(e) => onChange({ ...draft, trigger: e.target.value })}
          placeholder="trigger"
          spellCheck={false}
          autoComplete="off"
          className="font-mono font-semibold h-8 max-w-[200px]"
        />
        <span className="pt-1.5 text-muted-foreground/60 text-[12px]">→</span>
        <Textarea
          ref={expansionRef}
          value={draft.expansion}
          onChange={(e) => onChange({ ...draft, expansion: e.target.value })}
          placeholder="expansion (e.g. user@example.com or Today is {{DATE}})"
          spellCheck={false}
          autoComplete="off"
          rows={2}
          className="flex-1 resize-none font-mono text-xs min-h-[36px] py-1.5"
        />
      </div>
      <div className="flex items-center gap-1.5 flex-wrap pl-0.5">
        <span className="text-eyebrow uppercase text-muted-foreground/70 mr-1">
          Insert
        </span>
        {PLACEHOLDERS.map((p) => (
          <Tooltip key={p.label}>
            <TooltipTrigger asChild>
              <Button
                type="button"
                variant="outline"
                size="sm"
                className="font-mono text-help h-6 px-1.5"
                onClick={() => insertPlaceholder(p.label)}
              >
                {p.label}
              </Button>
            </TooltipTrigger>
            <TooltipContent>{p.description}</TooltipContent>
          </Tooltip>
        ))}
        <div className="ml-auto flex items-center gap-1">
          <Button
            variant="ghost"
            size="xs"
            onClick={onCancel}
            disabled={saving}
          >
            Cancel
          </Button>
          <Button
            size="xs"
            onClick={onSave}
            disabled={saving || !draft.trigger.trim()}
          >
            {saving ? "Saving…" : "Save"}
          </Button>
        </div>
      </div>
      {error && <p className="text-help text-destructive px-0.5">{error}</p>}
    </RowCard>
  );
}

function EmptyState({ onAdd }: { onAdd: () => void }) {
  return (
    <EmptyRowCard
      preview={
        <span className="font-mono text-[13px] font-semibold text-muted-foreground/70">
          [sample]
        </span>
      }
      hint="Triggers in your dictation expand to their text after cleanup."
      action="Add snippet"
      onClick={onAdd}
    />
  );
}

type EditingState =
  | { kind: "none" }
  | { kind: "edit"; id: string; draft: Draft }
  | { kind: "new"; draft: Draft };

export function SnippetsPage() {
  const { settings, setSettings } = useSettings();
  const [editing, setEditing] = useState<EditingState>({ kind: "none" });
  const [saveError, setSaveError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const { flash, isFlashing } = useFlash();

  const snippets = settings.snippets;

  function startNew() {
    setEditing({ kind: "new", draft: { trigger: "", expansion: "" } });
    setSaveError(null);
  }

  function startEdit(s: Snippet) {
    setEditing({
      kind: "edit",
      id: s.id,
      draft: { trigger: s.trigger, expansion: s.expansion },
    });
    setSaveError(null);
  }

  function cancelEdit() {
    setEditing({ kind: "none" });
    setSaveError(null);
  }

  async function persist(updated: Snippet[]): Promise<boolean> {
    setSaving(true);
    try {
      await setSnippets(updated);
      const fresh = await getSettings();
      setSettings((prev) => ({ ...prev, snippets: fresh.snippets }));
      return true;
    } catch (e) {
      setSaveError(String(e));
      return false;
    } finally {
      setSaving(false);
    }
  }

  async function handleDelete(id: string) {
    await persist(snippets.filter((s) => s.id !== id));
  }

  async function handleSave() {
    if (editing.kind === "none") return;
    const trimmed = editing.draft.trigger.trim();
    if (!trimmed) {
      setSaveError("Trigger cannot be empty.");
      return;
    }

    let id: string;
    let updated: Snippet[];
    if (editing.kind === "new") {
      id = `snippet-${Date.now()}`;
      updated = [
        ...snippets,
        { id, trigger: trimmed, expansion: editing.draft.expansion },
      ];
    } else {
      id = editing.id;
      updated = snippets.map((s) =>
        s.id === id
          ? { ...s, trigger: trimmed, expansion: editing.draft.expansion }
          : s,
      );
    }
    setSaveError(null);
    if (await persist(updated)) {
      setEditing({ kind: "none" });
      flash(id);
    }
  }

  const editingNew = editing.kind === "new";
  const editingId = editing.kind === "edit" ? editing.id : null;
  const showTopLevelError = saveError !== null && editing.kind === "none";

  return (
    <div className="p-6 flex flex-col gap-8">
      <SectionHeader
        title="Snippets"
        trailing={
          snippets.length > 0
            ? `${snippets.length} ${snippets.length === 1 ? "entry" : "entries"}`
            : undefined
        }
      />

      {snippets.length === 0 && !editingNew ? (
        <EmptyState onAdd={startNew} />
      ) : (
        <div className="flex flex-col gap-2">
          {snippets.map((snippet) =>
            editingId === snippet.id && editing.kind === "edit" ? (
              <EditorRow
                key={snippet.id}
                draft={editing.draft}
                onChange={(draft) =>
                  setEditing({ kind: "edit", id: snippet.id, draft })
                }
                onSave={handleSave}
                onCancel={cancelEdit}
                saving={saving}
                error={saveError}
              />
            ) : (
              <SnippetRow
                key={snippet.id}
                snippet={snippet}
                flashing={isFlashing(snippet.id)}
                onEdit={() => startEdit(snippet)}
                onDelete={() => handleDelete(snippet.id)}
              />
            ),
          )}

          {editingNew && editing.kind === "new" && (
            <EditorRow
              draft={editing.draft}
              onChange={(draft) => setEditing({ kind: "new", draft })}
              onSave={handleSave}
              onCancel={cancelEdit}
              saving={saving}
              error={saveError}
            />
          )}

          {!editingNew && (
            <EmptyRowCard
              preview={
                <span className="font-mono text-[13px] font-semibold text-muted-foreground/55">
                  trigger → expansion
                </span>
              }
              action="Add snippet"
              onClick={startNew}
              className="py-4"
            />
          )}
        </div>
      )}

      {showTopLevelError && (
        <Alert variant="destructive">
          <AlertDescription>{saveError}</AlertDescription>
        </Alert>
      )}
    </div>
  );
}
