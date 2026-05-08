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

const TOOLTIP_GAP = 6;
const TOOLTIP_MARGIN = 8;

/// Hover/focus tooltip badge. The bubble is rendered via portal so it
/// escapes any `overflow: hidden` ancestor (collapsible card body, etc).
/// We swallow click + mousedown so placing one inside a `<label>` doesn't
/// trigger the wrapped radio/checkbox.
export function InfoTip({ text, ariaLabel }: InfoTipProps) {
  const triggerRef = useRef<HTMLSpanElement | null>(null);
  const bubbleRef = useRef<HTMLDivElement | null>(null);
  const [open, setOpen] = useState(false);
  const [pos, setPos] = useState<{
    top: number;
    left: number;
    placement: "top" | "bottom";
  } | null>(null);

  const updatePosition = useCallback(() => {
    const trigger = triggerRef.current;
    const bubble = bubbleRef.current;
    if (!trigger || !bubble) return;
    const tr = trigger.getBoundingClientRect();
    const br = bubble.getBoundingClientRect();
    const vw = window.innerWidth;
    const vh = window.innerHeight;
    const triggerCenter = tr.left + tr.width / 2;

    // Prefer above, fall back to below if there's not enough room.
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

    setPos({ top, left, placement });
  }, []);

  useLayoutEffect(() => {
    if (!open) return;
    updatePosition();
    const onScroll = () => updatePosition();
    window.addEventListener("scroll", onScroll, true);
    window.addEventListener("resize", onScroll);
    return () => {
      window.removeEventListener("scroll", onScroll, true);
      window.removeEventListener("resize", onScroll);
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
