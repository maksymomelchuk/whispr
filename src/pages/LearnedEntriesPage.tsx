import { ArrowUpIcon, TrashIcon } from "@phosphor-icons/react";
import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useState } from "react";
import { toast } from "sonner";

import { EmptyPanel } from "@/components/EmptyPanel";
import { ListRow, RowActionButton } from "@/components/ListRow";
import { ListSurface } from "@/components/ListSurface";
import { RowCard } from "@/components/RowCard";
import { SectionHeader } from "@/components/SectionHeader";
import { ToggleRow } from "@/components/ToggleRow";
import { Badge } from "@/components/ui/badge";

import { useSettings } from "../context/SettingsContext";
import {
  deleteLearnedEntry,
  getLearnedEntries,
  promoteLearnedEntry,
  setLearnFromCorrections,
} from "../lib/api";
import type { LearnedEntry } from "../lib/types";

export function LearnedEntriesPage() {
  const { settings, setSetting } = useSettings();
  const [entries, setEntries] = useState<LearnedEntry[]>([]);
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(() => {
    getLearnedEntries()
      .then(setEntries)
      .catch(() => setEntries([]))
      .finally(() => setLoading(false));
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  useEffect(() => {
    let unlisten: (() => void) | null = null;
    listen("learned-updated", () => refresh())
      .then((u) => {
        unlisten = u;
      })
      .catch((e) => console.error("learned-updated listen failed", e));
    return () => {
      unlisten?.();
    };
  }, [refresh]);

  const handleToggleLearning = async (enabled: boolean) => {
    await setSetting(
      "learn_from_corrections",
      enabled,
      () => setLearnFromCorrections(enabled),
      (e) => toast.error("Couldn't update setting", { description: String(e) }),
    );
  };

  const handleDelete = async (id: string) => {
    try {
      await deleteLearnedEntry(id);
      setEntries((prev) => prev.filter((e) => e.id !== id));
    } catch (e) {
      toast.error("Couldn't delete entry", { description: String(e) });
    }
  };

  const handlePromote = async (id: string) => {
    try {
      await promoteLearnedEntry(id);
      setEntries((prev) =>
        prev.map((entry) =>
          entry.id === id ? { ...entry, status: "promoted" as const } : entry,
        ),
      );
      toast.success("Entry activated");
    } catch (e) {
      toast.error("Couldn't promote entry", { description: String(e) });
    }
  };

  const candidateEntries = entries.filter((e) => e.status === "candidate");
  const promotedEntries = entries.filter((e) => e.status === "promoted");

  return (
    <ListSurface
      title="Auto-Learn"
      description="Edits you make to History entries become vocabulary and correction suggestions. Promote the ones worth keeping."
    >
      <RowCard interactive={false}>
        <ToggleRow
          id="learn-from-corrections"
          label="Learn from my corrections"
          info="When on, edits you make to History entries are analysed and vocabulary corrections are suggested automatically."
          checked={settings.learn_from_corrections}
          onCheckedChange={handleToggleLearning}
          className="flex-1"
        />
      </RowCard>

      {settings.learn_from_corrections &&
        (!loading && entries.length === 0 ? (
          <EmptyPanel
            title="No learned entries yet"
            hint="Edit History entries to teach the app your corrections."
          />
        ) : (
          <>
            {promotedEntries.length > 0 && (
              <div className="flex flex-col gap-2">
                <SectionHeader
                  title="Ready to use"
                  trailing={`${promotedEntries.length}`}
                />
                {promotedEntries.map((entry) => (
                  <LearnedEntryRow
                    key={entry.id}
                    entry={entry}
                    onDelete={() => handleDelete(entry.id)}
                  />
                ))}
              </div>
            )}

            {candidateEntries.length > 0 && (
              <div className="flex flex-col gap-2">
                <SectionHeader
                  title="Candidates"
                  trailing={`${candidateEntries.length}`}
                />
                <p className="text-[12px] text-muted-foreground">
                  Seen once. Needs one more observation to activate.
                </p>
                {candidateEntries.map((entry) => (
                  <LearnedEntryRow
                    key={entry.id}
                    entry={entry}
                    onDelete={() => handleDelete(entry.id)}
                    onPromote={() => handlePromote(entry.id)}
                  />
                ))}
              </div>
            )}
          </>
        ))}
    </ListSurface>
  );
}

function LearnedEntryRow({
  entry,
  onDelete,
  onPromote,
}: {
  entry: LearnedEntry;
  onDelete: () => void;
  onPromote?: () => void;
}) {
  return (
    <ListRow
      label={<LearnedRowLabel entry={entry} />}
      meta={
        <span className="ml-auto shrink-0">
          {entry.status === "promoted" ? (
            <Badge variant="neutral" className="text-[10px]">
              Active
            </Badge>
          ) : (
            <span className="text-[11px] text-muted-foreground/60 tabular-nums">
              {entry.total_observations}×
            </span>
          )}
        </span>
      }
      actions={
        <>
          {onPromote && (
            <RowActionButton
              icon={<ArrowUpIcon size={14} />}
              label="Promote to permanent entry"
              onClick={onPromote}
            />
          )}
          <RowActionButton
            icon={<TrashIcon size={14} />}
            label={`Delete ${entry.word}`}
            tone="destructive"
            onClick={onDelete}
          />
        </>
      }
    />
  );
}

function LearnedRowLabel({ entry }: { entry: LearnedEntry }) {
  if (entry.kind === "correction" && entry.from) {
    return (
      <span className="text-sm font-mono text-muted-foreground truncate">
        {entry.from}
        <span className="mx-1 text-muted-foreground/50">→</span>
        <span className="text-foreground font-semibold">{entry.word}</span>
      </span>
    );
  }
  return (
    <span className="text-sm font-mono text-foreground font-semibold truncate">
      {entry.word}
    </span>
  );
}
