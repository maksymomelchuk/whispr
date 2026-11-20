import { describe, expect, it } from "vitest";

import type { MeetingSegment } from "@/lib/types";

import {
  filterLines,
  remoteSpeakerLabels,
  shouldSuggestHeadphones,
  transcriptLines,
} from "./transcriptLines";

function segment(speaker: string, text: string, echo = false): MeetingSegment {
  return { start_ms: 0, end_ms: 1000, speaker, text, echo };
}

const conversation = [
  segment("Speaker 1", "Thanks, Daniel."),
  segment("You", "Thanks Daniel", true),
  segment("Speaker 2", "Olena here, quick update."),
  segment("You", "Sounds good."),
];

describe("filterLines", () => {
  it("hides echo lines unless asked to show them", () => {
    const lines = transcriptLines(conversation, {});

    const shown = filterLines(lines, { showEchoes: false, query: "" });

    expect(shown.map((line) => line.index)).toEqual([0, 2, 3]);
  });

  it("keeps each line's position in the full transcript", () => {
    const lines = transcriptLines(conversation, {});

    const shown = filterLines(lines, { showEchoes: true, query: "olena" });

    expect(shown.map((line) => line.index)).toEqual([2]);
  });

  it("matches search against the speaker's name as well as the text", () => {
    const lines = transcriptLines(conversation, { "Speaker 2": "Olena" });

    const shown = filterLines(lines, { showEchoes: false, query: "OLENA" });

    expect(shown.map((line) => line.speakerName)).toEqual(["Olena"]);
  });
});

describe("transcriptLines", () => {
  it("shows a named speaker by name and others by label", () => {
    const names = transcriptLines(conversation, { "Speaker 1": "Daniel" }).map(
      (line) => line.speakerName,
    );

    expect(names).toEqual(["Daniel", "You", "Speaker 2", "You"]);
  });
});

describe("remoteSpeakerLabels", () => {
  it("lists call speakers once, in order of appearance, without You", () => {
    expect(remoteSpeakerLabels([...conversation, ...conversation])).toEqual([
      "Speaker 1",
      "Speaker 2",
    ]);
  });
});

describe("shouldSuggestHeadphones", () => {
  it("stays quiet for a single stray echo", () => {
    expect(shouldSuggestHeadphones(conversation)).toBe(false);
  });

  it("suggests headphones when many of the user's lines are echoes", () => {
    const echoes = [1, 2, 3].map(() => segment("You", "echo", true));

    expect(shouldSuggestHeadphones([...conversation, ...echoes])).toBe(true);
  });
});
