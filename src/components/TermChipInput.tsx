import { X } from "@phosphor-icons/react";
import { useRef, useState } from "react";

import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";

interface TermChipInputProps {
  value: string[];
  onChange: (terms: string[]) => void;
}

export function TermChipInput({ value, onChange }: TermChipInputProps) {
  const [inputValue, setInputValue] = useState("");
  const [pasteMode, setPasteMode] = useState(false);
  const [pasteText, setPasteText] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);

  const addTerm = (raw: string) => {
    const term = raw.trim();
    if (term && !value.includes(term)) {
      onChange([...value, term]);
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
      const parts = inputValue.split(",").map((s) => s.trim()).filter(Boolean);
      parts.forEach(addTerm);
      setInputValue("");
    } else if (e.key === "Backspace" && inputValue === "") {
      removeLast();
    }
  };

  const handleBlur = () => {
    const parts = inputValue.split(",").map((s) => s.trim()).filter(Boolean);
    parts.forEach(addTerm);
    setInputValue("");
  };

  const commitPaste = () => {
    const parts = pasteText
      .split(/[\n,]+/)
      .map((s) => s.trim())
      .filter(Boolean);
    parts.forEach(addTerm);
    setPasteText("");
    setPasteMode(false);
  };

  return (
    <div className="flex flex-col gap-2">
      <div
        className="min-h-[40px] flex flex-wrap gap-1 items-center p-2 border rounded-md bg-background cursor-text focus-within:ring-1 focus-within:ring-ring"
        onClick={() => inputRef.current?.focus()}
      >
        {value.map((term) => (
          <span
            key={term}
            className="inline-flex items-center gap-0.5 px-2 py-0.5 rounded-md bg-secondary text-secondary-foreground text-[12px] leading-snug"
          >
            {term}
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                onChange(value.filter((t) => t !== term));
              }}
              className="ml-0.5 text-muted-foreground hover:text-foreground transition-colors"
              aria-label={`Remove ${term}`}
            >
              <X size={10} weight="bold" />
            </button>
          </span>
        ))}
        <input
          ref={inputRef}
          value={inputValue}
          onChange={(e) => setInputValue(e.target.value)}
          onKeyDown={handleKeyDown}
          onBlur={handleBlur}
          placeholder={value.length === 0 ? "Type a term and press Enter…" : ""}
          className="flex-1 min-w-[120px] bg-transparent outline-none text-[13px] placeholder:text-muted-foreground/60"
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
            className="text-[13px] resize-none min-h-[80px]"
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
          className="text-[11px] text-muted-foreground hover:text-foreground transition-colors w-fit"
        >
          Paste list…
        </button>
      )}
    </div>
  );
}
