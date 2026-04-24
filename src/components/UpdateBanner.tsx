import { useAppUpdate } from "../hooks/useAppUpdate";

export function UpdateBanner() {
  const { state, installAndRestart } = useAppUpdate();

  if (state.status === "idle") return null;

  if (state.status === "error") {
    return (
      <div className="update-banner err" role="alert">
        Update failed: {state.message}
      </div>
    );
  }

  if (state.status === "downloading") {
    return (
      <div className="update-banner" role="status">
        Downloading update…
      </div>
    );
  }

  const { version } = state.update;
  return (
    <div className="update-banner" role="status">
      <span>Update available (v{version})</span>
      <button className="primary" onClick={installAndRestart}>
        Install &amp; restart
      </button>
    </div>
  );
}
