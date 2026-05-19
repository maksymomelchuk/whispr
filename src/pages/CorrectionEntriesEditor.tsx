import { TrashIcon } from "@phosphor-icons/react";
import { useEffect, useRef, useState } from "react";

import { EmptyRowCard } from "@/components/EmptyRowCard";
import { RowCard } from "@/components/RowCard";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";

import { useFlash } from "../hooks/useFlash";
import type { CorrectionEntry } from "../lib/types";

type EntryDraft = { from: string; to: string };

type EntryEditState =
  | { kind: "none" }
  | { kind: "edit"; index: number; draft: EntryDraft }
  | { kind: "new"; draft: EntryDraft };

export function EntriesEditor({
  entries,
  onChange,
}: {
  entries: CorrectionEntry[];
  onChange: (next: CorrectionEntry[]) => void;
}) {
  const [editing, setEditing] = useState<EntryEditState>({ kind: "none" });
  const [saveError, setSaveError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const { flash, isFlashing } = useFlash();

  function startNew() {
    setEditing({ kind: "new", draft: { from: "", to: "" } });
    setSaveError(null);
  }

  function startEdit(index: number) {
    setEditing({ kind: "edit", index, draft: { ...entries[index] } });
    setSaveError(null);
  }

  function cancelEdit() {
    setEditing({ kind: "none" });
    setSaveError(null);
  }

  async function persist(next: CorrectionEntry[]): Promise<boolean> {
    setSaving(true);
    try {
      onChange(next);
      return true;
    } catch (e) {
      setSaveError(String(e));
      return false;
    } finally {
      setSaving(false);
    }
  }

  async function handleDelete(index: number) {
    await persist(entries.filter((_, i) => i !== index));
  }

  async function handleSave() {
    if (editing.kind === "none") return;
    const from = editing.draft.from.trim();
    if (!from) { setSaveError("Spoken form cannot be empty."); return; }
    const entry: CorrectionEntry = { from, to: editing.draft.to };
    let next: CorrectionEntry[];
    let flashKey: string;
    if (editing.kind === "new") {
      next = [...entries, entry];
      flashKey = `e-${next.length - 1}`;
    } else {
      next = entries.map((e, i) => (i === editing.index ? entry : e));
      flashKey = `e-${editing.index}`;
    }
    setSaveError(null);
    if (await persist(next)) {
      setEditing({ kind: "none" });
      flash(flashKey);
    }
  }

  const editingNew = editing.kind === "new";
  const editingIndex = editing.kind === "edit" ? editing.index : null;
  const showTopError = saveError !== null && editing.kind === "none";

  return (
    <div className="flex flex-col gap-2">
      <p className="text-[12px] text-muted-foreground/85 max-w-prose">
        Find-and-replace rules applied after AI cleanup and snippets.
      </p>

      {entries.length === 0 && !editingNew ? (
        <EmptyRowCard
          preview={
            <span className="font-mono text-[13px] font-semibold text-muted-foreground/70">
              spoken → text
            </span>
          }
          action="Add rule"
          onClick={startNew}
        />
      ) : (
        <div className="flex flex-col gap-1.5">
          {entries.map((entry, i) =>
            editingIndex === i && editing.kind === "edit" ? (
              <EntryEditorRow
                key={`edit-${i}`}
                draft={editing.draft}
                onChange={(draft) => setEditing({ kind: "edit", index: i, draft })}
                onSave={handleSave}
                onCancel={cancelEdit}
                saving={saving}
                error={saveError}
              />
            ) : (
              <CorrectionRow
                key={`e-${i}`}
                entry={entry}
                flashing={isFlashing(`e-${i}`)}
                onEdit={() => startEdit(i)}
                onDelete={() => handleDelete(i)}
              />
            ),
          )}

          {editingNew && editing.kind === "new" && (
            <EntryEditorRow
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
                  spoken → text
                </span>
              }
              action="Add rule"
              onClick={startNew}
              className="py-3"
            />
          )}
        </div>
      )}

      {showTopError && (
        <Alert variant="destructive">
          <AlertDescription>{saveError}</AlertDescription>
        </Alert>
      )}
    </div>
  );
}

function CorrectionRow({
  entry,
  flashing,
  onEdit,
  onDelete,
}: {
  entry: CorrectionEntry;
  flashing: boolean;
  onEdit: () => void;
  onDelete: () => void;
}) {
  return (
    <RowCard flashing={flashing}>
      <div className="flex flex-1 min-w-0 items-center gap-3">
        <span className="font-mono text-[13px] font-semibold text-foreground truncate max-w-[40%]">
          {entry.from || <span className="text-muted-foreground/60 italic">(empty)</span>}
        </span>
        <span className="text-muted-foreground/60 text-help shrink-0">→</span>
        <span className="flex-1 truncate text-xs text-muted-foreground">
          {entry.to || <span className="italic text-muted-foreground/60">(empty)</span>}
        </span>
      </div>
      <div className="flex items-center gap-0.5 shrink-0 transform-gpu opacity-65 group-hover:opacity-100 transition-opacity">
        <Button
          variant="ghost"
          size="sm"
          className="h-7 px-2 text-[12px] text-muted-foreground hover:text-foreground"
          onClick={onEdit}
        >
          Edit
        </Button>
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant="ghost"
              size="icon-sm"
              aria-label="Delete rule"
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

function EntryEditorRow({
  draft,
  onChange,
  onSave,
  onCancel,
  saving,
  error,
}: {
  draft: EntryDraft;
  onChange: (next: EntryDraft) => void;
  onSave: () => void;
  onCancel: () => void;
  saving: boolean;
  error: string | null;
}) {
  const fromRef = useRef<HTMLInputElement>(null);
  useEffect(() => { fromRef.current?.focus(); }, []);

  function handleKeyDown(e: React.KeyboardEvent) {
    if (e.key === "Escape") { e.preventDefault(); onCancel(); }
    if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) { e.preventDefault(); onSave(); }
  }

  return (
    <RowCard
      tone="accent"
      interactive={false}
      className="shadow-sm ring-2 ring-ring/15 items-stretch flex-col gap-2 py-2.5 pr-3"
      onKeyDown={handleKeyDown}
    >
      <div className="flex items-center gap-2">
        <Input
          ref={fromRef}
          value={draft.from}
          onChange={(e) => onChange({ ...draft, from: e.target.value })}
          placeholder="spoken"
          spellCheck={false}
          autoComplete="off"
          className="font-mono font-semibold h-8 flex-1 min-w-0"
        />
        <span className="select-none text-muted-foreground/60 text-[12px]">→</span>
        <Input
          value={draft.to}
          onChange={(e) => onChange({ ...draft, to: e.target.value })}
          placeholder="text"
          spellCheck={false}
          autoComplete="off"
          className="text-xs h-8 flex-1 min-w-0"
          onKeyDown={(e) => {
            if (e.key === "Enter" && !e.metaKey && !e.ctrlKey) { e.preventDefault(); onSave(); }
          }}
        />
        <div className="flex items-center gap-1 ml-1">
          <Button variant="ghost" size="xs" onClick={onCancel} disabled={saving}>Cancel</Button>
          <Button size="xs" onClick={onSave} disabled={saving || !draft.from.trim()}>
            {saving ? "Saving…" : "Save"}
          </Button>
        </div>
      </div>
      {error && <p className="text-help text-destructive px-0.5">{error}</p>}
    </RowCard>
  );
}
