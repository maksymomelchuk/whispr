import { act, render } from "@testing-library/react";
import { listen } from "@tauri-apps/api/event";
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
    expect(container.querySelector(".overlay-error-icon")).not.toBeInTheDocument();
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

  it("ptt-thinking shows spinner and hides waveform", async () => {
    const { container } = await renderAndSettle();
    fire("ptt-thinking");
    expect(container.querySelector(".overlay-spinner")).toBeInTheDocument();
    expect(container.querySelector(".overlay-wave")).not.toBeInTheDocument();
  });

  it("ptt-error shows error icon and hides waveform and spinner", async () => {
    const { container } = await renderAndSettle();
    fire("ptt-error");
    expect(container.querySelector(".overlay-error-icon")).toBeInTheDocument();
    expect(container.querySelector(".overlay-wave")).not.toBeInTheDocument();
    expect(container.querySelector(".overlay-spinner")).not.toBeInTheDocument();
  });

  it("overlay-reset returns to recording mode and shows waveform", async () => {
    const { container } = await renderAndSettle();
    fire("ptt-thinking");
    expect(container.querySelector(".overlay-spinner")).toBeInTheDocument();
    fire("overlay-reset");
    expect(container.querySelector(".overlay-wave")).toBeInTheDocument();
    expect(container.querySelector(".overlay-spinner")).not.toBeInTheDocument();
  });
});
