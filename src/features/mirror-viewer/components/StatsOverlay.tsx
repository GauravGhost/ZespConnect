import { useEffect, useRef, useState, type RefObject } from "react";

import type { StreamCounters } from "../hooks";

const SAMPLE_MS = 1000;

interface Sample {
  fps: number;
  mbps: number;
  dropped: number;
  queue: number;
}

/** Game Mode's stream stats, sampled once a second from the viewer's counters. */
export function StatsOverlay({ counters }: Readonly<{ counters: RefObject<StreamCounters> }>) {
  const [sample, setSample] = useState<Sample | null>(null);
  const lastRef = useRef({ frames: 0, bytes: 0, at: performance.now() });

  useEffect(() => {
    const current = counters.current;
    lastRef.current = { frames: current.frames, bytes: current.bytes, at: performance.now() };
    const timer = window.setInterval(() => {
      const now = performance.now();
      const { frames, bytes, decoder } = counters.current;
      const last = lastRef.current;
      const seconds = Math.max((now - last.at) / 1000, 0.001);
      setSample({
        fps: Math.round((frames - last.frames) / seconds),
        mbps: ((bytes - last.bytes) * 8) / seconds / 1_000_000,
        dropped: decoder?.dropped ?? 0,
        queue: decoder?.pending ?? 0,
      });
      lastRef.current = { frames, bytes, at: now };
    }, SAMPLE_MS);
    return () => window.clearInterval(timer);
  }, [counters]);

  if (!sample) return null;
  return (
    <div className="pointer-events-none absolute top-2 left-2 z-20 rounded-md bg-black/70 px-2 py-1 font-mono text-[11px] leading-4 text-white tabular-nums">
      <div>{sample.fps} fps</div>
      <div>{sample.mbps.toFixed(1)} Mbps</div>
      <div>
        queue {sample.queue} · drops {sample.dropped}
      </div>
    </div>
  );
}
