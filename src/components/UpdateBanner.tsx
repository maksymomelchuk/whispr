import { useAppUpdate } from "../hooks/useAppUpdate";

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
      <button className="primary" onClick={installAndRestart}>
        Install &amp; restart
      </button>
    </div>
  );
}
