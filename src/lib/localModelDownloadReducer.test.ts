import { describe, expect, it } from "vitest";
import { localModelDownloadReducer } from "./localModelDownloadReducer";
import type { DownloadState } from "./localModelDownloadReducer";

describe("localModelDownloadReducer", () => {
  it("start transitions to downloading at 0%", () => {
    const result = localModelDownloadReducer({ kind: "idle" }, { type: "start" });
    expect(result).toEqual<DownloadState>({
      kind: "downloading",
      percentage: 0,
      bytesDownloaded: 0,
      totalBytes: 0,
    });
  });

  it("progress updates bytesDownloaded, totalBytes, and percentage", () => {
    const initial: DownloadState = { kind: "downloading", percentage: 0, bytesDownloaded: 0, totalBytes: 0 };
    const result = localModelDownloadReducer(initial, {
      type: "progress",
      bytesDownloaded: 512_000,
      totalBytes: 1_024_000,
      percentage: 50,
    });
    expect(result).toEqual<DownloadState>({
      kind: "downloading",
      percentage: 50,
      bytesDownloaded: 512_000,
      totalBytes: 1_024_000,
    });
  });

  it("complete transitions to idle", () => {
    const initial: DownloadState = { kind: "downloading", percentage: 100, bytesDownloaded: 1000, totalBytes: 1000 };
    const result = localModelDownloadReducer(initial, { type: "complete" });
    expect(result).toEqual<DownloadState>({ kind: "idle" });
  });

  it("error transitions to error with message", () => {
    const initial: DownloadState = { kind: "downloading", percentage: 50, bytesDownloaded: 500, totalBytes: 1000 };
    const result = localModelDownloadReducer(initial, { type: "error", message: "Network timeout" });
    expect(result).toEqual<DownloadState>({ kind: "error", message: "Network timeout" });
  });

  it("cancel transitions to idle", () => {
    const initial: DownloadState = { kind: "downloading", percentage: 30, bytesDownloaded: 300, totalBytes: 1000 };
    const result = localModelDownloadReducer(initial, { type: "cancel" });
    expect(result).toEqual<DownloadState>({ kind: "idle" });
  });
});
