import { CheckIcon } from "@phosphor-icons/react";
import type { ComponentProps } from "react";
import Markdown, { type Components } from "react-markdown";
import remarkGfm from "remark-gfm";

import { cn } from "@/lib/utils";

const LEADING_TITLE_HEADING = /^\s*# [^\n]*\n?/;

function TaskCheckbox({ checked }: ComponentProps<"input">) {
  return (
    <span
      role="checkbox"
      aria-checked={Boolean(checked)}
      aria-readonly="true"
      className={cn(
        "mr-2 inline-flex size-4 shrink-0 translate-y-[3px] items-center justify-center rounded-[4px] border border-input bg-background align-top",
        checked && "border-primary bg-primary text-primary-foreground",
      )}
    >
      {checked && <CheckIcon weight="bold" className="size-3" />}
    </span>
  );
}

const NOTES_COMPONENTS: Components = {
  h1: ({ children }) => (
    <h2 className="mt-2 text-base font-semibold first:mt-0">{children}</h2>
  ),
  h2: ({ children }) => (
    <h3 className="mt-5 mb-1.5 text-[13px] font-semibold uppercase tracking-wide text-muted-foreground first:mt-0">
      {children}
    </h3>
  ),
  h3: ({ children }) => (
    <h4 className="mt-4 mb-1 text-sm font-semibold first:mt-0">{children}</h4>
  ),
  p: ({ children }) => <p className="my-1.5">{children}</p>,
  ul: ({ className, children }) => (
    <ul
      className={cn(
        "my-1.5 flex flex-col gap-1 pl-5",
        className?.includes("contains-task-list")
          ? "list-none pl-0"
          : "list-disc marker:text-muted-foreground",
      )}
    >
      {children}
    </ul>
  ),
  ol: ({ children }) => (
    <ol className="my-1.5 flex list-decimal flex-col gap-1 pl-5 marker:text-muted-foreground">
      {children}
    </ol>
  ),
  li: ({ className, children }) => (
    <li
      className={
        className?.includes("task-list-item") ? "flex items-start" : "pl-0.5"
      }
    >
      {children}
    </li>
  ),
  input: TaskCheckbox,
  strong: ({ children }) => (
    <strong className="font-semibold">{children}</strong>
  ),
  code: ({ children }) => (
    <code className="rounded bg-muted px-1 py-0.5 font-mono text-xs">
      {children}
    </code>
  ),
  // A plain anchor would navigate the app's own webview away from the UI.
  a: ({ children }) => <span className="underline">{children}</span>,
};

export function MeetingNotes({
  notes,
  hasTitle,
}: {
  notes: string;
  hasTitle: boolean;
}) {
  const body = hasTitle ? notes.replace(LEADING_TITLE_HEADING, "") : notes;
  return (
    <div className="text-sm leading-relaxed">
      <Markdown remarkPlugins={[remarkGfm]} components={NOTES_COMPONENTS}>
        {body}
      </Markdown>
    </div>
  );
}
