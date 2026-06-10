import { PencilSimpleIcon, TrashIcon } from "@phosphor-icons/react";
import { useRef, useState } from "react";
import { toast } from "sonner";

import { EmptyRowCard } from "@/components/EmptyRowCard";
import { RowCard } from "@/components/RowCard";
import { SectionHeader } from "@/components/SectionHeader";
import { TermChipInput } from "@/components/TermChipInput";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";

import { useSettings } from "../context/SettingsContext";
import {
  createTermSet,
  deleteTermSet,
  renameTermSet,
  updateTermSetEntries,
} from "../lib/api";
import type { NamedTermSet } from "../lib/types";

type DeleteTarget = { set: NamedTermSet; affectedModeNames: string[] };

export function TermsPage() {
  const { settings, setSettings } = useSettings();
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const [renamingId, setRenamingId] = useState<string | null>(null);
  const [renameDraft, setRenameDraft] = useState("");
  const [deleteTarget, setDeleteTarget] = useState<DeleteTarget | null>(null);
  const [newName, setNewName] = useState("");
  const [creating, setCreating] = useState(false);
  const newInputRef = useRef<HTMLInputElement>(null);

  const termSets = settings.term_sets ?? [];

  async function handleCreate() {
    const name = newName.trim();
    if (!name) return;
    setCreating(true);
    try {
      const updated = await createTermSet(name);
      const newId = updated.term_sets.at(-1)?.id ?? null;
      setSettings(() => updated);
      setNewName("");
      setExpandedId(newId);
    } catch (e) {
      toast.error("Couldn't create term set", { description: String(e) });
    } finally {
      setCreating(false);
    }
  }

  function startRename(set: NamedTermSet) {
    setRenamingId(set.id);
    setRenameDraft(set.name);
  }

  async function commitRename(id: string) {
    const name = renameDraft.trim();
    if (!name) {
      setRenamingId(null);
      return;
    }
    try {
      const updated = await renameTermSet(id, name);
      setSettings(() => updated);
    } catch (e) {
      toast.error("Couldn't rename term set", { description: String(e) });
    } finally {
      setRenamingId(null);
    }
  }

  function openDeleteDialog(set: NamedTermSet) {
    const affectedModeNames = settings.modes
      .filter((m) => m.term_set_ids.includes(set.id))
      .map((m) => m.name);
    setDeleteTarget({ set, affectedModeNames });
  }

  async function confirmDelete() {
    if (!deleteTarget) return;
    const { set } = deleteTarget;
    setDeleteTarget(null);
    try {
      const updated = await deleteTermSet(set.id);
      setSettings(() => updated);
      if (expandedId === set.id) setExpandedId(null);
    } catch (e) {
      toast.error("Couldn't delete term set", { description: String(e) });
    }
  }

  async function handleEntriesChange(id: string, entries: string[]) {
    try {
      const updated = await updateTermSetEntries(id, entries);
      setSettings(() => updated);
    } catch (e) {
      toast.error("Couldn't save entries", { description: String(e) });
    }
  }

  const totalEntries = termSets.reduce((n, ts) => n + ts.entries.length, 0);
  const trailing =
    termSets.length > 0
      ? `${termSets.length} ${termSets.length === 1 ? "set" : "sets"}, ${totalEntries} ${totalEntries === 1 ? "entry" : "entries"}`
      : undefined;

  return (
    <div className="p-6 flex flex-col gap-8">
      <SectionHeader title="Vocabulary" trailing={trailing} />

      <p className="text-[12px] text-muted-foreground/85 max-w-prose -mt-5">
        Vocabulary hints sent to the recognizer so it picks your exact spelling.
        Organize terms into named sets and assign sets to profiles.
      </p>

      <div className="flex flex-col gap-2">
        {termSets.length === 0 ? (
          <EmptyRowCard
            preview={
              <span className="text-[13px] text-muted-foreground/70 font-semibold">
                No term sets yet
              </span>
            }
            hint="Create a set and assign it to profiles to bias transcription."
            action="New term set"
            onClick={() => newInputRef.current?.focus()}
          />
        ) : (
          termSets.map((set) => (
            <TermSetRow
              key={set.id}
              set={set}
              expanded={expandedId === set.id}
              renaming={renamingId === set.id}
              renameDraft={renameDraft}
              onToggleExpand={() =>
                setExpandedId((id) => (id === set.id ? null : set.id))
              }
              onStartRename={() => startRename(set)}
              onRenameDraftChange={setRenameDraft}
              onCommitRename={() => commitRename(set.id)}
              onDelete={() => openDeleteDialog(set)}
              onEntriesChange={(entries) =>
                handleEntriesChange(set.id, entries)
              }
            />
          ))
        )}
      </div>

      <NewSetRow
        value={newName}
        onChange={setNewName}
        onSubmit={handleCreate}
        creating={creating}
        inputRef={newInputRef}
      />

      <DeleteConfirmDialog
        target={deleteTarget}
        onConfirm={confirmDelete}
        onCancel={() => setDeleteTarget(null)}
      />
    </div>
  );
}

function TermSetRow({
  set,
  expanded,
  renaming,
  renameDraft,
  onToggleExpand,
  onStartRename,
  onRenameDraftChange,
  onCommitRename,
  onDelete,
  onEntriesChange,
}: {
  set: NamedTermSet;
  expanded: boolean;
  renaming: boolean;
  renameDraft: string;
  onToggleExpand: () => void;
  onStartRename: () => void;
  onRenameDraftChange: (v: string) => void;
  onCommitRename: () => void;
  onDelete: () => void;
  onEntriesChange: (entries: string[]) => void;
}) {
  const count = set.entries.length;

  return (
    <div className="flex flex-col gap-0 group/row">
      <RowCard
        interactive={!renaming}
        className={
          expanded
            ? "rounded-b-none border-b-0 group-hover/row:border-ring/55"
            : ""
        }
        onClick={renaming ? undefined : onToggleExpand}
      >
        <div className="flex flex-1 min-w-0 items-center gap-2">
          {renaming ? (
            <Input
              autoFocus
              value={renameDraft}
              onChange={(e) => onRenameDraftChange(e.target.value)}
              onBlur={onCommitRename}
              onKeyDown={(e) => {
                if (e.key === "Enter") onCommitRename();
                if (e.key === "Escape") onCommitRename();
              }}
              className="h-7 text-sm font-semibold w-48"
              onClick={(e) => e.stopPropagation()}
            />
          ) : (
            <span className="text-sm font-semibold truncate">{set.name}</span>
          )}
          <span className="text-xs text-muted-foreground shrink-0">
            {count} {count === 1 ? "entry" : "entries"}
          </span>
        </div>

        <div
          className="flex items-center gap-0.5 shrink-0 opacity-65 group-hover:opacity-100 transition-opacity"
          onClick={(e) => e.stopPropagation()}
        >
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                variant="ghost"
                size="icon-sm"
                aria-label="Rename set"
                onClick={onStartRename}
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
                aria-label="Delete set"
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
        <div className="border-x border-b border-border group-hover/row:border-ring/55 transition-[border-color] duration-150 rounded-b-lg px-3 py-3 bg-card">
          <TermChipInput value={set.entries} onChange={onEntriesChange} />
        </div>
      )}
    </div>
  );
}

function NewSetRow({
  value,
  onChange,
  onSubmit,
  creating,
  inputRef,
}: {
  value: string;
  onChange: (v: string) => void;
  onSubmit: () => void;
  creating: boolean;
  inputRef: React.RefObject<HTMLInputElement | null>;
}) {
  return (
    <div className="flex items-center gap-2">
      <Input
        ref={inputRef}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") onSubmit();
        }}
        placeholder="New set name"
        className="max-w-xs h-8 text-sm"
      />
      <Button size="sm" onClick={onSubmit} disabled={creating || !value.trim()}>
        {creating ? "Creating…" : "Create set"}
      </Button>
    </div>
  );
}

function DeleteConfirmDialog({
  target,
  onConfirm,
  onCancel,
}: {
  target: DeleteTarget | null;
  onConfirm: () => void;
  onCancel: () => void;
}) {
  if (!target) return null;
  const { set, affectedModeNames } = target;

  return (
    <Dialog open onOpenChange={(open) => !open && onCancel()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Delete &ldquo;{set.name}&rdquo;?</DialogTitle>
          {affectedModeNames.length > 0 && (
            <DialogDescription>
              This set is used by{" "}
              {affectedModeNames.length === 1
                ? `the profile "${affectedModeNames[0]}"`
                : `${affectedModeNames.length} profiles: ${affectedModeNames.join(", ")}`}
              . Deleting it will unlink it from{" "}
              {affectedModeNames.length === 1
                ? "that profile"
                : "those profiles"}
              .
            </DialogDescription>
          )}
        </DialogHeader>
        <DialogFooter>
          <Button variant="ghost" onClick={onCancel}>
            Cancel
          </Button>
          <Button variant="destructive" onClick={onConfirm}>
            Delete
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
