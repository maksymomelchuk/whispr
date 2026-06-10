import { PencilSimpleIcon, TrashIcon } from "@phosphor-icons/react";
import { useEffect, useRef, useState } from "react";
import { toast } from "sonner";

import { EmptyRowCard } from "@/components/EmptyRowCard";
import { RowCard } from "@/components/RowCard";
import { SectionHeader } from "@/components/SectionHeader";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";

import { useSettings } from "../context/SettingsContext";
import {
  createCorrectionSet,
  deleteCorrectionSet,
  renameCorrectionSet,
  updateCorrectionSetEntries,
} from "../lib/api";
import type { CorrectionEntry, Mode, NamedCorrectionSet } from "../lib/types";
import { EntriesEditor } from "./CorrectionEntriesEditor";

type DeleteConfirm = { set: NamedCorrectionSet; affectedModes: Mode[] };

export function CorrectionsPage() {
  const { settings, setSettings } = useSettings();
  const [expandedSetId, setExpandedSetId] = useState<string | null>(null);
  const [creatingName, setCreatingName] = useState<string | null>(null);
  const [deleteConfirm, setDeleteConfirm] = useState<DeleteConfirm | null>(
    null,
  );

  const correctionSets = settings.correction_sets ?? [];

  const handleCreateSave = async () => {
    if (creatingName === null || !creatingName.trim()) return;
    try {
      const updated = await createCorrectionSet(creatingName.trim());
      const newId = updated.correction_sets.at(-1)?.id ?? null;
      setSettings(() => updated);
      setCreatingName(null);
      setExpandedSetId(newId);
    } catch (e) {
      toast.error("Couldn't create correction set", { description: String(e) });
    }
  };

  const handleRename = async (setId: string, newName: string) => {
    try {
      const updated = await renameCorrectionSet(setId, newName);
      setSettings(() => updated);
    } catch (e) {
      toast.error("Couldn't rename correction set", { description: String(e) });
    }
  };

  const handleEntriesChange = async (
    setId: string,
    entries: CorrectionEntry[],
  ) => {
    try {
      const updated = await updateCorrectionSetEntries(setId, entries);
      setSettings(() => updated);
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
      const updated = await deleteCorrectionSet(set.id);
      setSettings(() => updated);
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
            hint="Group find-and-replace rules into named sets. Profiles can apply any combination."
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
                onEntriesChange={(entries) =>
                  handleEntriesChange(set.id, entries)
                }
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
    <div className="flex flex-col gap-0 group/row">
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
                if (e.key === "Escape") {
                  setDraftName(set.name);
                  setRenaming(false);
                }
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
                onClick={() => {
                  setDraftName(set.name);
                  setRenaming(true);
                }}
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
        <div className="border-x border-b border-border group-hover/row:border-ring/55 transition-[border-color] duration-150 rounded-b-md p-3 flex flex-col gap-2">
          <EntriesEditor entries={set.entries} onChange={onEntriesChange} />
        </div>
      )}
    </div>
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
      onClick={(e) => {
        if (e.target === e.currentTarget) onCancel();
      }}
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
              . It will be unlinked from{" "}
              {affectedModes.length === 1 ? "that profile" : "those profiles"}.
            </p>
          )}
        </div>
        <div className="flex justify-end gap-2">
          <Button variant="ghost" size="sm" onClick={onCancel}>
            Cancel
          </Button>
          <Button variant="destructive" size="sm" onClick={onConfirm}>
            Delete
          </Button>
        </div>
      </div>
    </div>
  );
}
