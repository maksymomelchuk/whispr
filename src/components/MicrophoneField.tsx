import { useEffect, useState } from "react";
import { useForm } from "react-hook-form";

import {
  Form,
  FormControl,
  FormField,
  FormItem,
  FormLabel,
} from "@/components/ui/form";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";

import {
  listInputDevices,
  setInputDevice as persistInputDevice,
  setPauseMediaOnRecord as persistPauseMediaOnRecord,
} from "../lib/api";

interface Props {
  initial: string | null;
  onSaved: (device: string | null) => void;
  pauseMedia: boolean;
  onPauseMediaSaved: (enabled: boolean) => void;
}

type LoadState = "loading" | "ready" | "error";
type SaveStatus = "idle" | "saving" | "saved" | "error";

// Radix Select doesn't render correctly with empty-string values; use a sentinel.
const DEVICE_DEFAULT = "__system_default__";

function toSelectValue(device: string | null): string {
  return device ?? DEVICE_DEFAULT;
}

function fromSelectValue(val: string): string | null {
  return val === DEVICE_DEFAULT ? null : val;
}

export function MicrophoneField({
  initial,
  onSaved,
  pauseMedia,
  onPauseMediaSaved,
}: Props) {
  const [devices, setDevices] = useState<string[]>([]);
  const [loadState, setLoadState] = useState<LoadState>("loading");
  const [loadError, setLoadError] = useState<string | null>(null);
  const [status, setStatus] = useState<SaveStatus>("idle");
  const [saveError, setSaveError] = useState<string | null>(null);
  const [pauseEnabled, setPauseEnabled] = useState(pauseMedia);

  const form = useForm({ values: { device: toSelectValue(initial) } });

  useEffect(() => {
    listInputDevices()
      .then((list) => {
        setDevices(list);
        setLoadState("ready");
      })
      .catch((e) => {
        setLoadState("error");
        setLoadError(String(e));
      });
  }, []);

  useEffect(() => {
    setPauseEnabled(pauseMedia);
  }, [pauseMedia]);

  useEffect(() => {
    if (status !== "saved") return;
    const t = setTimeout(() => setStatus("idle"), 1500);
    return () => clearTimeout(t);
  }, [status]);

  const handleChange = async (next: string) => {
    const previous = form.getValues("device");
    form.setValue("device", next);
    const payload = fromSelectValue(next);
    setStatus("saving");
    setSaveError(null);
    try {
      await persistInputDevice(payload);
      onSaved(payload);
      setStatus("saved");
    } catch (e) {
      form.setValue("device", previous);
      setStatus("error");
      setSaveError(String(e));
    }
  };

  const togglePauseMedia = async () => {
    const next = !pauseEnabled;
    setPauseEnabled(next);
    setSaveError(null);
    try {
      await persistPauseMediaOnRecord(next);
      onPauseMediaSaved(next);
    } catch (e) {
      setPauseEnabled(!next);
      setSaveError(String(e));
    }
  };

  const missing =
    loadState === "ready" &&
    initial !== null &&
    initial !== undefined &&
    !devices.includes(initial);

  const isDisabled = loadState !== "ready" || status === "saving";

  return (
    <section className="card">
      <h2>Audio</h2>

      <Form {...form}>
        <FormField
          control={form.control}
          name="device"
          render={({ field }) => (
            <FormItem className="mt-2.5 gap-[6px]">
              <FormLabel className="field-label">Input device</FormLabel>
              <FormControl>
                <Select
                  value={field.value}
                  onValueChange={handleChange}
                  disabled={isDisabled}
                >
                  <SelectTrigger className="w-full">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value={DEVICE_DEFAULT}>
                      System default
                    </SelectItem>
                    {devices.map((name) => (
                      <SelectItem key={name} value={name}>
                        {name}
                      </SelectItem>
                    ))}
                    {missing && initial ? (
                      <SelectItem value={initial}>
                        {initial} (unavailable)
                      </SelectItem>
                    ) : null}
                  </SelectContent>
                </Select>
              </FormControl>
            </FormItem>
          )}
        />
      </Form>

      <label className="toggle-row">
        <span className="toggle-row-label">
          Mute system audio while recording
        </span>
        <Switch checked={pauseEnabled} onCheckedChange={togglePauseMedia} />
      </label>

      {loadState === "loading" && (
        <div className="status">Enumerating devices…</div>
      )}
      {loadState === "error" && <div className="status err">{loadError}</div>}
      {missing && (
        <div className="status err">
          Saved device isn&rsquo;t currently available. Recording will use the
          system default until it&rsquo;s reconnected.
        </div>
      )}
      {status === "saved" && <div className="status ok">Saved</div>}
      {status === "error" && <div className="status err">{saveError}</div>}
    </section>
  );
}
