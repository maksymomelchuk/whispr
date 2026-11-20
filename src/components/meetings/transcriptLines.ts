import type { MeetingSegment } from "@/lib/types";

export const MIC_SPEAKER_LABEL = "You";
/** Below this, a few stray echoes aren't worth a headphones nag. */
const HEADPHONES_HINT_MIN_ECHOES = 3;
const HEADPHONES_HINT_MIN_ECHO_SHARE = 0.2;

export interface TranscriptLine {
  /** Position in the full transcript, stable across filtering. */
  index: number;
  segment: MeetingSegment;
  speakerName: string;
}

export interface LineFilter {
  showEchoes: boolean;
  query: string;
}

export function speakerName(
  label: string,
  speakerNames: Record<string, string>,
): string {
  return speakerNames[label] ?? label;
}

export function transcriptLines(
  segments: MeetingSegment[],
  speakerNames: Record<string, string>,
): TranscriptLine[] {
  return segments.map((segment, index) => ({
    index,
    segment,
    speakerName: speakerName(segment.speaker, speakerNames),
  }));
}

export function filterLines(
  lines: TranscriptLine[],
  { showEchoes, query }: LineFilter,
): TranscriptLine[] {
  const needle = query.trim().toLocaleLowerCase();
  return lines.filter(
    (line) =>
      (showEchoes || !line.segment.echo) &&
      (needle === "" ||
        line.segment.text.toLocaleLowerCase().includes(needle) ||
        line.speakerName.toLocaleLowerCase().includes(needle)),
  );
}

export function remoteSpeakerLabels(segments: MeetingSegment[]): string[] {
  const labels = segments
    .map((segment) => segment.speaker)
    .filter((speaker) => speaker !== MIC_SPEAKER_LABEL);
  return [...new Set(labels)];
}

export function echoCount(segments: MeetingSegment[]): number {
  return segments.filter((segment) => segment.echo).length;
}

export function shouldSuggestHeadphones(segments: MeetingSegment[]): boolean {
  const echoes = echoCount(segments);
  const micLines = segments.filter(
    (segment) => segment.speaker === MIC_SPEAKER_LABEL,
  ).length;
  return (
    echoes >= HEADPHONES_HINT_MIN_ECHOES &&
    echoes / micLines >= HEADPHONES_HINT_MIN_ECHO_SHARE
  );
}
