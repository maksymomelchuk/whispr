import { Toaster as SonnerToaster } from "sonner";

export function Toaster() {
  return (
    <SonnerToaster
      position="bottom-right"
      toastOptions={{
        classNames: {
          toast:
            "group toast font-sans text-sm bg-background text-foreground border border-border shadow-lg",
          description: "text-muted-foreground",
          actionButton: "bg-shadcn-primary text-shadcn-primary-foreground",
          cancelButton: "bg-muted text-muted-foreground",
          error: "bg-destructive/10 border-destructive/30 text-destructive",
        },
      }}
    />
  );
}
