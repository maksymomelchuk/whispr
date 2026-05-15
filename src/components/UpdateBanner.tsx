import { cn } from "../lib/utils";
import { useAppUpdate } from "../hooks/useAppUpdate";
import { Button } from "./ui/button";

interface Props {
  inline?: boolean;
}

export function UpdateBanner({ inline }: Props) {
  const { state, installAndRestart } = useAppUpdate();

  if (state.status === "idle") return null;

  const baseClass = inline ? "update-banner-inline" : "update-banner";

  if (state.status === "error") {
    return (
      <div className={`${baseClass} err`} role="alert">
        Update failed: {state.message}
      </div>
    );
  }

  if (state.status === "downloading") {
    return (
      <div className={baseClass} role="status">
        Downloading update…
      </div>
    );
  }

  const { version } = state.update;
  return (
    <div className={baseClass} role="status">
      <span>
        Update available (
        <span className="update-banner-version">v{version}</span>)
      </span>
      <Button
        size="sm"
        className={cn("rounded-full text-xs", inline && "ml-auto")}
        onClick={installAndRestart}
      >
        Install &amp; restart
      </Button>
    </div>
  );
}
