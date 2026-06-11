import { PencilSimpleIcon, TrashIcon } from "@phosphor-icons/react";
import { useEffect, useRef, useState } from "react";
import { toast } from "sonner";

import { EmptyRowCard } from "@/components/EmptyRowCard";
import { RowCard } from "@/components/RowCard";
import { SectionHeader } from "@/components/SectionHeader";
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

export interface GenericSet {
  id: string;
  name: string;
  entryCount: number;
}

interface DeleteTarget {
  set: GenericSet;
  affectedModeNames: string[];
}

export interface SetManagementPageProps {
  title: string;
  trailing?: string;
  description: string;
  emptyPreview: React.ReactNode;
  emptyHint: string;
  emptyAction: string;
  sets: GenericSet[];
  renderEntryBadge: (count: number) => React.ReactNode;
  expandVariant: "row-click" | "open-button";
  getAffectedModeNames: (setId: string) => string[];
  onCreateSet: (name: string) => Promise<string | null>;
  onRenameSet: (id: string, name: string) => Promise<void>;
  onDeleteSet: (id: string) => Promise<void>;
  renderEntriesEditor: (setId: string) => React.ReactNode;
  createVariant: "bottom-input" | "inline-card";
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
  expandVariant,
  getAffectedModeNames,
  onCreateSet,
  onRenameSet,
  onDeleteSet,
  renderEntriesEditor,
  createVariant,
  errorMessages,
}: SetManagementPageProps) {
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<DeleteTarget | null>(null);
  const [newName, setNewName] = useState("");
  const [creating, setCreating] = useState(false);
  const newInputRef = useRef<HTMLInputElement>(null);
  const [creatingName, setCreatingName] = useState<string | null>(null);

  async function handleCreateBottomInput() {
    const name = newName.trim();
    if (!name) return;
    setCreating(true);
    try {
      const newId = await onCreateSet(name);
      setNewName("");
      setExpandedId(newId);
    } catch (e) {
      toast.error(errorMessages.create, { description: String(e) });
    } finally {
      setCreating(false);
    }
  }

  async function handleCreateInlineCard() {
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

  const showEmptyState =
    sets.length === 0 &&
    (createVariant === "bottom-input" || creatingName === null);

  return (
    <div className="p-6 flex flex-col gap-8">
      <SectionHeader title={title} trailing={trailing} />

      <p className="text-[12px] text-muted-foreground/85 max-w-prose -mt-5">
        {description}
      </p>

      <div className="flex flex-col gap-2">
        {showEmptyState ? (
          <EmptyRowCard
            preview={emptyPreview}
            hint={emptyHint}
            action={emptyAction}
            onClick={
              createVariant === "bottom-input"
                ? () => newInputRef.current?.focus()
                : () => setCreatingName("")
            }
          />
        ) : (
          <>
            {sets.map((set) => (
              <SetRow
                key={set.id}
                set={set}
                expanded={expandedId === set.id}
                expandVariant={expandVariant}
                renderEntryBadge={renderEntryBadge}
                onToggleExpand={() =>
                  setExpandedId((id) => (id === set.id ? null : set.id))
                }
                onRename={(name) => handleRename(set.id, name)}
                onDelete={() => openDeleteDialog(set)}
                renderEntriesEditor={renderEntriesEditor}
              />
            ))}

            {createVariant === "inline-card" &&
              (creatingName !== null ? (
                <InlineCreateRow
                  name={creatingName}
                  onChange={setCreatingName}
                  onSave={handleCreateInlineCard}
                  onCancel={() => setCreatingName(null)}
                />
              ) : (
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
              ))}
          </>
        )}
      </div>

      {createVariant === "bottom-input" && (
        <BottomCreateRow
          value={newName}
          onChange={setNewName}
          onSubmit={handleCreateBottomInput}
          creating={creating}
          inputRef={newInputRef}
        />
      )}

      <DeleteConfirmDialog
        target={deleteTarget}
        onConfirm={confirmDelete}
        onCancel={() => setDeleteTarget(null)}
      />
    </div>
  );
}

function SetRow({
  set,
  expanded,
  expandVariant,
  renderEntryBadge,
  onToggleExpand,
  onRename,
  onDelete,
  renderEntriesEditor,
}: {
  set: GenericSet;
  expanded: boolean;
  expandVariant: "row-click" | "open-button";
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

  const clickable = !renaming && expandVariant === "row-click";

  return (
    <div className="flex flex-col gap-0 group/row">
      <RowCard
        interactive={clickable}
        className={
          expanded
            ? "rounded-b-none border-b-0 group-hover/row:border-ring/55"
            : ""
        }
        onClick={clickable ? onToggleExpand : undefined}
      >
        <div className="flex flex-1 min-w-0 items-center gap-2">
          {renaming ? (
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
              onClick={(e) => e.stopPropagation()}
            />
          ) : (
            <span className="text-sm font-semibold truncate">{set.name}</span>
          )}
          {renderEntryBadge(set.entryCount)}
        </div>

        <div
          className="flex items-center gap-0.5 shrink-0"
          onClick={(e) => e.stopPropagation()}
        >
          {expandVariant === "open-button" && (
            <Button
              variant="ghost"
              size="sm"
              className="h-7 px-2 text-[12px] text-muted-foreground hover:text-foreground"
              onClick={onToggleExpand}
            >
              {expanded ? "Close" : "Open"}
            </Button>
          )}
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                variant="ghost"
                size="icon-sm"
                aria-label="Rename set"
                onClick={startRename}
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
          {renderEntriesEditor(set.id)}
        </div>
      )}
    </div>
  );
}

function BottomCreateRow({
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
