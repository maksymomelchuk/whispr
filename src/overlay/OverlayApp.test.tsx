import { listen } from "@tauri-apps/api/event";
import { act, render } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { OverlayApp } from "./OverlayApp";

type Handler<T = unknown> = (e: { payload: T }) => void;

describe("OverlayApp", () => {
  let handlers: Record<string, Handler>;

  beforeEach(() => {
    handlers = {};
    vi.mocked(listen).mockImplementation(async (event, handler) => {
      handlers[event as string] = handler as Handler;
      return () => {};
    });
  });

  async function renderAndSettle() {
    const result = render(<OverlayApp />);
    await act(async () => {});
    return result;
  }

  function fire<T = undefined>(event: string, payload?: T) {
    act(() => handlers[event]?.({ payload: payload as unknown }));
  }

  it("ptt-pressed resets to recording mode and shows waveform", async () => {
    const { container } = await renderAndSettle();
    fire("ptt-error");
    expect(container.querySelector(".overlay-error-icon")).toBeInTheDocument();
    fire("ptt-pressed");
    expect(container.querySelector(".overlay-wave")).toBeInTheDocument();
    expect(
      container.querySelector(".overlay-error-icon"),
    ).not.toBeInTheDocument();
    expect(container.querySelector(".overlay-spinner")).not.toBeInTheDocument();
  });

  it("transcript-partial shows partial text in recording mode", async () => {
    const { container, getByText } = await renderAndSettle();
    expect(container.querySelector(".overlay-wave")).toBeInTheDocument();
    fire("transcript-partial", "hello world");
    expect(getByText("hello world")).toBeInTheDocument();
  });

  it("ptt-released shows spinner and hides waveform", async () => {
    const { container } = await renderAndSettle();
    fire("ptt-released");
    expect(container.querySelector(".overlay-spinner")).toBeInTheDocument();
    expect(container.querySelector(".overlay-wave")).not.toBeInTheDocument();
  });

  it("ptt-thinking shows sparkle and hides waveform", async () => {
    const { container } = await renderAndSettle();
    fire("ptt-thinking");
    expect(container.querySelector(".overlay-sparkle")).toBeInTheDocument();
    expect(container.querySelector(".overlay-wave")).not.toBeInTheDocument();
  });

  it("ptt-error shows error icon and hides waveform and spinner", async () => {
    const { container } = await renderAndSettle();
    fire("ptt-error");
    expect(container.querySelector(".overlay-error-icon")).toBeInTheDocument();
    expect(container.querySelector(".overlay-wave")).not.toBeInTheDocument();
    expect(container.querySelector(".overlay-spinner")).not.toBeInTheDocument();
  });

  it("ptt-cancelled renders cancel icon and Cancelled label", async () => {
    const { container, getByText } = await renderAndSettle();
    fire("ptt-cancelled");
    expect(container.querySelector(".overlay-cancel-icon")).toBeInTheDocument();
    expect(getByText("Cancelled")).toBeInTheDocument();
    expect(container.querySelector(".overlay-wave")).not.toBeInTheDocument();
    expect(container.querySelector(".overlay-spinner")).not.toBeInTheDocument();
  });

  it("a meeting-started notice is a one-line pill with a record dot", async () => {
    const { container, getByText } = await renderAndSettle();
    fire("overlay-notice", {
      text: "Recording meeting",
      kind: "meeting_started",
    });
    expect(getByText("Recording meeting")).toBeInTheDocument();
    expect(container.querySelector(".overlay-record-dot")).toBeInTheDocument();
    expect(container.querySelector(".overlay-timer")).not.toBeInTheDocument();
    expect(container.querySelector(".overlay-wave")).not.toBeInTheDocument();
  });

  it("a pinned meeting shows a recording pill with elapsed time", async () => {
    const startedAtSeconds = Math.floor(Date.now() / 1000) - 75;
    const { getByText } = await renderAndSettle();
    fire("overlay-meeting", startedAtSeconds);
    expect(getByText("Recording meeting · 1:15")).toBeInTheDocument();
  });

  it("dictation takes over the pinned meeting pill", async () => {
    const { container } = await renderAndSettle();
    fire("overlay-meeting", Math.floor(Date.now() / 1000));
    fire("ptt-pressed");
    expect(container.querySelector(".overlay-wave")).toBeInTheDocument();
  });

  it("a meeting-stopped notice shows a check instead of the record dot", async () => {
    const { container, getByText } = await renderAndSettle();
    fire("overlay-notice", { text: "Writing notes…", kind: "meeting_stopped" });
    expect(getByText("Writing notes…")).toBeInTheDocument();
    expect(container.querySelector(".overlay-check-icon")).toBeInTheDocument();
    expect(
      container.querySelector(".overlay-record-dot"),
    ).not.toBeInTheDocument();
  });

  it("an error notice shows the error icon with its message", async () => {
    const { container, getByText } = await renderAndSettle();
    fire("overlay-notice", { text: "Meeting: no key", kind: "error" });
    expect(getByText("Meeting: no key")).toBeInTheDocument();
    expect(container.querySelector(".overlay-error-icon")).toBeInTheDocument();
  });

  it("ptt-pressed after ptt-cancelled resets to recording", async () => {
    const { container } = await renderAndSettle();
    fire("ptt-cancelled");
    expect(container.querySelector(".overlay-cancel-icon")).toBeInTheDocument();
    fire("ptt-pressed");
    expect(container.querySelector(".overlay-wave")).toBeInTheDocument();
    expect(
      container.querySelector(".overlay-cancel-icon"),
    ).not.toBeInTheDocument();
  });

  it("transcript-partial during cancelled state does not render preview", async () => {
    const { container } = await renderAndSettle();
    fire("ptt-cancelled");
    fire("transcript-partial", "leaked words");
    expect(container.querySelector(".overlay-partial")).not.toBeInTheDocument();
  });

  it("overlay-reset returns to recording mode and shows waveform", async () => {
    const { container } = await renderAndSettle();
    fire("ptt-thinking");
    expect(container.querySelector(".overlay-sparkle")).toBeInTheDocument();
    fire("overlay-reset");
    expect(container.querySelector(".overlay-wave")).toBeInTheDocument();
    expect(container.querySelector(".overlay-sparkle")).not.toBeInTheDocument();
  });
});
