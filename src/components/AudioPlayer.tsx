import { PauseIcon, PlayIcon } from "@phosphor-icons/react";
import { useEffect, useRef, useState } from "react";

import { Button } from "@/components/ui/button";
import { readRecordingWav } from "@/lib/api";
import { formatClock } from "@/lib/formatDuration";

/// Plays a saved recording via a WAV blob: the clip is stored as FLAC and
/// decoded to WAV in Rust because WKWebView's `<audio>` can't decode FLAC. The
/// WAV is fetched lazily on first play — not on mount — so a page full of
/// recordings doesn't decode every clip into memory at once.
export function AudioPlayer({ entryId }: { entryId: string }) {
  const audioRef = useRef<HTMLAudioElement | null>(null);
  const objectUrlRef = useRef<string | null>(null);
  const playWhenReady = useRef(false);
  const [src, setSrc] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState(false);
  const [playing, setPlaying] = useState(false);
  const [current, setCurrent] = useState(0);
  const [duration, setDuration] = useState(0);

  useEffect(() => {
    return () => {
      if (objectUrlRef.current) URL.revokeObjectURL(objectUrlRef.current);
    };
  }, []);

  // The FLAC is encoded off the paste path, so for a just-saved entry the file
  // can lag the history-updated event that renders this row. Retry a few times
  // before giving up rather than getting stuck on a hard error.
  const loadSrc = (remaining: number) => {
    readRecordingWav(entryId)
      .then((bytes) => {
        const url = URL.createObjectURL(
          new Blob([bytes], { type: "audio/wav" }),
        );
        objectUrlRef.current = url;
        setLoading(false);
        setSrc(url);
      })
      .catch(() => {
        if (remaining > 0) {
          window.setTimeout(() => loadSrc(remaining - 1), 500);
        } else {
          setLoading(false);
          setError(true);
        }
      });
  };

  const togglePlay = () => {
    if (!src) {
      playWhenReady.current = true;
      setLoading(true);
      loadSrc(4);
      return;
    }
    const el = audioRef.current;
    if (!el) return;
    if (el.paused) {
      void el.play();
    } else {
      el.pause();
    }
  };

  // preload="metadata" buffers only metadata, so `canplay` never fires until
  // playback is requested — auto-play the just-loaded clip off loadedmetadata,
  // whose play() call forces the data load.
  const handleLoadedMetadata = (
    event: React.SyntheticEvent<HTMLAudioElement>,
  ) => {
    setDuration(event.currentTarget.duration);
    if (!playWhenReady.current) return;
    playWhenReady.current = false;
    void event.currentTarget.play();
  };

  const handleSeek = (event: React.ChangeEvent<HTMLInputElement>) => {
    const el = audioRef.current;
    if (!el) return;
    el.currentTime = Number(event.target.value);
    setCurrent(el.currentTime);
  };

  if (error) {
    return (
      <p className="text-help italic text-muted-foreground/70">
        Recording unavailable.
      </p>
    );
  }

  return (
    <div className="flex items-center gap-2">
      {src && (
        <audio
          ref={audioRef}
          src={src}
          preload="metadata"
          onLoadedMetadata={handleLoadedMetadata}
          onTimeUpdate={(e) => setCurrent(e.currentTarget.currentTime)}
          onPlay={() => setPlaying(true)}
          onPause={() => setPlaying(false)}
          onEnded={() => setPlaying(false)}
          onError={() => setError(true)}
        />
      )}
      <Button
        type="button"
        variant="ghost"
        size="icon-sm"
        className="shrink-0 text-muted-foreground"
        aria-label={playing ? "Pause recording" : "Play recording"}
        disabled={loading}
        onClick={togglePlay}
      >
        {playing ? <PauseIcon weight="fill" /> : <PlayIcon weight="fill" />}
      </Button>
      <input
        type="range"
        min={0}
        max={duration || 0}
        step={0.1}
        value={current}
        onChange={handleSeek}
        aria-label="Seek recording"
        disabled={!src || !duration}
        className="h-1 flex-1 cursor-pointer rounded-full accent-primary disabled:cursor-not-allowed"
      />
      <span className="shrink-0 font-mono text-kbd tabular-nums text-muted-foreground/70">
        {formatClock(current)} / {formatClock(duration)}
      </span>
    </div>
  );
}
