import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { useEffect, useRef, useState } from "react";

type UpdateState =
  | { status: "idle" }
  | { status: "available"; update: Update }
  | { status: "downloading" }
  | { status: "error"; message: string };

const CHECK_INTERVAL_MS = 10 * 60 * 1000;

// Silent about "no update found" and transient network errors — the
// updater runs in the background and shouldn't nag the user if GitHub is
// unreachable. The window is a hidden menu-bar surface most of the time, so
// re-check on focus/visibility (when the user reopens it) with a long interval
// as a backstop, rather than relying on the once-on-mount check alone.
export function useAppUpdate() {
  const [state, setState] = useState<UpdateState>({ status: "idle" });
  const stateRef = useRef(state);
  stateRef.current = state;
  const checking = useRef(false);

  useEffect(() => {
    let cancelled = false;

    const runCheck = async () => {
      // Once an update is surfaced or installing, stop polling — re-checking
      // can't improve on it and would race the in-flight install.
      const { status } = stateRef.current;
      if (
        checking.current ||
        status === "available" ||
        status === "downloading"
      ) {
        return;
      }
      checking.current = true;
      try {
        const update = await check();
        if (!cancelled && update) {
          setState({ status: "available", update });
        }
      } catch (e) {
        console.warn("updater: check failed", e);
      } finally {
        checking.current = false;
      }
    };

    runCheck();

    const onFocus = () => runCheck();
    const onVisibilityChange = () => {
      if (document.visibilityState === "visible") runCheck();
    };
    window.addEventListener("focus", onFocus);
    document.addEventListener("visibilitychange", onVisibilityChange);
    const intervalId = window.setInterval(runCheck, CHECK_INTERVAL_MS);

    return () => {
      cancelled = true;
      window.removeEventListener("focus", onFocus);
      document.removeEventListener("visibilitychange", onVisibilityChange);
      window.clearInterval(intervalId);
    };
  }, []);

  const installAndRestart = async () => {
    if (state.status !== "available") return;
    setState({ status: "downloading" });
    try {
      await state.update.downloadAndInstall();
      await relaunch();
    } catch (e) {
      setState({ status: "error", message: String(e) });
    }
  };

  return { state, installAndRestart };
}
