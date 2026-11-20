import { HeadphonesIcon, MagnifyingGlassIcon } from "@phosphor-icons/react";
import { useMemo, useState } from "react";

import { Alert, AlertDescription } from "@/components/ui/alert";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import type { Meeting } from "@/lib/types";

import { TranscriptLineList } from "./TranscriptLineList";
import {
  echoCount,
  filterLines,
  remoteSpeakerLabels,
  shouldSuggestHeadphones,
  transcriptLines,
  type TranscriptLine,
} from "./transcriptLines";
import { TranscriptSpeakers } from "./TranscriptSpeakers";

export function MeetingTranscript({ meeting }: { meeting: Meeting }) {
  const { segments, meta } = meeting;
  const [query, setQuery] = useState("");
  const [showEchoes, setShowEchoes] = useState(false);
  const [focusedIndex, setFocusedIndex] = useState<number | null>(null);

  const allLines = useMemo(
    () => transcriptLines(segments, meta.speaker_names),
    [segments, meta.speaker_names],
  );
  const lines = useMemo(
    () => filterLines(allLines, { showEchoes, query }),
    [allLines, showEchoes, query],
  );
  const echoes = echoCount(segments);

  if (segments.length === 0) return null;

  const showInContext = (line: TranscriptLine) => {
    setQuery("");
    setFocusedIndex(line.index);
  };

  return (
    <div className="flex flex-col gap-3">
      {shouldSuggestHeadphones(segments) && (
        <Alert className="flex items-center gap-2 [&>svg~*]:pl-0">
          <HeadphonesIcon />
          <AlertDescription>
            Use headphones for cleaner transcripts — your mic picked up the call{" "}
            {echoes} times. Those echo lines are hidden and left out of the
            notes.
          </AlertDescription>
        </Alert>
      )}
      <TranscriptSpeakers
        meetingId={meta.id}
        labels={remoteSpeakerLabels(segments)}
        speakerNames={meta.speaker_names}
      />
      <div className="flex flex-wrap items-center gap-3">
        <div className="relative min-w-48 flex-1">
          <MagnifyingGlassIcon className="pointer-events-none absolute left-2 top-1/2 size-3.5 -translate-y-1/2 text-muted-foreground" />
          <Input
            type="search"
            autoComplete="off"
            autoCorrect="off"
            spellCheck={false}
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder="Search transcript"
            aria-label="Search transcript"
            className="h-8 pl-7 text-sm"
          />
        </div>
        {echoes > 0 && (
          <div className="flex items-center gap-2">
            <Switch
              id="show-echo-lines"
              checked={showEchoes}
              onCheckedChange={setShowEchoes}
            />
            <Label htmlFor="show-echo-lines" className="text-xs">
              Show hidden echo lines ({echoes})
            </Label>
          </div>
        )}
      </div>
      {query.trim() !== "" && (
        <p className="text-xs text-muted-foreground">
          {lines.length} matching {lines.length === 1 ? "line" : "lines"} —
          click a time to see it in context
        </p>
      )}
      <TranscriptLineList
        lines={lines}
        focusedIndex={focusedIndex}
        onTimestampClick={showInContext}
      />
    </div>
  );
}
