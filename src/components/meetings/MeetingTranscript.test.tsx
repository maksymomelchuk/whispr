import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeAll, describe, expect, it, vi } from "vitest";

import { renameMeetingSpeaker } from "@/lib/api";
import type { Meeting, MeetingSegment } from "@/lib/types";

import { MeetingTranscript } from "./MeetingTranscript";

vi.mock("@/lib/api", () => ({
  renameMeetingSpeaker: vi.fn().mockResolvedValue(undefined),
}));

function segment(speaker: string, text: string, echo = false): MeetingSegment {
  return { start_ms: 0, end_ms: 1000, speaker, text, echo };
}

function meeting(speakerNames: Record<string, string> = {}): Meeting {
  return {
    meta: {
      id: "meeting-1",
      started_at: 0,
      duration_ms: 0,
      status: { kind: "ready" },
      summary_model: null,
      system_audio_error: null,
      speaker_names: speakerNames,
    },
    segments: [
      segment("Speaker 1", "Thanks, Daniel."),
      segment("You", "Thanks Daniel", true),
      segment("Speaker 2", "Olena here."),
    ],
    notes: null,
  };
}

// jsdom does no layout, so the virtualized list would see a 0px viewport and
// render no lines.
const STUB_LAYOUT_SIZE_PX = 600;

beforeAll(() => {
  for (const property of ["clientHeight", "clientWidth", "offsetHeight"]) {
    Object.defineProperty(HTMLElement.prototype, property, {
      configurable: true,
      get: () => STUB_LAYOUT_SIZE_PX,
    });
  }
});

describe("MeetingTranscript", () => {
  it("hides echo lines until the toggle is switched on", async () => {
    render(<MeetingTranscript meeting={meeting()} />);
    expect(screen.queryByText("Thanks Daniel")).toBeNull();

    await userEvent.click(screen.getByLabelText(/show hidden echo lines/i));

    expect(screen.getByText("Thanks Daniel")).toBeTruthy();
  });

  it("search narrows the transcript to matching lines", async () => {
    render(<MeetingTranscript meeting={meeting()} />);

    await userEvent.type(screen.getByLabelText("Search transcript"), "olena");

    expect(screen.queryByText("Thanks, Daniel.")).toBeNull();
    expect(screen.getByText("Olena here.")).toBeTruthy();
  });

  it("shows detected names in place of speaker labels", () => {
    render(<MeetingTranscript meeting={meeting({ "Speaker 2": "Olena" })} />);

    expect(screen.getByText("Olena:")).toBeTruthy();
  });

  it("renaming a speaker saves the typed name", async () => {
    render(<MeetingTranscript meeting={meeting()} />);

    await userEvent.click(screen.getByTitle("Rename Speaker 1"));
    await userEvent.type(
      screen.getByLabelText("Name for Speaker 1"),
      "Daniel{Enter}",
    );

    expect(renameMeetingSpeaker).toHaveBeenCalledWith(
      "meeting-1",
      "Speaker 1",
      "Daniel",
    );
  });
});
