import { toast } from "sonner";
import { afterEach, describe, expect, it, vi } from "vitest";

import { toastRetry } from "./toastRetry";

vi.mock("sonner", () => ({
  toast: {
    error: vi.fn().mockReturnValue("toast-1"),
    dismiss: vi.fn(),
  },
}));

afterEach(() => vi.clearAllMocks());

describe("toastRetry", () => {
  it("calls toast.error with the message and a Retry action", () => {
    toastRetry("Couldn't save", vi.fn().mockResolvedValue(undefined));
    expect(toast.error).toHaveBeenCalledWith(
      "Couldn't save",
      expect.objectContaining({
        action: expect.objectContaining({ label: "Retry" }),
      }),
    );
  });

  it("includes description when provided", () => {
    toastRetry(
      "Couldn't save",
      vi.fn().mockResolvedValue(undefined),
      "Network error",
    );
    expect(toast.error).toHaveBeenCalledWith(
      "Couldn't save",
      expect.objectContaining({ description: "Network error" }),
    );
  });

  it("omits description when not provided", () => {
    toastRetry("Couldn't save", vi.fn().mockResolvedValue(undefined));
    const opts = vi.mocked(toast.error).mock.calls[0][1] as Record<
      string,
      unknown
    >;
    expect(opts).not.toHaveProperty("description");
  });

  it("dismisses the toast when retry succeeds", async () => {
    const retry = vi.fn().mockResolvedValue(undefined);
    toastRetry("Couldn't save", retry);

    const opts = vi.mocked(toast.error).mock.calls[0][1] as Record<
      string,
      unknown
    >;
    const action = opts.action as { onClick: () => void };
    action.onClick();

    await vi.waitFor(() =>
      expect(toast.dismiss).toHaveBeenCalledWith("toast-1"),
    );
  });

  it("shows a new error toast when retry fails", async () => {
    const retry = vi.fn().mockRejectedValue(new Error("still broken"));
    toastRetry("Couldn't save", retry);

    const opts = vi.mocked(toast.error).mock.calls[0][1] as Record<
      string,
      unknown
    >;
    const action = opts.action as { onClick: () => void };
    action.onClick();

    await vi.waitFor(() => expect(toast.error).toHaveBeenCalledTimes(2));
    expect(toast.error).toHaveBeenLastCalledWith(
      "Couldn't save",
      expect.objectContaining({ description: "Error: still broken" }),
    );
  });

  it("new error toast after failed retry also has a Retry action", async () => {
    const retry = vi.fn().mockRejectedValue(new Error("still broken"));
    toastRetry("Couldn't save", retry);

    const opts = vi.mocked(toast.error).mock.calls[0][1] as Record<
      string,
      unknown
    >;
    const action = opts.action as { onClick: () => void };
    action.onClick();

    await vi.waitFor(() => expect(toast.error).toHaveBeenCalledTimes(2));
    const secondOpts = vi.mocked(toast.error).mock.calls[1][1] as Record<
      string,
      unknown
    >;
    expect(secondOpts.action).toMatchObject({ label: "Retry" });
  });
});
