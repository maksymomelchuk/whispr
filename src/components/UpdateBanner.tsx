import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";

import { useAppUpdate } from "../hooks/useAppUpdate";

export function UpdateBanner() {
  const { state, installAndRestart } = useAppUpdate();

  if (state.status === "idle") return null;

  if (state.status === "error") {
    return (
      <Alert
        variant="destructive"
        className="rounded-none border-x-0 border-t-0"
      >
        <AlertDescription>Update failed: {state.message}</AlertDescription>
      </Alert>
    );
  }

  if (state.status === "downloading") {
    return (
      <Alert className="rounded-none border-x-0 border-t-0">
        <AlertDescription>Downloading update…</AlertDescription>
      </Alert>
    );
  }

  const { version } = state.update;
  return (
    <Alert className="flex items-center gap-2 rounded-none border-x-0 border-t-0">
      <AlertDescription className="flex-1">
        Update available (
        <span className="font-variant-numeric-tabular">v{version}</span>)
      </AlertDescription>
      <Button size="sm" className="ml-auto" onClick={installAndRestart}>
        Install &amp; restart
      </Button>
    </Alert>
  );
}
