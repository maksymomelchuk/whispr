import { TrashIcon } from "@phosphor-icons/react";
import { useEffect, useRef, useState } from "react";
import { toast } from "sonner";

import { EmptyRowCard } from "@/components/EmptyRowCard";
import { RowCard } from "@/components/RowCard";
import { SectionHeader } from "@/components/SectionHeader";
import { TermChipInput } from "@/components/TermChipInput";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";

import { useSettings } from "../context/SettingsContext";
import { useFlash } from "../hooks/useFlash";
import {
  setCorrections as persistCorrections,
  setTerms as persistTerms,
} from "../lib/api";
import type { CorrectionEntry } from "../lib/types";

type Tab = "terms" | "corrections";

export function DictionaryPage() {
  const { settings, setSettings } = useSettings();
  const [activeTab, setActiveTab] = useState<Tab>("terms");

  if (!settings) return null;

  const termsCount = settings.terms?.length ?? 0;
  const correctionsCount = settings.corrections?.length ?? 0;

  const trailing =
    activeTab === "terms"
      ? termsCount > 0
        ? `${termsCount} ${termsCount === 1 ? "term" : "terms"}`
        : undefined
      : correctionsCount > 0
        ? `${correctionsCount} ${correctionsCount === 1 ? "entry" : "entries"}`
        : undefined;

  return (
    <div className="p-6 flex flex-col gap-8">
      <SectionHeader title="Dictionary" trailing={trailing} />

      <ToggleGroup
        type="single"
        variant="outline"
        value={activeTab}
        onValueChange={(v) => {
          if (v === "terms" || v === "corrections") setActiveTab(v);
        }}
        className="w-fit"
      >
        <ToggleGroupItem value="terms" className="px-4 text-xs">
          Terms
        </ToggleGroupItem>
        <ToggleGroupItem value="corrections" className="px-4 text-xs">
          Corrections
        </ToggleGroupItem>
      </ToggleGroup>

      {activeTab === "terms" ? (
        <TermsTab
          initial={settings.terms ?? []}
          onSaved={(saved) =>
            setSettings((s) => (s ? { ...s, terms: saved } : s))
          }
        />
      ) : (
        <CorrectionsTab
          corrections={settings.corrections ?? []}
          onPersist={(next) =>
            setSettings((s) => (s ? { ...s, corrections: next } : s))
          }
        />
      )}
    </div>
  );
}

function TermsTab({
  initial,
  onSaved,
}: {
  initial: string[];
  onSaved: (saved: string[]) => void;
}) {
  const [terms, setTerms] = useState(initial);
  // Guard against out-of-order persist completions.
  const requestIdRef = useRef(0);

  const handleChange = async (next: string[]) => {
    setTerms(next);
    const id = ++requestIdRef.current;
    try {
      await persistTerms(next);
      if (id !== requestIdRef.current) return;
      onSaved(next);
    } catch (e) {
      if (id !== requestIdRef.current) return;
      toast.error("Couldn't save terms", { description: String(e) });
    }
  };

  return (
    <div className="flex flex-col gap-2.5">
      <p className="text-[12px] text-muted-foreground/85 max-w-prose">
        Vocabulary hints sent to the recognizer so it picks your exact spelling.
        Terms bias transcription — no find-and-replace happens here.
      </p>
      <TermChipInput value={terms} onChange={handleChange} />
    </div>
  );
}

type Draft = { from: string; to: string };

type EditingState =
  | { kind: "none" }
  | { kind: "edit"; index: number; draft: Draft }
  | { kind: "new"; draft: Draft };

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
          {entry.from || (
            <span className="text-muted-foreground/60 italic">(empty)</span>
          )}
        </span>
        <span className="text-muted-foreground/60 text-help shrink-0">→</span>
        <span className="flex-1 truncate text-xs text-muted-foreground">
          {entry.to || (
            <span className="italic text-muted-foreground/60">(empty)</span>
          )}
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
              aria-label="Delete correction"
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

function EditorRow({
  draft,
  onChange,
  onSave,
  onCancel,
  saving,
  error,
}: {
  draft: Draft;
  onChange: (next: Draft) => void;
  onSave: () => void;
  onCancel: () => void;
  saving: boolean;
  error: string | null;
}) {
  const fromRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    fromRef.current?.focus();
  }, []);

  function handleKeyDown(e: React.KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      onCancel();
    }
    if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
      e.preventDefault();
      onSave();
    }
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
        <span className="select-none text-muted-foreground/60 text-[12px]">
          →
        </span>
        <Input
          value={draft.to}
          onChange={(e) => onChange({ ...draft, to: e.target.value })}
          placeholder="text"
          spellCheck={false}
          autoComplete="off"
          className="text-xs h-8 flex-1 min-w-0"
          onKeyDown={(e) => {
            if (e.key === "Enter" && !e.metaKey && !e.ctrlKey) {
              e.preventDefault();
              onSave();
            }
          }}
        />
        <div className="flex items-center gap-1 ml-1">
          <Button
            variant="ghost"
            size="xs"
            onClick={onCancel}
            disabled={saving}
          >
            Cancel
          </Button>
          <Button
            size="xs"
            onClick={onSave}
            disabled={saving || !draft.from.trim()}
          >
            {saving ? "Saving…" : "Save"}
          </Button>
        </div>
      </div>
      {error && <p className="text-help text-destructive px-0.5">{error}</p>}
    </RowCard>
  );
}

function EmptyState({ onAdd }: { onAdd: () => void }) {
  return (
    <EmptyRowCard
      preview={
        <span className="font-mono text-[13px] font-semibold text-muted-foreground/70">
          spoken → text
        </span>
      }
      hint="Rewrite phrases in the transcript after cleanup and snippets run."
      action="Add correction"
      onClick={onAdd}
    />
  );
}

function CorrectionsTab({
  corrections,
  onPersist,
}: {
  corrections: CorrectionEntry[];
  onPersist: (next: CorrectionEntry[]) => void;
}) {
  const [editing, setEditing] = useState<EditingState>({ kind: "none" });
  const [saveError, setSaveError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const { flash, isFlashing } = useFlash();

  function startNew() {
    setEditing({ kind: "new", draft: { from: "", to: "" } });
    setSaveError(null);
  }

  function startEdit(index: number) {
    setEditing({ kind: "edit", index, draft: { ...corrections[index] } });
    setSaveError(null);
  }

  function cancelEdit() {
    setEditing({ kind: "none" });
    setSaveError(null);
  }

  async function persist(next: CorrectionEntry[]): Promise<boolean> {
    setSaving(true);
    try {
      await persistCorrections(next);
      onPersist(next);
      return true;
    } catch (e) {
      setSaveError(String(e));
      return false;
    } finally {
      setSaving(false);
    }
  }

  async function handleDelete(index: number) {
    await persist(corrections.filter((_, idx) => idx !== index));
  }

  async function handleSave() {
    if (editing.kind === "none") return;
    const from = editing.draft.from.trim();
    if (!from) {
      setSaveError("Spoken form cannot be empty.");
      return;
    }
    const entry: CorrectionEntry = { from, to: editing.draft.to };
    let next: CorrectionEntry[];
    let flashKey: string;
    if (editing.kind === "new") {
      next = [...corrections, entry];
      flashKey = `c-${next.length - 1}`;
    } else {
      next = corrections.map((c, idx) => (idx === editing.index ? entry : c));
      flashKey = `c-${editing.index}`;
    }
    setSaveError(null);
    if (await persist(next)) {
      setEditing({ kind: "none" });
      flash(flashKey);
    }
  }

  const editingNew = editing.kind === "new";
  const editingIndex = editing.kind === "edit" ? editing.index : null;
  const showTopLevelError = saveError !== null && editing.kind === "none";

  return (
    <div className="flex flex-col gap-2.5">
      <p className="text-[12px] text-muted-foreground/85 max-w-prose">
        Find-and-replace rules applied to the transcript after AI cleanup and
        snippets. The spoken form on the left never reaches the STT engine —
        only real vocabulary hints go there (see Terms).
      </p>

      {corrections.length === 0 && !editingNew ? (
        <EmptyState onAdd={startNew} />
      ) : (
        <div className="flex flex-col gap-2">
          {corrections.map((entry, i) =>
            editingIndex === i && editing.kind === "edit" ? (
              <EditorRow
                key={`edit-${i}`}
                draft={editing.draft}
                onChange={(draft) =>
                  setEditing({ kind: "edit", index: i, draft })
                }
                onSave={handleSave}
                onCancel={cancelEdit}
                saving={saving}
                error={saveError}
              />
            ) : (
              <CorrectionRow
                key={`c-${i}`}
                entry={entry}
                flashing={isFlashing(`c-${i}`)}
                onEdit={() => startEdit(i)}
                onDelete={() => handleDelete(i)}
              />
            ),
          )}

          {editingNew && editing.kind === "new" && (
            <EditorRow
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
              action="Add correction"
              onClick={startNew}
              className="py-4"
            />
          )}
        </div>
      )}

      {showTopLevelError && (
        <Alert variant="destructive">
          <AlertDescription>{saveError}</AlertDescription>
        </Alert>
      )}
    </div>
  );
}
