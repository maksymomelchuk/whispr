import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";

import {
  cancelModelDownload,
  deleteLocalModel,
  getLocalModelStatuses,
  startModelDownload,
} from "../lib/api";
import type {
  LocalModelStatus,
  LocalWhisperModel,
  ModelDownloadError,
  ModelDownloadProgress,
} from "../lib/types";
import { SectionCard } from "./SectionCard";

const MODEL_LABELS: Record<LocalWhisperModel, string> = {
  large_v3: "Large v3",
  large_v3_turbo: "Large v3 Turbo",
};

const MODEL_SIZE_LABELS: Record<LocalWhisperModel, string> = {
  large_v3: "~1.5 GB",
  large_v3_turbo: "~809 MB",
};

type DownloadState =
  | { kind: "idle" }
  | { kind: "downloading"; percentage: number; bytesDownloaded: number; totalBytes: number }
  | { kind: "error"; message: string };

function formatBytes(bytes: number): string {
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(0)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

interface ModelRowProps {
  status: LocalModelStatus;
  downloadState: DownloadState;
  onDownload: () => void;
  onCancel: () => void;
  onDelete: () => void;
}

function ModelRow({ status, downloadState, onDownload, onCancel, onDelete }: ModelRowProps) {
  const label = MODEL_LABELS[status.model];
  const sizeLabel = MODEL_SIZE_LABELS[status.model];
  const isDownloading = downloadState.kind === "downloading";

  return (
    <div className="flex flex-col gap-2 rounded-lg border border-border bg-card px-3 py-2.5">
      <div className="flex items-center gap-3">
        <div className="flex flex-1 flex-col gap-0.5 min-w-0">
          <span className="text-sm font-medium leading-tight">{label}</span>
          <span className="text-xs text-muted-foreground">{sizeLabel}</span>
        </div>

        <div className="flex items-center gap-2 shrink-0">
          {status.downloaded ? (
            <>
              <span className="text-xs text-muted-foreground">Downloaded</span>
              <Button
                variant="ghost"
                size="sm"
                onClick={onDelete}
                className="text-muted-foreground hover:text-destructive text-xs h-7 px-2"
              >
                Delete
              </Button>
            </>
          ) : isDownloading ? (
            <Button
              variant="ghost"
              size="sm"
              onClick={onCancel}
              className="text-muted-foreground text-xs h-7 px-2"
            >
              Cancel
            </Button>
          ) : (
            <Button size="sm" onClick={onDownload} className="text-xs h-7 px-3">
              Download
            </Button>
          )}
        </div>
      </div>

      {isDownloading && downloadState.kind === "downloading" && (
        <div className="flex flex-col gap-1">
          <div className="h-1.5 w-full rounded-full bg-muted overflow-hidden">
            <div
              className="h-full rounded-full bg-ring transition-all duration-300"
              style={{ width: `${downloadState.percentage}%` }}
            />
          </div>
          <span className="text-[11px] text-muted-foreground">
            {formatBytes(downloadState.bytesDownloaded)} /{" "}
            {formatBytes(downloadState.totalBytes)} ({downloadState.percentage}%)
          </span>
        </div>
      )}

      {downloadState.kind === "error" && (
        <p className="text-xs text-destructive">{downloadState.message}</p>
      )}
    </div>
  );
}

export function LocalModelsField() {
  const [statuses, setStatuses] = useState<LocalModelStatus[]>([]);
  const [downloadStates, setDownloadStates] = useState<Record<string, DownloadState>>({});

  useEffect(() => {
    getLocalModelStatuses().then((s) => {
      setStatuses(s);
      const initial: Record<string, DownloadState> = {};
      for (const item of s) {
        initial[item.model] = item.downloading
          ? { kind: "downloading", percentage: 0, bytesDownloaded: 0, totalBytes: item.size_bytes }
          : { kind: "idle" };
      }
      setDownloadStates(initial);
    });
  }, []);

  useEffect(() => {
    let cancelled = false;
    const unlisteners: Array<() => void> = [];

    const attach = async () => {
      const unProgress = await listen<ModelDownloadProgress>(
        "model-download-progress",
        (e) => {
          const { model, bytes_downloaded, total_bytes, percentage } = e.payload;
          setDownloadStates((prev) => ({
            ...prev,
            [model]: {
              kind: "downloading",
              percentage,
              bytesDownloaded: bytes_downloaded,
              totalBytes: total_bytes,
            },
          }));
        },
      );

      const unComplete = await listen<LocalWhisperModel>("model-download-complete", (e) => {
        const model = e.payload;
        setDownloadStates((prev) => ({ ...prev, [model]: { kind: "idle" } }));
        setStatuses((prev) =>
          prev.map((s) => (s.model === model ? { ...s, downloaded: true, downloading: false } : s)),
        );
        toast.success(`${MODEL_LABELS[model]} downloaded`);
      });

      const unError = await listen<ModelDownloadError>("model-download-error", (e) => {
        const { model, message } = e.payload;
        setDownloadStates((prev) => ({ ...prev, [model]: { kind: "error", message } }));
        setStatuses((prev) =>
          prev.map((s) => (s.model === model ? { ...s, downloading: false } : s)),
        );
      });

      if (cancelled) {
        unProgress();
        unComplete();
        unError();
        return;
      }
      unlisteners.push(unProgress, unComplete, unError);
    };

    attach();
    return () => {
      cancelled = true;
      for (const unlisten of unlisteners) unlisten();
    };
  }, []);

  const handleDownload = async (model: LocalWhisperModel) => {
    setDownloadStates((prev) => ({
      ...prev,
      [model]: { kind: "downloading", percentage: 0, bytesDownloaded: 0, totalBytes: 0 },
    }));
    try {
      await startModelDownload(model);
    } catch (e) {
      setDownloadStates((prev) => ({ ...prev, [model]: { kind: "error", message: String(e) } }));
    }
  };

  const handleCancel = async (model: LocalWhisperModel) => {
    try {
      await cancelModelDownload(model);
      setDownloadStates((prev) => ({ ...prev, [model]: { kind: "idle" } }));
      setStatuses((prev) =>
        prev.map((s) => (s.model === model ? { ...s, downloading: false } : s)),
      );
    } catch (e) {
      toast.error("Couldn't cancel download", { description: String(e) });
    }
  };

  const handleDelete = async (model: LocalWhisperModel) => {
    try {
      await deleteLocalModel(model);
      setStatuses((prev) =>
        prev.map((s) => (s.model === model ? { ...s, downloaded: false } : s)),
      );
      toast.success(`${MODEL_LABELS[model]} deleted`);
    } catch (e) {
      toast.error("Couldn't delete model", { description: String(e) });
    }
  };

  if (statuses.length === 0) return null;

  return (
    <SectionCard title="Local Models">
      <div className="flex flex-col gap-2 mt-3">
        {statuses.map((status) => (
          <ModelRow
            key={status.model}
            status={status}
            downloadState={downloadStates[status.model] ?? { kind: "idle" }}
            onDownload={() => handleDownload(status.model)}
            onCancel={() => handleCancel(status.model)}
            onDelete={() => handleDelete(status.model)}
          />
        ))}
      </div>
    </SectionCard>
  );
}
