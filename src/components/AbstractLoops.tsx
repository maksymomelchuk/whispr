import { useEffect, useMemo, useRef } from "react";

type Tone = "primary" | "warm";

interface Harmonic {
  n: number;
  amp: number;
  rate: number;
  phase: number;
}

interface LoopSpec {
  tone: Tone;
  baseR: number;
  opacity: number;
  width: number;
  spin: number;
  phase0: number;
  blurPx: number;
  harmonics: Harmonic[];
}

const POINTS = 110;
const VIEW = 200;
const CENTER = VIEW / 2;

const TONE_VAR: Record<Tone, string> = {
  primary: "var(--loops-primary)",
  warm: "var(--loops-warm)",
};

function rng(seed: number) {
  let s = seed;
  return () => {
    s = (s * 9301 + 49297) % 233280;
    return s / 233280;
  };
}

function makeHarmonics(seed: number, count: number, ampScale: number): Harmonic[] {
  const r = rng(seed);
  const out: Harmonic[] = [];
  for (let i = 0; i < count; i++) {
    const n = 2 + Math.floor(r() * 4);
    out.push({
      n,
      amp: ampScale * (0.45 + r() * 0.55) * (1 / (1 + (n - 2) * 0.35)),
      rate: (r() * 0.18 + 0.06) * (r() > 0.5 ? 1 : -1),
      phase: r() * Math.PI * 2,
    });
  }
  return out;
}

function buildLoops(): LoopSpec[] {
  const loops: LoopSpec[] = [];

  loops.push({
    tone: "primary",
    baseR: 0.62,
    opacity: 0.78,
    width: 1.35,
    spin: 0.045,
    phase0: 0.4,
    blurPx: 0,
    harmonics: makeHarmonics(11, 3, 0.075),
  });
  loops.push({
    tone: "warm",
    baseR: 0.6,
    opacity: 0.78,
    width: 1.35,
    spin: -0.055,
    phase0: 1.7,
    blurPx: 0,
    harmonics: makeHarmonics(23, 3, 0.085),
  });

  const midConfigs: Array<{ tone: Tone; seed: number; baseR: number; phase0: number; spin: number }> = [
    { tone: "primary", seed: 37, baseR: 0.66, phase0: 2.8, spin: 0.03 },
    { tone: "primary", seed: 53, baseR: 0.58, phase0: 4.1, spin: -0.04 },
    { tone: "primary", seed: 71, baseR: 0.64, phase0: 5.3, spin: 0.022 },
    { tone: "warm", seed: 89, baseR: 0.63, phase0: 0.9, spin: -0.025 },
  ];
  for (const c of midConfigs) {
    loops.push({
      tone: c.tone,
      baseR: c.baseR,
      opacity: 0.28,
      width: 1.05,
      spin: c.spin,
      phase0: c.phase0,
      blurPx: 0.3,
      harmonics: makeHarmonics(c.seed, 3, 0.08),
    });
  }

  const ambientConfigs: Array<{ tone: Tone; seed: number; baseR: number; phase0: number; spin: number }> = [
    { tone: "primary", seed: 101, baseR: 0.7, phase0: 3.5, spin: 0.015 },
    { tone: "primary", seed: 113, baseR: 0.55, phase0: 1.2, spin: -0.018 },
    { tone: "warm", seed: 131, baseR: 0.68, phase0: 4.7, spin: 0.02 },
  ];
  for (const c of ambientConfigs) {
    loops.push({
      tone: c.tone,
      baseR: c.baseR,
      opacity: 0.14,
      width: 0.9,
      spin: c.spin,
      phase0: c.phase0,
      blurPx: 1.2,
      harmonics: makeHarmonics(c.seed, 4, 0.09),
    });
  }

  return loops;
}

function pathString(spec: LoopSpec, t: number, scale: number): string {
  const rot = spec.phase0 + spec.spin * t;
  const cosR = Math.cos(rot);
  const sinR = Math.sin(rot);
  let d = "";
  for (let i = 0; i <= POINTS; i++) {
    const theta = (i / POINTS) * Math.PI * 2;
    let r = spec.baseR;
    for (const h of spec.harmonics) {
      r += h.amp * Math.sin(h.n * theta + h.rate * t + h.phase);
    }
    const px = Math.cos(theta) * r;
    const py = Math.sin(theta) * r;
    const x = (px * cosR - py * sinR) * scale + CENTER;
    const y = (px * sinR + py * cosR) * scale + CENTER;
    d += i === 0 ? `M${x.toFixed(2)},${y.toFixed(2)}` : `L${x.toFixed(2)},${y.toFixed(2)}`;
  }
  return `${d}Z`;
}

interface AbstractLoopsProps {
  active?: boolean;
  className?: string;
  fillMode?: "contain" | "cover";
  scale?: number;
}

export function AbstractLoops({
  active = true,
  className,
  fillMode = "contain",
  scale = 1,
}: AbstractLoopsProps) {
  const loops = useMemo(buildLoops, []);
  const pathRefs = useRef<(SVGPathElement | null)[]>([]);
  const activeRef = useRef(active);
  const scaleRef = useRef(0);

  useEffect(() => {
    activeRef.current = active;
  }, [active]);

  useEffect(() => {
    const reduce = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    const radius = (VIEW / 2) * 0.92 * scale;

    if (reduce) {
      scaleRef.current = radius;
      for (let i = 0; i < loops.length; i++) {
        const el = pathRefs.current[i];
        if (el) el.setAttribute("d", pathString(loops[i], 0, radius));
      }
      return;
    }

    let raf = 0;
    let running = true;
    const t0 = performance.now();
    scaleRef.current = radius;

    const tick = (now: number) => {
      if (!running) return;
      const t = (now - t0) / 1000;
      const target = activeRef.current ? radius : radius * 0.92;
      scaleRef.current += (target - scaleRef.current) * 0.04;

      for (let i = 0; i < loops.length; i++) {
        const el = pathRefs.current[i];
        if (el) el.setAttribute("d", pathString(loops[i], t, scaleRef.current));
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
  }, [loops, scale]);

  return (
    <svg
      viewBox={`0 0 ${VIEW} ${VIEW}`}
      preserveAspectRatio={fillMode === "cover" ? "xMidYMid slice" : "xMidYMid meet"}
      aria-hidden="true"
      className={`abstract-loops ${className ?? ""}`}
      style={{ userSelect: "none", pointerEvents: "none" }}
    >
      {loops.map((spec, i) => (
        <path
          key={i}
          ref={(el) => {
            pathRefs.current[i] = el;
          }}
          fill="none"
          strokeWidth={spec.width}
          strokeLinejoin="round"
          strokeLinecap="round"
          vectorEffect="non-scaling-stroke"
          opacity={spec.opacity}
          style={{
            stroke: TONE_VAR[spec.tone],
            filter: spec.blurPx > 0 ? `blur(${spec.blurPx}px)` : undefined,
          }}
        />
      ))}
    </svg>
  );
}
