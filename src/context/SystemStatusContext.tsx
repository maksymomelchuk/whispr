import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useRef,
  useState,
} from "react";

import {
  checkSpeechProviderKey,
  getLocalModelStatuses,
  listInputDevices,
} from "../lib/api";
import type { LocalWhisperModel, ProviderHealthStatus } from "../lib/types";
import { useSettings } from "./SettingsContext";

interface SystemStatusContextValue {
  micMissing: boolean;
  loadFailedModels: Set<LocalWhisperModel>;
  speechProviderStatuses: Map<string, ProviderHealthStatus>;
}

const SystemStatusContext = createContext<SystemStatusContextValue>({
  micMissing: false,
  loadFailedModels: new Set(),
  speechProviderStatuses: new Map(),
});

export function useSystemStatus(): SystemStatusContextValue {
  return useContext(SystemStatusContext);
}

function toProviderHealth(
  validation: { kind: string } | null,
): ProviderHealthStatus | null {
  if (!validation) return null;
  if (validation.kind === "valid") return "valid";
  if (validation.kind === "invalid") return "rejected";
  return "unreachable";
}

const PROVIDER_CHECK_DEBOUNCE_MS = 2000;

export function SystemStatusProvider({
  children,
}: {
  children: React.ReactNode;
}) {
  const { settings } = useSettings();
  const [micMissing, setMicMissing] = useState(false);
  const [loadFailedModels, setLoadFailedModels] = useState<
    Set<LocalWhisperModel>
  >(new Set());
  const [speechProviderStatuses, setSpeechProviderStatuses] = useState<
    Map<string, ProviderHealthStatus>
  >(new Map());

  const debounceTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const inputDevice = settings.input_device;
  const configuredProvidersKey = settings.modes
    .map((m) => m.provider_model.provider)
    .filter((p) => p !== "local")
    .sort()
    .join(",");

  const checkDevices = useCallback(async () => {
    if (!inputDevice) {
      setMicMissing(false);
      return;
    }
    try {
      const devices = await listInputDevices();
      setMicMissing(!devices.includes(inputDevice));
    } catch (e) {
      // Non-critical; leave prior micMissing rather than fabricate a warning
      // on a transient enumeration failure.
      console.error("listInputDevices failed", e);
    }
  }, [inputDevice]);

  const checkModels = useCallback(async () => {
    try {
      const statuses = await getLocalModelStatuses();
      const failed = new Set<LocalWhisperModel>(
        statuses.filter((s) => s.load_failed).map((s) => s.model),
      );
      setLoadFailedModels(failed);
    } catch (e) {
      // Non-critical; leave prior loadFailedModels on a transient failure.
      console.error("getLocalModelStatuses failed", e);
    }
  }, []);

  const checkProviders = useCallback(async () => {
    const providers = configuredProvidersKey
      ? Array.from(new Set(configuredProvidersKey.split(",")))
      : [];
    if (providers.length === 0) {
      setSpeechProviderStatuses(new Map());
      return;
    }
    const results = await Promise.allSettled(
      providers.map(async (provider) => {
        try {
          const validation = await checkSpeechProviderKey(provider);
          const health = toProviderHealth(validation);
          return { provider, health };
        } catch {
          return { provider, health: "unreachable" as ProviderHealthStatus };
        }
      }),
    );
    setSpeechProviderStatuses(() => {
      const next = new Map<string, ProviderHealthStatus>();
      for (const result of results) {
        if (result.status === "fulfilled" && result.value.health) {
          next.set(result.value.provider, result.value.health);
        }
      }
      return next;
    });
  }, [configuredProvidersKey]);

  useEffect(() => {
    checkDevices();
    checkModels();
    checkProviders();
  }, [checkDevices, checkModels, checkProviders]);

  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;

    getCurrentWindow()
      .onFocusChanged(({ payload: focused }) => {
        if (!focused) return;
        if (debounceTimer.current) clearTimeout(debounceTimer.current);
        debounceTimer.current = setTimeout(() => {
          if (!cancelled) {
            checkDevices();
            checkProviders();
          }
        }, PROVIDER_CHECK_DEBOUNCE_MS);
      })
      .then((un) => {
        if (cancelled) {
          un();
        } else {
          unlisten = un;
        }
      })
      .catch((e) => console.error("onFocusChanged listen failed", e));

    return () => {
      cancelled = true;
      unlisten?.();
      if (debounceTimer.current) clearTimeout(debounceTimer.current);
    };
  }, [checkDevices, checkProviders]);

  useEffect(() => {
    let cancelled = false;
    let unlistenFn: (() => void) | undefined;

    listen<LocalWhisperModel>("model-download-complete", () => {
      if (!cancelled) checkModels();
    })
      .then((un) => {
        if (cancelled) un();
        else unlistenFn = un;
      })
      .catch((e) => console.error("model-download-complete listen failed", e));

    return () => {
      cancelled = true;
      unlistenFn?.();
    };
  }, [checkModels]);

  return (
    <SystemStatusContext.Provider
      value={{ micMissing, loadFailedModels, speechProviderStatuses }}
    >
      {children}
    </SystemStatusContext.Provider>
  );
}
