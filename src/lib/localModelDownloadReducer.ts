export type DownloadState =
  | { kind: "idle" }
  | {
      kind: "downloading";
      percentage: number;
      bytesDownloaded: number;
      totalBytes: number;
    }
  | { kind: "error"; message: string };

export type DownloadEvent =
  | { type: "start" }
  | {
      type: "progress";
      bytesDownloaded: number;
      totalBytes: number;
      percentage: number;
    }
  | { type: "complete" }
  | { type: "error"; message: string }
  | { type: "cancel" };

export function localModelDownloadReducer(
  _state: DownloadState,
  event: DownloadEvent,
): DownloadState {
  switch (event.type) {
    case "start":
      return {
        kind: "downloading",
        percentage: 0,
        bytesDownloaded: 0,
        totalBytes: 0,
      };
    case "progress":
      return {
        kind: "downloading",
        percentage: event.percentage,
        bytesDownloaded: event.bytesDownloaded,
        totalBytes: event.totalBytes,
      };
    case "complete":
      return { kind: "idle" };
    case "error":
      return { kind: "error", message: event.message };
    case "cancel":
      return { kind: "idle" };
  }
}
