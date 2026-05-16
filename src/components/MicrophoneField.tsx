import { useEffect, useState } from "react";
import { useForm } from "react-hook-form";

import { Alert, AlertDescription } from "@/components/ui/alert";
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

import {
  listInputDevices,
  setInputDevice as persistInputDevice,
  setPauseMediaOnRecord as persistPauseMediaOnRecord,
} from "../lib/api";
import { SectionCard } from "./SectionCard";
import { ToggleRow } from "./ToggleRow";

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
    <SectionCard title="Audio">
      <Form {...form}>
        <FormField
          control={form.control}
          name="device"
          render={({ field }) => (
            <FormItem className="mt-2.5 gap-[6px]">
              <FormLabel className="text-[11px] font-medium tracking-[0.2px] text-muted-foreground">
                Input device
              </FormLabel>
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

      <ToggleRow
        id="mute-system-audio"
        label="Mute system audio while recording"
        checked={pauseEnabled}
        onCheckedChange={togglePauseMedia}
      />

      {loadState === "loading" && (
        <p className="mt-2 text-xs text-muted-foreground">
          Enumerating devices…
        </p>
      )}
      {loadState === "error" && (
        <Alert variant="destructive" className="mt-2">
          <AlertDescription>{loadError}</AlertDescription>
        </Alert>
      )}
      {missing && (
        <Alert variant="destructive" className="mt-2">
          <AlertDescription>
            Saved device isn&rsquo;t currently available. Recording will use the
            system default until it&rsquo;s reconnected.
          </AlertDescription>
        </Alert>
      )}
      {status === "saved" && (
        <Alert variant="success" className="mt-2">
          <AlertDescription>Saved</AlertDescription>
        </Alert>
      )}
      {status === "error" && (
        <Alert variant="destructive" className="mt-2">
          <AlertDescription>{saveError}</AlertDescription>
        </Alert>
      )}
    </SectionCard>
  );
}
