import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@/components/ui/sheet", () => ({
  SheetHeader: ({ children, ...p }: any) => <div {...p}>{children}</div>,
  SheetTitle: ({ children, ...p }: any) => <h2 {...p}>{children}</h2>,
  SheetFooter: ({ children, ...p }: any) => <div {...p}>{children}</div>,
}));

vi.mock("../lib/api", () => ({
  updateMode: vi.fn(),
  addMode: vi.fn(),
  deleteMode: vi.fn(),
  duplicateMode: vi.fn(),
  getSettings: vi.fn(),
  setDefaultMode: vi.fn(),
  formatShortcut: vi.fn(),
}));

vi.mock("sonner", () => ({
  toast: { error: vi.fn() },
}));

import { updateMode as mockUpdateMode, addMode as mockAddMode } from "../lib/api";
import { toast } from "sonner";
import type { Mode } from "@/lib/types";
import { ModeEditor } from "./ModesPage";

const MODE: Mode = {
  id: "mode-1",
  name: "Original",
  icon: null,
  language: { kind: "auto" },
  translate: { kind: "off" },
  ai_cleanup: { enabled: false, prompt_override: null },
  use_terms: true,
  use_corrections: true,
  use_snippets: true,
};

function EditorWrapper({
  mode = MODE,
  isNew = false,
  onClose = vi.fn(),
  onPersist = vi.fn(),
}: {
  mode?: Mode;
  isNew?: boolean;
  onClose?: () => void;
  onPersist?: (m: Mode, wasNew: boolean) => void;
}) {
  return (
    <ModeEditor mode={mode} isNew={isNew} onClose={onClose} onPersist={onPersist} />
  );
}

beforeEach(() => {
  vi.mocked(mockUpdateMode).mockResolvedValue(undefined);
  vi.mocked(mockAddMode).mockResolvedValue(undefined);
});

afterEach(() => {
  vi.clearAllMocks();
  vi.useRealTimers();
});

describe("ModeEditor – autosave", () => {
  it("coalesces rapid edits into a single persist call after 450 ms", async () => {
    render(<EditorWrapper />);
    const nameInput = screen.getByLabelText("Name");

    // Enable fake timers after render so React's initial setup isn't affected.
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });

    // Two rapid changes — each cancels the previous debounce and schedules a new one.
    fireEvent.change(nameInput, { target: { value: "First" } });
    fireEvent.change(nameInput, { target: { value: "Renamed" } });

    act(() => vi.advanceTimersByTime(449));
    expect(vi.mocked(mockUpdateMode)).not.toHaveBeenCalled();

    await act(async () => {
      vi.advanceTimersByTime(1);
    });
    expect(vi.mocked(mockUpdateMode)).toHaveBeenCalledTimes(1);
    expect(vi.mocked(mockUpdateMode)).toHaveBeenCalledWith(
      expect.objectContaining({ name: "Renamed" }),
    );
  });

  it("unmount flushes pending debounced write before teardown", () => {
    const { unmount } = render(<EditorWrapper />);
    const nameInput = screen.getByLabelText("Name");

    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });

    fireEvent.change(nameInput, { target: { value: "Flushed" } });
    // 450 ms timer is pending — do NOT advance it.

    unmount();

    // Cleanup effect must have called updateMode synchronously on unmount.
    expect(vi.mocked(mockUpdateMode)).toHaveBeenCalledTimes(1);
    expect(vi.mocked(mockUpdateMode)).toHaveBeenCalledWith(
      expect.objectContaining({ name: "Flushed" }),
    );
  });

  it("persist failure shows toast without reverting local state", async () => {
    vi.mocked(mockUpdateMode).mockRejectedValueOnce(new Error("server error"));

    render(<EditorWrapper />);
    const nameInput = screen.getByLabelText("Name");

    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });

    fireEvent.change(nameInput, { target: { value: "Broken" } });

    await act(async () => {
      vi.advanceTimersByTime(450);
    });

    expect(vi.mocked(toast.error)).toHaveBeenCalledWith(
      "Couldn't save mode",
      expect.anything(),
    );
    // Local state must not revert.
    expect(nameInput).toHaveValue("Broken");
  });

  it("new-mode: CTA visible; changes do not autosave; creation calls addMode", async () => {
    const onClose = vi.fn();
    const onPersist = vi.fn();
    const newMode: Mode = { ...MODE, id: "mode-new", name: "" };

    render(
      <EditorWrapper
        mode={newMode}
        isNew={true}
        onClose={onClose}
        onPersist={onPersist}
      />,
    );

    expect(screen.getByRole("button", { name: "Create mode" })).toBeDisabled();

    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });

    fireEvent.change(screen.getByLabelText("Name"), { target: { value: "My Mode" } });

    // Advance past debounce window — must NOT autosave for new modes.
    act(() => vi.advanceTimersByTime(450));
    expect(vi.mocked(mockUpdateMode)).not.toHaveBeenCalled();

    // Restore real timers before the async click/create flow.
    vi.useRealTimers();

    expect(screen.getByRole("button", { name: "Create mode" })).not.toBeDisabled();
    await userEvent.click(screen.getByRole("button", { name: "Create mode" }));

    await waitFor(() => expect(vi.mocked(mockAddMode)).toHaveBeenCalledTimes(1));
    expect(onClose).toHaveBeenCalled();
  });
});
