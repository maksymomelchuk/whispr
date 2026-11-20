import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { MeetingNotes } from "./MeetingNotes";

const NOTES = `# Beta launch sync

## Action items
- [ ] Daniel — write the release notes
- [x] Check Ukrainian transcription
`;

describe("MeetingNotes", () => {
  it("renders Markdown sections as headings, not raw hashes", () => {
    render(<MeetingNotes notes={NOTES} hasTitle />);

    expect(
      screen.getByRole("heading", { name: "Action items" }),
    ).toBeInTheDocument();
    expect(screen.queryByText(/##/)).not.toBeInTheDocument();
  });

  it("renders action items as checkboxes reflecting their done state", () => {
    render(<MeetingNotes notes={NOTES} hasTitle />);

    const checkboxes = screen.getAllByRole("checkbox");
    expect(checkboxes.map((box) => box.getAttribute("aria-checked"))).toEqual([
      "false",
      "true",
    ]);
  });

  it("drops the title heading when it is already shown as the meeting title", () => {
    render(<MeetingNotes notes={NOTES} hasTitle />);

    expect(screen.queryByText("Beta launch sync")).not.toBeInTheDocument();
  });

  it("keeps the leading heading when the meeting has no title", () => {
    render(<MeetingNotes notes={"## Summary\n- one"} hasTitle={false} />);

    expect(
      screen.getByRole("heading", { name: "Summary" }),
    ).toBeInTheDocument();
  });
});
