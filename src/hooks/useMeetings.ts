import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useState } from "react";
import { toast } from "sonner";

import { listMeetings } from "@/lib/api";
import type { MeetingMeta } from "@/lib/types";

export const MEETINGS_UPDATED_EVENT = "meetings-updated";

export function useMeetingsUpdated(onUpdate: () => void) {
  useEffect(() => {
    // Race-safe: cleanup may run before listen() resolves (StrictMode).
    let cancelled = false;
    let unlisten: (() => void) | undefined;
    listen(MEETINGS_UPDATED_EVENT, onUpdate)
      .then((un) => {
        if (cancelled) un();
        else unlisten = un;
      })
      .catch((err) => console.error("listen(meetings-updated) failed", err));
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [onUpdate]);
}

export function useMeetings() {
  const [meetings, setMeetings] = useState<MeetingMeta[]>([]);

  const refresh = useCallback(() => {
    listMeetings()
      .then(setMeetings)
      .catch((err) => toast.error(`Failed to load meetings: ${String(err)}`));
  }, []);

  useEffect(refresh, [refresh]);
  useMeetingsUpdated(refresh);

  return { meetings, refresh };
}
