import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { useEffect, useState } from "react";

type UpdateState =
  | { status: "idle" }
  | { status: "available"; update: Update }
  | { status: "downloading" }
  | { status: "error"; message: string };

// Silent about "no update found" and transient network errors — the
// updater runs in the background and shouldn't nag the user if GitHub is
// unreachable.
export function useAppUpdate() {
  const [state, setState] = useState<UpdateState>({ status: "idle" });

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const update = await check();
        if (!cancelled && update) {
          setState({ status: "available", update });
        }
      } catch (e) {
        // Don't prompt; only log so a dev can see it in a debug build.
        console.warn("updater: check failed", e);
      }
    })();
    return () => {
      cancelled = true;
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
