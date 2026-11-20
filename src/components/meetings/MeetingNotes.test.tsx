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

  it("tells screen readers whether each action item is done", () => {
    render(<MeetingNotes notes={NOTES} hasTitle />);

    const items = screen
      .getAllByRole("listitem")
      .map((li) => li.textContent?.replace(/\s+/g, " "));
    expect(items).toEqual([
      "To do: Daniel — write the release notes",
      "Done: Check Ukrainian transcription",
    ]);
  });

  it("keeps formatted task text together beside its checkbox", () => {
    const { container } = render(
      <MeetingNotes
        notes={"- [ ] Review **release notes** by Friday"}
        hasTitle={false}
      />,
    );

    const content =
      container.querySelector(".task-checkbox")?.parentElement
        ?.lastElementChild;
    expect(content?.textContent?.trim()).toBe("Review release notes by Friday");
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
