import { toast } from "sonner";

import { SetManagementPage } from "@/components/SetManagementPage";
import { TermChipInput } from "@/components/TermChipInput";

import { useSettings } from "../context/SettingsContext";
import {
  createTermSet,
  deleteTermSet,
  renameTermSet,
  updateTermSetEntries,
} from "../lib/api";

export function TermsPage() {
  const { settings, setSettings } = useSettings();

  const termSets = settings.term_sets ?? [];

  const sets = termSets.map((s) => ({
    id: s.id,
    name: s.name,
    entryCount: s.entries.length,
  }));

  const totalEntries = termSets.reduce((n, ts) => n + ts.entries.length, 0);
  const trailing =
    termSets.length > 0
      ? `${termSets.length} ${termSets.length === 1 ? "set" : "sets"}, ${totalEntries} ${totalEntries === 1 ? "entry" : "entries"}`
      : undefined;

  async function handleCreate(name: string): Promise<string | null> {
    const updated = await createTermSet(name);
    const newId = updated.term_sets[updated.term_sets.length - 1]?.id ?? null;
    setSettings(() => updated);
    return newId;
  }

  async function handleRename(id: string, name: string): Promise<void> {
    const updated = await renameTermSet(id, name);
    setSettings(() => updated);
  }

  async function handleDelete(id: string): Promise<void> {
    const updated = await deleteTermSet(id);
    setSettings(() => updated);
  }

  async function handleEntriesChange(
    id: string,
    entries: string[],
  ): Promise<void> {
    try {
      const updated = await updateTermSetEntries(id, entries);
      setSettings(() => updated);
    } catch (e) {
      toast.error("Couldn't save entries", { description: String(e) });
    }
  }

  return (
    <SetManagementPage
      title="Vocabulary"
      trailing={trailing}
      description="Vocabulary hints sent to the recognizer so it picks your exact spelling. Organize terms into named sets and assign sets to profiles."
      emptyPreview={
        <span className="text-[13px] text-muted-foreground/70 font-semibold">
          No term sets yet
        </span>
      }
      emptyHint="Create a set and assign it to profiles to bias transcription."
      emptyAction="New term set"
      sets={sets}
      renderEntryBadge={(count) => (
        <span className="text-xs text-muted-foreground shrink-0">
          {count} {count === 1 ? "entry" : "entries"}
        </span>
      )}
      expandVariant="row-click"
      getAffectedModeNames={(setId) =>
        settings.modes
          .filter((m) => m.term_set_ids.includes(setId))
          .map((m) => m.name)
      }
      onCreateSet={handleCreate}
      onRenameSet={handleRename}
      onDeleteSet={handleDelete}
      renderEntriesEditor={(setId) => {
        const set = termSets.find((s) => s.id === setId);
        if (!set) return null;
        return (
          <TermChipInput
            value={set.entries}
            onChange={(entries) => handleEntriesChange(setId, entries)}
          />
        );
      }}
      createVariant="bottom-input"
      errorMessages={{
        create: "Couldn't create term set",
        rename: "Couldn't rename term set",
        delete: "Couldn't delete term set",
      }}
    />
  );
}
