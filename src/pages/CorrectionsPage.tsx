import { PencilSimpleIcon, TrashIcon } from "@phosphor-icons/react";
import { useEffect, useRef, useState } from "react";
import { toast } from "sonner";

import { EmptyRowCard } from "@/components/EmptyRowCard";
import { RowCard } from "@/components/RowCard";
import { SectionHeader } from "@/components/SectionHeader";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";

import { useSettings } from "../context/SettingsContext";
import { useFlash } from "../hooks/useFlash";
import {
  addCorrectionSet,
  deleteCorrectionSet,
  updateCorrectionSet,
} from "../lib/api";
import type { CorrectionEntry, Mode, NamedCorrectionSet } from "../lib/types";

type EntryDraft = { from: string; to: string };

type EntryEditState =
  | { kind: "none" }
  | { kind: "edit"; index: number; draft: EntryDraft }
  | { kind: "new"; draft: EntryDraft };

type DeleteConfirm = { set: NamedCorrectionSet; affectedModes: Mode[] };

export function CorrectionsPage() {
  const { settings, setSettings } = useSettings();
  const [expandedSetId, setExpandedSetId] = useState<string | null>(null);
  const [creatingName, setCreatingName] = useState<string | null>(null);
  const [deleteConfirm, setDeleteConfirm] = useState<DeleteConfirm | null>(null);

  const correctionSets = settings.correction_sets ?? [];

  const handleCreateSave = async () => {
    if (creatingName === null || !creatingName.trim()) return;
    const ms = Date.now();
    const newSet: NamedCorrectionSet = {
      id: `correction-set-${ms}`,
      name: creatingName.trim(),
      entries: [],
    };
    try {
      await addCorrectionSet(newSet);
      setSettings((s) => ({ ...s, correction_sets: [...s.correction_sets, newSet] }));
      setCreatingName(null);
      setExpandedSetId(newSet.id);
    } catch (e) {
      toast.error("Couldn't create correction set", { description: String(e) });
    }
  };

  const handleRename = async (setId: string, newName: string) => {
    const existing = correctionSets.find((cs) => cs.id === setId);
    if (!existing) return;
    const updated = { ...existing, name: newName };
    try {
      await updateCorrectionSet(updated);
      setSettings((s) => ({
        ...s,
        correction_sets: s.correction_sets.map((cs) =>
          cs.id === setId ? updated : cs,
        ),
      }));
    } catch (e) {
      toast.error("Couldn't rename correction set", { description: String(e) });
    }
  };

  const handleEntriesChange = async (setId: string, entries: CorrectionEntry[]) => {
    const existing = correctionSets.find((cs) => cs.id === setId);
    if (!existing) return;
    const updated = { ...existing, entries };
    try {
      await updateCorrectionSet(updated);
      setSettings((s) => ({
        ...s,
        correction_sets: s.correction_sets.map((cs) =>
          cs.id === setId ? updated : cs,
        ),
      }));
    } catch (e) {
      toast.error("Couldn't save corrections", { description: String(e) });
    }
  };

  const handleDeleteClick = (set: NamedCorrectionSet) => {
    const affectedModes = settings.modes.filter((m) =>
      m.correction_set_ids.includes(set.id),
    );
    setDeleteConfirm({ set, affectedModes });
  };

  const handleDeleteConfirm = async () => {
    if (!deleteConfirm) return;
    const { set } = deleteConfirm;
    try {
      await deleteCorrectionSet(set.id);
      setSettings((s) => ({
        ...s,
        correction_sets: s.correction_sets.filter((cs) => cs.id !== set.id),
        modes: s.modes.map((m) => ({
          ...m,
          correction_set_ids: m.correction_set_ids.filter((id) => id !== set.id),
        })),
      }));
      if (expandedSetId === set.id) setExpandedSetId(null);
    } catch (e) {
      toast.error("Couldn't delete correction set", { description: String(e) });
    } finally {
      setDeleteConfirm(null);
    }
  };

  return (
    <div className="p-6 flex flex-col gap-8">
      <SectionHeader
        title="Corrections"
        trailing={
          correctionSets.length > 0
            ? `${correctionSets.length} ${correctionSets.length === 1 ? "set" : "sets"}`
            : undefined
        }
      />

      <div className="flex flex-col gap-2">
        {correctionSets.length === 0 && creatingName === null ? (
          <EmptyRowCard
            preview={
              <span className="font-mono text-[13px] font-semibold text-muted-foreground/70">
                spoken → text
              </span>
            }
            hint="Group find-and-replace rules into named sets. Modes can apply any combination."
            action="New correction set"
            onClick={() => setCreatingName("")}
          />
        ) : (
          <>
            {correctionSets.map((set) => (
              <SetCard
                key={set.id}
                set={set}
                expanded={expandedSetId === set.id}
                onToggle={() =>
                  setExpandedSetId(expandedSetId === set.id ? null : set.id)
                }
                onRename={(name) => handleRename(set.id, name)}
                onDelete={() => handleDeleteClick(set)}
                onEntriesChange={(entries) => handleEntriesChange(set.id, entries)}
              />
            ))}

            {creatingName !== null ? (
              <NewSetRow
                name={creatingName}
                onChange={setCreatingName}
                onSave={handleCreateSave}
                onCancel={() => setCreatingName(null)}
              />
            ) : (
              <EmptyRowCard
                preview={
                  <span className="font-mono text-[13px] font-semibold text-muted-foreground/55">
                    New set
                  </span>
                }
                action="New correction set"
                onClick={() => setCreatingName("")}
                className="py-4"
              />
            )}
          </>
        )}
      </div>

      {deleteConfirm && (
        <DeleteConfirmDialog
          confirm={deleteConfirm}
          onConfirm={handleDeleteConfirm}
          onCancel={() => setDeleteConfirm(null)}
        />
      )}
    </div>
  );
}

function NewSetRow({
  name,
  onChange,
  onSave,
  onCancel,
}: {
  name: string;
  onChange: (v: string) => void;
  onSave: () => void;
  onCancel: () => void;
}) {
  const ref = useRef<HTMLInputElement>(null);
  useEffect(() => { ref.current?.focus(); }, []);

  return (
    <RowCard tone="accent" interactive={false} className="shadow-sm ring-2 ring-ring/15 gap-2 py-2.5">
      <Input
        ref={ref}
        value={name}
        onChange={(e) => onChange(e.target.value)}
        placeholder="Set name"
        className="h-8 flex-1"
        onKeyDown={(e) => {
          if (e.key === "Enter") onSave();
          if (e.key === "Escape") onCancel();
        }}
      />
      <div className="flex items-center gap-1">
        <Button variant="ghost" size="xs" onClick={onCancel}>Cancel</Button>
        <Button size="xs" onClick={onSave} disabled={!name.trim()}>Create</Button>
      </div>
    </RowCard>
  );
}

function SetCard({
  set,
  expanded,
  onToggle,
  onRename,
  onDelete,
  onEntriesChange,
}: {
  set: NamedCorrectionSet;
  expanded: boolean;
  onToggle: () => void;
  onRename: (name: string) => void;
  onDelete: () => void;
  onEntriesChange: (entries: CorrectionEntry[]) => void;
}) {
  const [renaming, setRenaming] = useState(false);
  const [draftName, setDraftName] = useState(set.name);
  const renameRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (renaming) renameRef.current?.focus();
  }, [renaming]);

  const commitRename = () => {
    const trimmed = draftName.trim();
    if (trimmed && trimmed !== set.name) onRename(trimmed);
    else setDraftName(set.name);
    setRenaming(false);
  };

  const count = set.entries.length;

  return (
    <div className="flex flex-col gap-0">
      <RowCard className={expanded ? "rounded-b-none" : undefined}>
        <div className="flex flex-1 min-w-0 items-center gap-2">
          {renaming ? (
            <Input
              ref={renameRef}
              value={draftName}
              onChange={(e) => setDraftName(e.target.value)}
              className="h-7 flex-1 font-semibold text-sm"
              onBlur={commitRename}
              onKeyDown={(e) => {
                if (e.key === "Enter") commitRename();
                if (e.key === "Escape") { setDraftName(set.name); setRenaming(false); }
              }}
            />
          ) : (
            <span className="text-sm font-semibold text-foreground truncate">
              {set.name}
            </span>
          )}
          {count > 0 && (
            <Badge variant="neutral" className="text-[10px] shrink-0">
              {count} {count === 1 ? "rule" : "rules"}
            </Badge>
          )}
        </div>

        <div className="flex items-center gap-0.5 shrink-0">
          <Button
            variant="ghost"
            size="sm"
            className="h-7 px-2 text-[12px] text-muted-foreground hover:text-foreground"
            onClick={onToggle}
          >
            {expanded ? "Close" : "Open"}
          </Button>
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                variant="ghost"
                size="icon-sm"
                aria-label="Rename"
                onClick={() => { setDraftName(set.name); setRenaming(true); }}
              >
                <PencilSimpleIcon size={14} />
              </Button>
            </TooltipTrigger>
            <TooltipContent>Rename</TooltipContent>
          </Tooltip>
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                variant="ghost"
                size="icon-sm"
                aria-label="Delete correction set"
                onClick={onDelete}
                className="text-muted-foreground/70 hover:text-destructive"
              >
                <TrashIcon size={14} />
              </Button>
            </TooltipTrigger>
            <TooltipContent>Delete</TooltipContent>
          </Tooltip>
        </div>
      </RowCard>

      {expanded && (
        <div className="border border-t-0 border-border rounded-b-md p-3 flex flex-col gap-2">
          <EntriesEditor entries={set.entries} onChange={onEntriesChange} />
        </div>
      )}
    </div>
  );
}

function EntriesEditor({
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

function DeleteConfirmDialog({
  confirm,
  onConfirm,
  onCancel,
}: {
  confirm: DeleteConfirm;
  onConfirm: () => void;
  onCancel: () => void;
}) {
  const { set, affectedModes } = confirm;
  return (
    <div
      role="dialog"
      aria-modal="true"
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/40"
      onClick={(e) => { if (e.target === e.currentTarget) onCancel(); }}
    >
      <div className="bg-background border border-border rounded-lg shadow-lg p-5 max-w-sm w-full flex flex-col gap-4">
        <div className="flex flex-col gap-1">
          <h2 className="text-sm font-semibold">Delete "{set.name}"?</h2>
          {affectedModes.length > 0 && (
            <p className="text-xs text-muted-foreground">
              This set is used by:{" "}
              <span className="font-medium text-foreground">
                {affectedModes.map((m) => m.name).join(", ")}
              </span>
              . It will be unlinked from {affectedModes.length === 1 ? "that mode" : "those modes"}.
            </p>
          )}
        </div>
        <div className="flex justify-end gap-2">
          <Button variant="ghost" size="sm" onClick={onCancel}>Cancel</Button>
          <Button variant="destructive" size="sm" onClick={onConfirm}>Delete</Button>
        </div>
      </div>
    </div>
  );
}
