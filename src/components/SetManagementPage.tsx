import { PencilSimpleIcon, TrashIcon } from "@phosphor-icons/react";
import { useEffect, useRef, useState } from "react";
import { toast } from "sonner";

import { EmptyRowCard } from "@/components/EmptyRowCard";
import { ListRow, RowActionButton } from "@/components/ListRow";
import { ListSurface } from "@/components/ListSurface";
import { RowCard } from "@/components/RowCard";
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

export interface GenericSet {
  id: string;
  name: string;
  entryCount: number;
}

interface DeleteTarget {
  set: GenericSet;
  affectedModeNames: string[];
}

const SEARCH_THRESHOLD = 8;

export interface SetManagementPageProps {
  title: string;
  trailing?: string;
  description: string;
  emptyPreview: React.ReactNode;
  emptyHint: string;
  emptyAction: string;
  sets: GenericSet[];
  renderEntryBadge: (count: number) => React.ReactNode;
  getAffectedModeNames: (setId: string) => string[];
  onCreateSet: (name: string) => Promise<string | null>;
  onRenameSet: (id: string, name: string) => Promise<void>;
  onDeleteSet: (id: string) => Promise<void>;
  renderEntriesEditor: (setId: string) => React.ReactNode;
  errorMessages: {
    create: string;
    rename: string;
    delete: string;
  };
}

export function SetManagementPage({
  title,
  trailing,
  description,
  emptyPreview,
  emptyHint,
  emptyAction,
  sets,
  renderEntryBadge,
  getAffectedModeNames,
  onCreateSet,
  onRenameSet,
  onDeleteSet,
  renderEntriesEditor,
  errorMessages,
}: SetManagementPageProps) {
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<DeleteTarget | null>(null);
  const [creatingName, setCreatingName] = useState<string | null>(null);
  const [query, setQuery] = useState("");

  async function handleCreate() {
    if (creatingName === null || !creatingName.trim()) return;
    try {
      const newId = await onCreateSet(creatingName.trim());
      setCreatingName(null);
      setExpandedId(newId);
    } catch (e) {
      toast.error(errorMessages.create, { description: String(e) });
    }
  }

  async function handleRename(id: string, name: string) {
    try {
      await onRenameSet(id, name);
    } catch (e) {
      toast.error(errorMessages.rename, { description: String(e) });
    }
  }

  function openDeleteDialog(set: GenericSet) {
    setDeleteTarget({ set, affectedModeNames: getAffectedModeNames(set.id) });
  }

  async function confirmDelete() {
    if (!deleteTarget) return;
    const { set } = deleteTarget;
    setDeleteTarget(null);
    try {
      await onDeleteSet(set.id);
      if (expandedId === set.id) setExpandedId(null);
    } catch (e) {
      toast.error(errorMessages.delete, { description: String(e) });
    }
  }

  const normalizedQuery = query.trim().toLowerCase();
  const visibleSets = normalizedQuery
    ? sets.filter((s) => s.name.toLowerCase().includes(normalizedQuery))
    : sets;

  const isEmpty = sets.length === 0 && creatingName === null;

  return (
    <ListSurface
      title={title}
      description={description}
      count={trailing}
      search={
        sets.length > SEARCH_THRESHOLD
          ? {
              value: query,
              onChange: setQuery,
              placeholder: `Search ${title.toLowerCase()}…`,
            }
          : undefined
      }
    >
      <div className="flex flex-col gap-2">
        {isEmpty ? (
          <EmptyRowCard
            preview={emptyPreview}
            hint={emptyHint}
            action={emptyAction}
            onClick={() => setCreatingName("")}
          />
        ) : (
          <>
            {visibleSets.map((set) => (
              <SetRow
                key={set.id}
                set={set}
                expanded={expandedId === set.id}
                renderEntryBadge={renderEntryBadge}
                onToggleExpand={() =>
                  setExpandedId((id) => (id === set.id ? null : set.id))
                }
                onRename={(name) => handleRename(set.id, name)}
                onDelete={() => openDeleteDialog(set)}
                renderEntriesEditor={renderEntriesEditor}
              />
            ))}

            {normalizedQuery && visibleSets.length === 0 && (
              <p className="px-1 py-2 text-xs text-muted-foreground">
                No sets match “{query}”.
              </p>
            )}

            {creatingName !== null ? (
              <InlineCreateRow
                name={creatingName}
                onChange={setCreatingName}
                onSave={handleCreate}
                onCancel={() => setCreatingName(null)}
              />
            ) : (
              !normalizedQuery && (
                <EmptyRowCard
                  preview={
                    <span className="font-mono text-[13px] font-semibold text-muted-foreground/55">
                      New set
                    </span>
                  }
                  action={emptyAction}
                  onClick={() => setCreatingName("")}
                  className="py-4"
                />
              )
            )}
          </>
        )}
      </div>

      <DeleteConfirmDialog
        target={deleteTarget}
        onConfirm={confirmDelete}
        onCancel={() => setDeleteTarget(null)}
      />
    </ListSurface>
  );
}

function SetRow({
  set,
  expanded,
  renderEntryBadge,
  onToggleExpand,
  onRename,
  onDelete,
  renderEntriesEditor,
}: {
  set: GenericSet;
  expanded: boolean;
  renderEntryBadge: (count: number) => React.ReactNode;
  onToggleExpand: () => void;
  onRename: (name: string) => Promise<void>;
  onDelete: () => void;
  renderEntriesEditor: (setId: string) => React.ReactNode;
}) {
  const [renaming, setRenaming] = useState(false);
  const [draftName, setDraftName] = useState(set.name);
  const renameRef = useRef<HTMLInputElement>(null);
  const escapePressedRef = useRef(false);

  useEffect(() => {
    if (renaming) renameRef.current?.focus();
  }, [renaming]);

  function startRename() {
    setDraftName(set.name);
    setRenaming(true);
  }

  async function commitRename() {
    if (escapePressedRef.current) {
      escapePressedRef.current = false;
      return;
    }
    const trimmed = draftName.trim();
    setRenaming(false);
    if (trimmed && trimmed !== set.name) await onRename(trimmed);
  }

  function revertRename() {
    escapePressedRef.current = true;
    setDraftName(set.name);
    setRenaming(false);
  }

  return (
    <ListRow
      expanded={expanded}
      label={
        renaming ? (
          <Input
            ref={renameRef}
            value={draftName}
            onChange={(e) => setDraftName(e.target.value)}
            onBlur={commitRename}
            onKeyDown={(e) => {
              if (e.key === "Enter") commitRename();
              if (e.key === "Escape") revertRename();
            }}
            className="h-7 text-sm font-semibold w-48"
          />
        ) : (
          <span className="text-sm font-semibold truncate">{set.name}</span>
        )
      }
      meta={!renaming && renderEntryBadge(set.entryCount)}
      actions={
        <>
          <Button
            variant="ghost"
            size="sm"
            className="h-7 px-2 text-[12px] text-muted-foreground hover:text-foreground"
            onClick={onToggleExpand}
          >
            {expanded ? "Close" : "Open"}
          </Button>
          <RowActionButton
            icon={<PencilSimpleIcon size={14} />}
            label="Rename set"
            onClick={startRename}
          />
          <RowActionButton
            icon={<TrashIcon size={14} />}
            label="Delete set"
            tone="destructive"
            onClick={onDelete}
          />
        </>
      }
      below={expanded ? renderEntriesEditor(set.id) : null}
    />
  );
}

function InlineCreateRow({
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
  useEffect(() => {
    ref.current?.focus();
  }, []);

  return (
    <RowCard
      tone="accent"
      interactive={false}
      className="shadow-sm ring-2 ring-ring/15 gap-2 py-2.5"
    >
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
        <Button variant="ghost" size="xs" onClick={onCancel}>
          Cancel
        </Button>
        <Button size="xs" onClick={onSave} disabled={!name.trim()}>
          Create
        </Button>
      </div>
    </RowCard>
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
