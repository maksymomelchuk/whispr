import { Toaster as SonnerToaster } from "sonner";

export function Toaster() {
  return (
    <SonnerToaster
      position="bottom-right"
      closeButton
      gap={10}
      // Wider than the default so multi-line messages don't get crammed into a
      // narrow column when an action button is present.
      style={
        {
          "--width": "420px",
        } as React.CSSProperties
      }
      toastOptions={{
        classNames: {
          toast: [
            "group toast font-sans",
            "rounded-xl border border-border/70 bg-background text-foreground",
            "shadow-[0_10px_30px_-15px_rgba(0,0,0,0.25)]",
            "p-4 gap-3 items-start",
          ].join(" "),
          title:
            "text-[13px] font-medium leading-snug text-foreground tracking-[-0.005em]",
          description: "text-xs leading-snug text-muted-foreground mt-0.5",
          // Sit inline at the end of the message, look like a tertiary pill —
          // not a giant solid block competing with the title.
          actionButton: [
            "shrink-0 self-center",
            "inline-flex items-center gap-1 h-7 px-2.5 rounded-md",
            "text-[12px] font-medium",
            "bg-foreground/[0.06] text-foreground border border-border/60",
            "hover:bg-foreground/10 transition-colors",
          ].join(" "),
          cancelButton:
            "h-7 px-2 rounded-md text-[12px] text-muted-foreground hover:text-foreground",
          error: [
            "bg-destructive/[0.08] border-destructive/30",
            "[&_[data-title]]:text-destructive",
            "[&_[data-description]]:text-destructive/80",
            "[&_[data-icon]]:text-destructive",
            "[&_[data-button]]:bg-destructive/15 [&_[data-button]]:border-destructive/30",
            "[&_[data-button]]:text-destructive [&_[data-button]:hover]:bg-destructive/25",
          ].join(" "),
          success: "bg-emerald-500/[0.08] border-emerald-500/30",
          closeButton: "",
          icon: "shrink-0",
        },
      }}
    />
  );
}
