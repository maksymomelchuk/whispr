import { ArrowUpIcon, TrashIcon } from "@phosphor-icons/react";
import { useEffect, useState } from "react";
import { toast } from "sonner";

import { EmptyPanel } from "@/components/EmptyPanel";
import { RowCard } from "@/components/RowCard";
import { SectionHeader } from "@/components/SectionHeader";
import { ToggleRow } from "@/components/ToggleRow";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";

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

  useEffect(() => {
    getLearnedEntries()
      .then(setEntries)
      .catch(() => setEntries([]))
      .finally(() => setLoading(false));
  }, []);

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
      toast.success("Entry promoted to permanent dictionary");
    } catch (e) {
      toast.error("Couldn't promote entry", { description: String(e) });
    }
  };

  const candidateEntries = entries.filter((e) => e.status === "candidate");
  const promotedEntries = entries.filter((e) => e.status === "promoted");

  return (
    <div className="p-6 flex flex-col gap-8">
      <SectionHeader title="Auto-Learning" />

      <div className="flex flex-col gap-3">
        <div className="bg-card border border-border rounded-lg px-3 py-3">
          <ToggleRow
            id="learn-from-corrections"
            label="Learn from my corrections"
            info="When on, edits you make to History entries are analysed and vocabulary corrections are suggested automatically."
            checked={settings.learn_from_corrections}
            onCheckedChange={handleToggleLearning}
          />
        </div>
      </div>

      {settings.learn_from_corrections && (
        <>
          {!loading && entries.length === 0 ? (
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
                  <div className="flex flex-col gap-1.5">
                    {promotedEntries.map((entry) => (
                      <LearnedEntryRow
                        key={entry.id}
                        entry={entry}
                        onDelete={() => handleDelete(entry.id)}
                      />
                    ))}
                  </div>
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
                  <div className="flex flex-col gap-1.5">
                    {candidateEntries.map((entry) => (
                      <LearnedEntryRow
                        key={entry.id}
                        entry={entry}
                        onDelete={() => handleDelete(entry.id)}
                        onPromote={() => handlePromote(entry.id)}
                      />
                    ))}
                  </div>
                </div>
              )}
            </>
          )}
        </>
      )}
    </div>
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
  const isCorrection = entry.kind === "correction";

  return (
    <RowCard>
      <div className="flex flex-1 min-w-0 items-center gap-2">
        <Badge
          variant="neutral"
          className="text-[10px] shrink-0 font-mono uppercase"
        >
          {isCorrection ? "correction" : "term"}
        </Badge>
        <Badge
          variant={entry.status === "promoted" ? "accent" : "neutral"}
          className="text-[10px] shrink-0"
        >
          {entry.status === "promoted" ? "active" : "candidate"}
        </Badge>
        {isCorrection && entry.from ? (
          <span className="text-sm font-mono text-muted-foreground truncate">
            {entry.from}
            <span className="mx-1 text-muted-foreground/50">→</span>
            <span className="text-foreground font-semibold">{entry.word}</span>
          </span>
        ) : (
          <span className="text-sm font-mono text-foreground font-semibold truncate">
            {entry.word}
          </span>
        )}
        <span className="ml-auto text-[11px] text-muted-foreground/60 shrink-0 tabular-nums">
          {entry.total_observations}×
        </span>
      </div>

      <div className="flex items-center gap-0.5 shrink-0">
        {onPromote && (
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                variant="ghost"
                size="icon-sm"
                aria-label="Promote to permanent entry"
                onClick={onPromote}
              >
                <ArrowUpIcon size={14} />
              </Button>
            </TooltipTrigger>
            <TooltipContent>Promote to permanent entry</TooltipContent>
          </Tooltip>
        )}
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant="ghost"
              size="icon-sm"
              aria-label="Delete"
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
  );
}
