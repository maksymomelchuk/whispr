import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useState } from "react";
import { toast } from "sonner";

import { listMeetings } from "@/lib/api";
import type { MeetingMeta } from "@/lib/types";

export const MEETINGS_UPDATED_EVENT = "meetings-updated";

/** Calls `onUpdate` once the update listener is registered (the initial
 * read) and again on every `meetings-updated` event. Reading only after
 * registration means a change between the read and the subscription can't be
 * missed. */
export function useMeetingsSubscription(onUpdate: () => void) {
  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;
    listen(MEETINGS_UPDATED_EVENT, onUpdate)
      .then((un) => {
        if (cancelled) return un();
        unlisten = un;
        onUpdate();
      })
      .catch((err) => {
        console.error("listen(meetings-updated) failed", err);
        if (!cancelled) onUpdate();
      });
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

  useMeetingsSubscription(refresh);

  return { meetings, refresh };
}
