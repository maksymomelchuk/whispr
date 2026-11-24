import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { renameMeeting } from "@/lib/api";

import { RenamableTitle } from "./RenamableTitle";

vi.mock("@/lib/api", () => ({
  renameMeeting: vi.fn(() => Promise.resolve()),
}));

function startEditing() {
  render(<RenamableTitle meetingId="m1" title="Weekly sync" />);
  fireEvent.click(screen.getByRole("button", { name: /weekly sync/i }));
  return screen.getByRole("textbox", { name: "Meeting title" });
}

describe("RenamableTitle", () => {
  it("Enter saves the trimmed new title", () => {
    vi.mocked(renameMeeting).mockClear();
    const input = startEditing();
    fireEvent.change(input, { target: { value: "  Launch review " } });
    fireEvent.keyDown(input, { key: "Enter" });
    expect(renameMeeting).toHaveBeenCalledWith("m1", "Launch review");
  });

  it("Escape leaves the title unchanged", () => {
    vi.mocked(renameMeeting).mockClear();
    const input = startEditing();
    fireEvent.change(input, { target: { value: "Launch review" } });
    fireEvent.keyDown(input, { key: "Escape" });
    expect(renameMeeting).not.toHaveBeenCalled();
    expect(screen.getByText("Weekly sync")).toBeInTheDocument();
  });

  it("a blank title is not saved", () => {
    vi.mocked(renameMeeting).mockClear();
    const input = startEditing();
    fireEvent.change(input, { target: { value: "   " } });
    fireEvent.keyDown(input, { key: "Enter" });
    expect(renameMeeting).not.toHaveBeenCalled();
  });
});
