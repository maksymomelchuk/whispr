import { SetManagementPage } from "@/components/SetManagementPage";
import { Badge } from "@/components/ui/badge";

import { useSettings } from "../context/SettingsContext";
import {
  createCorrectionSet,
  deleteCorrectionSet,
  renameCorrectionSet,
  updateCorrectionSetEntries,
} from "../lib/api";
import { toastRetry } from "../lib/toastRetry";
import type { CorrectionEntry } from "../lib/types";
import { EntriesEditor } from "./CorrectionEntriesEditor";

export function CorrectionsPage() {
  const { settings, setSettings } = useSettings();

  const correctionSets = settings.correction_sets ?? [];

  const sets = correctionSets.map((s) => ({
    id: s.id,
    name: s.name,
    entryCount: s.entries.length,
  }));

  async function handleCreate(name: string): Promise<string | null> {
    const updated = await createCorrectionSet(name);
    const newId =
      updated.correction_sets[updated.correction_sets.length - 1]?.id ?? null;
    setSettings(() => updated);
    return newId;
  }

  async function handleRename(id: string, name: string): Promise<void> {
    const updated = await renameCorrectionSet(id, name);
    setSettings(() => updated);
  }

  async function handleDelete(id: string): Promise<void> {
    const updated = await deleteCorrectionSet(id);
    setSettings(() => updated);
  }

  async function handleEntriesChange(
    id: string,
    entries: CorrectionEntry[],
  ): Promise<void> {
    try {
      const updated = await updateCorrectionSetEntries(id, entries);
      setSettings(() => updated);
    } catch (e) {
      toastRetry(
        "Couldn't save entries",
        () =>
          updateCorrectionSetEntries(id, entries).then((updated) =>
            setSettings(() => updated),
          ),
        String(e),
      );
    }
  }

  return (
    <SetManagementPage
      title="Corrections"
      trailing={
        correctionSets.length > 0
          ? `${correctionSets.length} ${correctionSets.length === 1 ? "set" : "sets"}`
          : undefined
      }
      description="Group find-and-replace rules into named sets. Profiles can apply any combination."
      emptyPreview={
        <span className="font-mono text-[13px] font-semibold text-muted-foreground/70">
          spoken → text
        </span>
      }
      emptyHint="Group find-and-replace rules into named sets. Profiles can apply any combination."
      emptyAction="New correction set"
      sets={sets}
      renderEntryBadge={(count) =>
        count > 0 ? (
          <Badge variant="neutral" className="text-[10px] shrink-0">
            {count} {count === 1 ? "rule" : "rules"}
          </Badge>
        ) : null
      }
      getAffectedModeNames={(setId) =>
        settings.modes
          .filter((m) => m.correction_set_ids.includes(setId))
          .map((m) => m.name)
      }
      onCreateSet={handleCreate}
      onRenameSet={handleRename}
      onDeleteSet={handleDelete}
      renderEntriesEditor={(setId) => {
        const set = correctionSets.find((s) => s.id === setId);
        if (!set) return null;
        return (
          <EntriesEditor
            entries={set.entries}
            onChange={(entries) => handleEntriesChange(setId, entries)}
          />
        );
      }}
      errorMessages={{
        create: "Couldn't create correction set",
        rename: "Couldn't rename correction set",
        delete: "Couldn't delete correction set",
      }}
    />
  );
}
