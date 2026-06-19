import { useEffect, useRef, useState } from "react";
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

import { useSettings } from "../context/SettingsContext";
import {
  listInputDevices,
  setHandsFreeMaxMinutes as persistHandsFreeMaxMinutes,
  setInputDevice as persistInputDevice,
  setPauseMediaOnRecord as persistPauseMediaOnRecord,
} from "../lib/api";
import { SectionCard } from "./SectionCard";
import { SelectRow } from "./SelectRow";
import { ToggleRow } from "./ToggleRow";

// Radix Select doesn't render correctly with empty-string values; use a sentinel.
const DEVICE_DEFAULT = "__system_default__";

const HANDS_FREE_OPTIONS: { label: string; value: string }[] = [
  { label: "5 min", value: "5" },
  { label: "10 min", value: "10" },
  { label: "15 min", value: "15" },
  { label: "30 min", value: "30" },
  { label: "45 min", value: "45" },
  { label: "60 min", value: "60" },
];

function toSelectValue(device: string | null): string {
  return device ?? DEVICE_DEFAULT;
}

function fromSelectValue(val: string): string | null {
  return val === DEVICE_DEFAULT ? null : val;
}

type LoadState = "loading" | "ready" | "error";
type SaveStatus = "idle" | "saving" | "error";

export function MicrophoneField() {
  const { settings, setSettings } = useSettings();
  const { input_device, pause_media_on_record, hands_free_max_minutes } =
    settings;

  const [devices, setDevices] = useState<string[]>([]);
  const [loadState, setLoadState] = useState<LoadState>("loading");
  const [loadError, setLoadError] = useState<string | null>(null);
  const [status, setStatus] = useState<SaveStatus>("idle");
  const [saveError, setSaveError] = useState<string | null>(null);

  const form = useForm({ values: { device: toSelectValue(input_device) } });
  const selectTriggerRef = useRef<HTMLButtonElement>(null);

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

  const handleChange = async (next: string) => {
    const previous = form.getValues("device");
    form.setValue("device", next);
    const payload = fromSelectValue(next);
    setStatus("saving");
    setSaveError(null);
    try {
      await persistInputDevice(payload);
      setSettings((s) => ({ ...s, input_device: payload }));
      setStatus("idle");
    } catch (e) {
      form.setValue("device", previous);
      setStatus("error");
      setSaveError(String(e));
    }
  };

  const togglePauseMedia = async () => {
    const next = !pause_media_on_record;
    setSaveError(null);
    setSettings((s) => ({ ...s, pause_media_on_record: next }));
    try {
      await persistPauseMediaOnRecord(next);
    } catch (e) {
      setSettings((s) => ({ ...s, pause_media_on_record: !next }));
      setSaveError(String(e));
    }
  };

  const handleHandsFreeChange = async (value: string) => {
    const next = Number(value);
    const previous = hands_free_max_minutes;
    setSettings((s) => ({ ...s, hands_free_max_minutes: next }));
    try {
      await persistHandsFreeMaxMinutes(next);
    } catch (e) {
      setSettings((s) => ({ ...s, hands_free_max_minutes: previous }));
      setSaveError(String(e));
    }
  };

  const missing =
    loadState === "ready" &&
    input_device !== null &&
    input_device !== undefined &&
    !devices.includes(input_device);

  const isDisabled = loadState !== "ready" || status === "saving";

  return (
    <SectionCard title="Audio">
      <Form {...form}>
        <FormField
          control={form.control}
          name="device"
          render={({ field }) => (
            <FormItem className="mt-2.5 gap-[6px]">
              <FormLabel>Input device</FormLabel>
              <FormControl>
                <Select
                  value={field.value}
                  onValueChange={handleChange}
                  disabled={isDisabled}
                >
                  <SelectTrigger ref={selectTriggerRef} className="w-full">
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
                    {missing && input_device ? (
                      <SelectItem value={input_device}>
                        {input_device} (unavailable)
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
        checked={pause_media_on_record}
        onCheckedChange={togglePauseMedia}
      />

      <SelectRow
        id="hands-free-auto-stop"
        label="Auto-stop hands-free after"
        info="A latched hands-free recording (flick-tap your dictation key, then press again to stop) auto-finalizes at this duration as a runaway-mic safety net."
        value={String(hands_free_max_minutes)}
        options={HANDS_FREE_OPTIONS}
        onValueChange={handleHandsFreeChange}
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
        <Alert className="mt-2">
          <AlertDescription>
            Saved microphone isn&rsquo;t available. Recording will use the
            system default until it reconnects.{" "}
            <button
              type="button"
              className="underline underline-offset-2 cursor-pointer"
              onClick={() => selectTriggerRef.current?.click()}
            >
              Choose another
            </button>
          </AlertDescription>
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
