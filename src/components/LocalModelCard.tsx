import { listen } from "@tauri-apps/api/event";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import {
  DownloadSimpleIcon,
  FolderOpenIcon,
  TrashIcon,
  XIcon,
} from "@phosphor-icons/react";
import { useEffect, useReducer, useState } from "react";
import { toast } from "sonner";

import { NvidiaLogo } from "@/assets/NvidiaLogo";
import { OpenAiLogo } from "@/assets/OpenAiLogo";
import { Button } from "@/components/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";

import {
  cancelModelDownload,
  deleteLocalModel,
  getLocalModelPath,
  startModelDownload,
} from "../lib/api";
import { localModelDownloadReducer } from "../lib/localModelDownloadReducer";
import type {
  LocalModelStatus,
  LocalWhisperModel,
  ModelDownloadError,
  ModelDownloadProgress,
} from "../lib/types";

const MODEL_LABELS: Record<LocalWhisperModel, string> = {
  large_v3: "Large v3",
  large_v3_turbo: "Large v3 Turbo",
  parakeet: "Parakeet TDT",
};

const MODEL_SIZE_LABELS: Record<LocalWhisperModel, string> = {
  large_v3: "~2.9 GB",
  large_v3_turbo: "~1.5 GB",
  parakeet: "~575 MB",
};

const MODEL_LOGOS: Record<LocalWhisperModel, typeof OpenAiLogo> = {
  large_v3: OpenAiLogo,
  large_v3_turbo: OpenAiLogo,
  parakeet: NvidiaLogo,
};

function formatBytes(bytes: number): string {
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(0)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

interface Props {
  status: LocalModelStatus;
}

export function LocalModelCard({ status }: Props) {
  const [downloadState, dispatch] = useReducer(
    localModelDownloadReducer,
    status.downloading
      ? { kind: "downloading", percentage: 0, bytesDownloaded: 0, totalBytes: status.size_bytes }
      : { kind: "idle" },
  );
  const [downloaded, setDownloaded] = useState(status.downloaded);

  const { model } = status;
  const ModelLogo = MODEL_LOGOS[model];

  useEffect(() => {
    let cancelled = false;
    const unlisteners: Array<() => void> = [];

    const attach = async () => {
      const unProgress = await listen<ModelDownloadProgress>("model-download-progress", (e) => {
        if (e.payload.model !== model) return;
        const { bytes_downloaded, total_bytes, percentage } = e.payload;
        dispatch({ type: "progress", bytesDownloaded: bytes_downloaded, totalBytes: total_bytes, percentage });
      });

      const unComplete = await listen<LocalWhisperModel>("model-download-complete", (e) => {
        if (e.payload !== model) return;
        dispatch({ type: "complete" });
        setDownloaded(true);
        toast.success(`${MODEL_LABELS[model]} downloaded`);
      });

      const unError = await listen<ModelDownloadError>("model-download-error", (e) => {
        if (e.payload.model !== model) return;
        dispatch({ type: "error", message: e.payload.message });
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
  }, [model]);

  const handleDownload = async () => {
    dispatch({ type: "start" });
    try {
      await startModelDownload(model);
    } catch (e) {
      dispatch({ type: "error", message: String(e) });
    }
  };

  const handleCancel = async () => {
    try {
      await cancelModelDownload(model);
      dispatch({ type: "cancel" });
    } catch (e) {
      toast.error("Couldn't cancel download", { description: String(e) });
    }
  };

  const handleDelete = async () => {
    try {
      await deleteLocalModel(model);
      setDownloaded(false);
      toast.success(`${MODEL_LABELS[model]} deleted`);
    } catch (e) {
      toast.error("Couldn't delete model", { description: String(e) });
    }
  };

  const handleReveal = async () => {
    try {
      const path = await getLocalModelPath(model);
      await revealItemInDir(path);
    } catch (e) {
      toast.error("Couldn't open Finder", { description: String(e) });
    }
  };

  const isDownloading = downloadState.kind === "downloading";

  return (
    <div className="flex flex-col gap-2 rounded-lg border border-border bg-card px-4 py-3">
      <div className="flex items-center gap-3">
        <ModelLogo className="h-8 w-8 shrink-0 rounded-md" />
        <div className="flex flex-1 flex-col gap-0.5 min-w-0">
          <span className="text-sm font-medium leading-tight">{MODEL_LABELS[model]}</span>
          <span className="text-xs text-muted-foreground">{MODEL_SIZE_LABELS[model]}</span>
        </div>

        <div className="flex items-center gap-1 shrink-0">
          {downloaded ? (
            <>
              <Tooltip>
                <TooltipTrigger asChild>
                  <Button
                    variant="ghost"
                    size="icon-sm"
                    onClick={handleReveal}
                    aria-label="Show in Finder"
                  >
                    <FolderOpenIcon size={15} />
                  </Button>
                </TooltipTrigger>
                <TooltipContent>Show in Finder</TooltipContent>
              </Tooltip>
              <Tooltip>
                <TooltipTrigger asChild>
                  <Button
                    variant="ghost"
                    size="icon-sm"
                    onClick={handleDelete}
                    aria-label="Delete model"
                    className="text-muted-foreground hover:text-destructive"
                  >
                    <TrashIcon size={15} />
                  </Button>
                </TooltipTrigger>
                <TooltipContent>Delete</TooltipContent>
              </Tooltip>
            </>
          ) : isDownloading ? (
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  variant="ghost"
                  size="icon-sm"
                  onClick={handleCancel}
                  aria-label="Cancel download"
                >
                  <XIcon size={15} />
                </Button>
              </TooltipTrigger>
              <TooltipContent>Cancel</TooltipContent>
            </Tooltip>
          ) : (
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  variant="ghost"
                  size="icon-sm"
                  onClick={handleDownload}
                  aria-label="Download model"
                >
                  <DownloadSimpleIcon size={15} />
                </Button>
              </TooltipTrigger>
              <TooltipContent>Download</TooltipContent>
            </Tooltip>
          )}
        </div>
      </div>

      {isDownloading && (
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
