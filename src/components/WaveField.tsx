import { useEffect, useRef } from "react";

const NUM_BARS = 96;
const BAR_BASE_HEIGHT = 76;
const MAIN_BAR_WIDTH = 3;
const ECHO_BAR_WIDTH = 4;
const HALO_BAR_WIDTH = 5;
const MIN_SCALE = 0.05;
const MAX_SCALE = 1.0;
const AMP_LERP = 0.045;
const READY_AMP = 1.0;
const IDLE_AMP = 0.22;
const WIDTH_FLOOR = 0.55;
const ECHO_DELAY_SECONDS = 0.22;
const HALO_DELAY_SECONDS = 0.46;
const ECHO_OPACITY = 0.18;
const HALO_OPACITY = 0.09;
const ECHO_BLUR_PX = 0.5;
const HALO_BLUR_PX = 1.6;
const BAR_MASK =
  "linear-gradient(to bottom, transparent, black 14%, black 86%, transparent)";
const LAYER_TRANSITION = "opacity 700ms cubic-bezier(0.16, 1, 0.3, 1)";

function computeScale(i: number, t: number, amp: number): number {
  const x = i / (NUM_BARS - 1);

  // Exponent < 1 widens the shoulders — without it the silhouette reads as
  // a single hump rather than a band of waveform.
  const env = Math.pow(Math.sin(Math.PI * x), 0.55);

  // w3 travels against w1/w2 to break visible periodicity — drop it and the
  // wave loops obviously after a few seconds.
  const w1 = Math.sin(2 * Math.PI * (x * 1.6 - t * 0.17));
  const w2 = Math.sin(2 * Math.PI * (x * 4.2 - t * 0.48));
  const w3 = Math.sin(2 * Math.PI * (x * 10.8 + t * 0.92));

  // Two traveling Gaussians briefly amplify a region of the wave, the way a
  // syllable lands in real speech. Without them the level is flat over the
  // width — competent but uneventful.
  const phase1 = ((t * 0.27) % 1.4) - 0.2;
  const phase2 = 1.2 - ((t * 0.19) % 1.4);
  const burst1 = Math.exp(-Math.pow((x - phase1) * 4.5, 2));
  const burst2 = Math.exp(-Math.pow((x - phase2) * 5.5, 2));

  const blend =
    w1 * 0.28 + w2 * 0.42 + w3 * 0.15 + burst1 * 0.32 + burst2 * 0.22;
  const normalized = Math.max(0, Math.min(1, (blend + 1) / 2));

  // Power < 1 lifts mid-values toward peaks so the wave reads as "speaking"
  // instead of "humming".
  const punched = Math.pow(normalized, 0.78);

  const breath = 0.78 + 0.22 * Math.sin(2 * Math.PI * t * 0.13);

  const raw = punched * env * breath;
  return Math.max(
    MIN_SCALE,
    Math.min(MAX_SCALE, MIN_SCALE + (MAX_SCALE - MIN_SCALE) * raw * amp),
  );
}

function staticScale(i: number): number {
  const x = i / (NUM_BARS - 1);
  const env = Math.pow(Math.sin(Math.PI * x), 0.55);
  return MIN_SCALE + (MAX_SCALE - MIN_SCALE) * env * 0.55;
}

// Tall bars are full-width, short bars taper — mechanical grids don't.
function widthScale(heightScale: number): number {
  return WIDTH_FLOOR + (1 - WIDTH_FLOOR) * heightScale;
}

interface LayerProps {
  refs: React.MutableRefObject<(HTMLDivElement | null)[]>;
  width: number;
  opacity: number;
  blurPx?: number;
}

function WaveLayer({ refs, width, opacity, blurPx = 0 }: LayerProps) {
  return (
    <div
      className="absolute inset-0 flex items-center justify-between px-1"
      style={{
        opacity,
        transition: LAYER_TRANSITION,
        filter: blurPx > 0 ? `blur(${blurPx}px)` : undefined,
        willChange: blurPx > 0 ? "filter, opacity" : "opacity",
      }}
    >
      {Array.from({ length: NUM_BARS }, (_, i) => (
        <div
          key={i}
          ref={(el) => {
            refs.current[i] = el;
          }}
          className="shrink-0 rounded-full bg-primary"
          style={{
            width: `${width}px`,
            height: `${BAR_BASE_HEIGHT}px`,
            maskImage: BAR_MASK,
            WebkitMaskImage: BAR_MASK,
            transformOrigin: "center",
            transform: `scaleY(${MIN_SCALE})`,
            willChange: "transform",
          }}
        />
      ))}
    </div>
  );
}

export function WaveField({ ready }: { ready: boolean }) {
  const mainBars = useRef<(HTMLDivElement | null)[]>([]);
  const echoBars = useRef<(HTMLDivElement | null)[]>([]);
  const haloBars = useRef<(HTMLDivElement | null)[]>([]);
  const amp = useRef(0);
  const readyRef = useRef(ready);

  useEffect(() => {
    readyRef.current = ready;
  }, [ready]);

  useEffect(() => {
    const reduce = window.matchMedia("(prefers-reduced-motion: reduce)").matches;

    if (reduce) {
      for (let i = 0; i < NUM_BARS; i++) {
        const s = staticScale(i);
        const main = mainBars.current[i];
        const echo = echoBars.current[i];
        const halo = haloBars.current[i];
        if (main) main.style.transform = `scaleX(${widthScale(s)}) scaleY(${s})`;
        if (echo) echo.style.transform = `scaleY(${s})`;
        if (halo) halo.style.transform = `scaleY(${s})`;
      }
      return;
    }

    let raf = 0;
    let running = true;
    const t0 = performance.now();

    const tick = (now: number) => {
      if (!running) return;
      const t = (now - t0) / 1000;
      const target = readyRef.current ? READY_AMP : IDLE_AMP;
      amp.current += (target - amp.current) * AMP_LERP;

      for (let i = 0; i < NUM_BARS; i++) {
        const sMain = computeScale(i, t, amp.current);
        const sEcho = computeScale(i, t - ECHO_DELAY_SECONDS, amp.current);
        const sHalo = computeScale(i, t - HALO_DELAY_SECONDS, amp.current);
        const main = mainBars.current[i];
        const echo = echoBars.current[i];
        const halo = haloBars.current[i];
        if (main) main.style.transform = `scaleX(${widthScale(sMain)}) scaleY(${sMain})`;
        if (echo) echo.style.transform = `scaleY(${sEcho})`;
        if (halo) halo.style.transform = `scaleY(${sHalo})`;
      }

      raf = requestAnimationFrame(tick);
    };

    raf = requestAnimationFrame(tick);

    const onVisibility = () => {
      if (document.hidden) {
        running = false;
        cancelAnimationFrame(raf);
      } else if (!running) {
        running = true;
        raf = requestAnimationFrame(tick);
      }
    };
    document.addEventListener("visibilitychange", onVisibility);

    return () => {
      running = false;
      cancelAnimationFrame(raf);
      document.removeEventListener("visibilitychange", onVisibility);
    };
  }, []);

  return (
    <div
      className="relative w-full h-20 overflow-hidden"
      aria-hidden="true"
      style={{
        userSelect: "none",
        pointerEvents: "none",
        maskImage:
          "linear-gradient(to right, transparent, black 5%, black 95%, transparent)",
        WebkitMaskImage:
          "linear-gradient(to right, transparent, black 5%, black 95%, transparent)",
      }}
    >
      <div className="absolute inset-x-0 top-1/2 -translate-y-1/2 h-px bg-foreground/[0.04]" />

      <WaveLayer
        refs={haloBars}
        width={HALO_BAR_WIDTH}
        opacity={ready ? HALO_OPACITY : 0}
        blurPx={HALO_BLUR_PX}
      />
      <WaveLayer
        refs={echoBars}
        width={ECHO_BAR_WIDTH}
        opacity={ready ? ECHO_OPACITY : 0}
        blurPx={ECHO_BLUR_PX}
      />
      <WaveLayer
        refs={mainBars}
        width={MAIN_BAR_WIDTH}
        opacity={ready ? 1 : 0.5}
      />
    </div>
  );
}
