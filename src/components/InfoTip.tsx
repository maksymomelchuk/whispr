import {
  useCallback,
  useLayoutEffect,
  useRef,
  useState,
} from "react";
import { createPortal } from "react-dom";

interface InfoTipProps {
  text: string;
  ariaLabel?: string;
}

interface Pos {
  top: number;
  left: number;
  placement: "top" | "bottom";
}

const TOOLTIP_GAP = 6;
const TOOLTIP_MARGIN = 8;

/// Hover/focus tooltip. Portaled so it escapes `overflow: hidden`
/// ancestors. Click + mousedown are swallowed because the badge often
/// sits inside a `<label>` whose wrapped input would otherwise toggle.
export function InfoTip({ text, ariaLabel }: InfoTipProps) {
  const triggerRef = useRef<HTMLSpanElement | null>(null);
  const bubbleRef = useRef<HTMLDivElement | null>(null);
  const [open, setOpen] = useState(false);
  const [pos, setPos] = useState<Pos | null>(null);

  const updatePosition = useCallback(() => {
    const trigger = triggerRef.current;
    const bubble = bubbleRef.current;
    if (!trigger || !bubble) return;
    const tr = trigger.getBoundingClientRect();
    const br = bubble.getBoundingClientRect();
    const vw = window.innerWidth;
    const vh = window.innerHeight;
    const triggerCenter = tr.left + tr.width / 2;

    let placement: "top" | "bottom" = "top";
    let top = tr.top - br.height - TOOLTIP_GAP;
    if (top < TOOLTIP_MARGIN) {
      placement = "bottom";
      top = tr.bottom + TOOLTIP_GAP;
    }
    if (top + br.height > vh - TOOLTIP_MARGIN) {
      top = Math.max(TOOLTIP_MARGIN, vh - br.height - TOOLTIP_MARGIN);
    }

    let left = triggerCenter - br.width / 2;
    left = Math.min(
      Math.max(left, TOOLTIP_MARGIN),
      vw - br.width - TOOLTIP_MARGIN,
    );

    // Bail out on identical positions so scroll on an unrelated ancestor
    // doesn't trigger a no-op re-render of the portaled bubble.
    setPos((prev) =>
      prev &&
      prev.top === top &&
      prev.left === left &&
      prev.placement === placement
        ? prev
        : { top, left, placement },
    );
  }, []);

  useLayoutEffect(() => {
    if (!open) return;
    updatePosition();
    let raf: number | null = null;
    const schedule = () => {
      if (raf !== null) return;
      raf = requestAnimationFrame(() => {
        raf = null;
        updatePosition();
      });
    };
    window.addEventListener("scroll", schedule, true);
    window.addEventListener("resize", schedule);
    return () => {
      if (raf !== null) cancelAnimationFrame(raf);
      window.removeEventListener("scroll", schedule, true);
      window.removeEventListener("resize", schedule);
    };
  }, [open, updatePosition]);

  return (
    <>
      <span
        ref={triggerRef}
        className="info-tip"
        aria-label={ariaLabel ?? text}
        tabIndex={0}
        onMouseEnter={() => setOpen(true)}
        onMouseLeave={() => setOpen(false)}
        onFocus={() => setOpen(true)}
        onBlur={() => setOpen(false)}
        onClick={(e) => {
          e.preventDefault();
          e.stopPropagation();
        }}
        onMouseDown={(e) => e.preventDefault()}
      >
        ?
      </span>
      {open &&
        createPortal(
          <div
            ref={bubbleRef}
            className="info-tip-bubble"
            role="tooltip"
            style={{
              top: pos?.top ?? -9999,
              left: pos?.left ?? -9999,
              visibility: pos ? "visible" : "hidden",
            }}
          >
            {text}
          </div>,
          document.body,
        )}
    </>
  );
}
