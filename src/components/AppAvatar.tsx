import { useEffect, useState } from "react";

import { getAppIcon } from "../lib/api";
import { cn } from "../lib/utils";

interface AppAvatarProps {
  name: string;
  bundleId: string | null | undefined;
  size?: number;
  className?: string;
}

export function AppAvatar({
  name,
  bundleId,
  size = 22,
  className,
}: AppAvatarProps) {
  const [src, setSrc] = useState<string | null>(null);
  const [loaded, setLoaded] = useState(false);

  useEffect(() => {
    if (!bundleId) return;
    let cancelled = false;
    setLoaded(false);
    getAppIcon(bundleId)
      .then((url) => {
        if (!cancelled) setSrc(url ?? null);
      })
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, [bundleId]);

  return (
    <div
      className={cn("relative shrink-0", className)}
      style={{ width: size, height: size }}
    >
      <div
        className={cn(
          "absolute inset-0 flex items-center justify-center rounded-[5px] bg-muted transition-opacity duration-150",
          loaded && src ? "opacity-0" : "opacity-100",
        )}
      >
        <span className="select-none text-[11px] font-medium text-muted-foreground">
          {name[0]?.toUpperCase() ?? "?"}
        </span>
      </div>
      {src && (
        <img
          src={src}
          alt={name}
          draggable={false}
          onLoad={() => setLoaded(true)}
          className={cn(
            "absolute inset-0 select-none rounded-[5px] object-contain transition-opacity duration-150",
            loaded ? "opacity-100" : "opacity-0",
          )}
          style={{ width: size, height: size }}
        />
      )}
    </div>
  );
}
