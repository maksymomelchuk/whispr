import { toast } from "sonner";
import { afterEach, describe, expect, it, vi } from "vitest";

import { toastUndo } from "./toastUndo";

vi.mock("sonner", () => ({
  toast: vi.fn(),
}));

afterEach(() => vi.clearAllMocks());

describe("toastUndo", () => {
  it("calls toast with the message and an Undo action", () => {
    toastUndo("Deleted item", vi.fn(), vi.fn());
    expect(toast).toHaveBeenCalledWith(
      "Deleted item",
      expect.objectContaining({
        action: expect.objectContaining({ label: "Undo" }),
      }),
    );
  });

  it("commits when onAutoClose fires and undo was not pressed", () => {
    const onCommit = vi.fn().mockResolvedValue(undefined);
    toastUndo("Deleted item", onCommit, vi.fn());

    const opts = vi.mocked(toast).mock.calls[0][1] as Record<string, unknown>;
    (opts.onAutoClose as (t: unknown) => void)({});

    expect(onCommit).toHaveBeenCalledOnce();
  });

  it("commits when onDismiss fires and undo was not pressed", () => {
    const onCommit = vi.fn().mockResolvedValue(undefined);
    toastUndo("Deleted item", onCommit, vi.fn());

    const opts = vi.mocked(toast).mock.calls[0][1] as Record<string, unknown>;
    (opts.onDismiss as (t: unknown) => void)({});

    expect(onCommit).toHaveBeenCalledOnce();
  });

  it("restores and skips commit when undo action is clicked", () => {
    const onCommit = vi.fn().mockResolvedValue(undefined);
    const onRestore = vi.fn();
    toastUndo("Deleted item", onCommit, onRestore);

    const opts = vi.mocked(toast).mock.calls[0][1] as Record<string, unknown>;
    const action = opts.action as { onClick: () => void };
    action.onClick();
    (opts.onAutoClose as (t: unknown) => void)({});

    expect(onRestore).toHaveBeenCalledOnce();
    expect(onCommit).not.toHaveBeenCalled();
  });

  it("does not double-commit when both onDismiss and onAutoClose fire", () => {
    const onCommit = vi.fn().mockResolvedValue(undefined);
    toastUndo("Deleted item", onCommit, vi.fn());

    const opts = vi.mocked(toast).mock.calls[0][1] as Record<string, unknown>;
    (opts.onDismiss as (t: unknown) => void)({});
    (opts.onAutoClose as (t: unknown) => void)({});

    expect(onCommit).toHaveBeenCalledOnce();
  });
});
