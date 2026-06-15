import { useRef, useState } from "react";

import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";

import { Chip } from "./Chip";

interface TermChipInputProps {
  value: string[];
  onChange: (terms: string[]) => void;
}

export function TermChipInput({ value, onChange }: TermChipInputProps) {
  const [inputValue, setInputValue] = useState("");
  const [pasteMode, setPasteMode] = useState(false);
  const [pasteText, setPasteText] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);

  const addTerms = (raws: string[]) => {
    const next = [...value];
    const seen = new Set(next);
    for (const raw of raws) {
      const term = raw.trim();
      if (term && !seen.has(term)) {
        next.push(term);
        seen.add(term);
      }
    }
    if (next.length !== value.length) {
      onChange(next);
    }
  };

  const removeLast = () => {
    if (value.length > 0) {
      onChange(value.slice(0, -1));
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter" || e.key === ",") {
      e.preventDefault();
      addTerms(inputValue.split(","));
      setInputValue("");
    } else if (e.key === "Backspace" && inputValue === "") {
      removeLast();
    }
  };

  const handleBlur = () => {
    addTerms(inputValue.split(","));
    setInputValue("");
  };

  const commitPaste = () => {
    addTerms(pasteText.split(/[\n,]+/));
    setPasteText("");
    setPasteMode(false);
  };

  return (
    <div className="flex flex-col gap-2">
      <div
        className="min-h-[40px] flex flex-wrap gap-1 items-center p-2 rounded-lg bg-card border border-border shadow-xs cursor-text outline-none transition-[color,box-shadow,border-color] has-[input:focus-visible]:border-ring has-[input:focus-visible]:ring-[3px] has-[input:focus-visible]:ring-ring/50"
        onClick={() => inputRef.current?.focus()}
      >
        {value.map((term) => (
          <Chip
            key={term}
            label={term}
            removeLabel={`Remove ${term}`}
            onRemove={() => onChange(value.filter((t) => t !== term))}
          />
        ))}
        <input
          ref={inputRef}
          value={inputValue}
          onChange={(e) => setInputValue(e.target.value)}
          onKeyDown={handleKeyDown}
          onBlur={handleBlur}
          placeholder={value.length === 0 ? "Type a term and press Enter…" : ""}
          className="flex-1 min-w-[120px] bg-transparent outline-none text-md placeholder:text-muted-foreground"
          spellCheck={false}
          autoComplete="off"
        />
      </div>

      {pasteMode ? (
        <div className="flex flex-col gap-1.5">
          <Textarea
            value={pasteText}
            onChange={(e) => setPasteText(e.target.value)}
            placeholder="Paste a list — one term per line, or comma-separated."
            className="resize-none min-h-[80px] rounded-lg bg-card border-border dark:bg-card"
            autoFocus
            spellCheck={false}
          />
          <div className="flex gap-1.5">
            <Button type="button" size="sm" onClick={commitPaste}>
              Add terms
            </Button>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              onClick={() => {
                setPasteMode(false);
                setPasteText("");
              }}
            >
              Cancel
            </Button>
          </div>
        </div>
      ) : (
        <button
          type="button"
          onClick={() => setPasteMode(true)}
          className="text-help text-muted-foreground hover:text-foreground transition-colors w-fit rounded-sm outline-none focus-visible:ring-[3px] focus-visible:ring-ring/50"
        >
          Paste list…
        </button>
      )}
    </div>
  );
}
