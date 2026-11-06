import type { PerfSample } from "@/types";

/** Two minutes at the phone's one sample a second. */
export const HISTORY_LENGTH = 120;

export interface TimedSample extends PerfSample {
  /** When the sample reached the computer, in epoch milliseconds. */
  at: number;
}

export function appendSample(
  history: TimedSample[],
  sample: TimedSample,
  max = HISTORY_LENGTH,
): TimedSample[] {
  const next = history.length >= max ? history.slice(history.length - max + 1) : history.slice();
  next.push(sample);
  return next;
}

export interface SeriesStats {
  min: number;
  max: number;
  avg: number;
}

export function seriesStats(values: (number | null)[]): SeriesStats | null {
  const present = values.filter((value): value is number => value !== null);
  if (present.length === 0) return null;
  const sum = present.reduce((total, value) => total + value, 0);
  return { min: Math.min(...present), max: Math.max(...present), avg: sum / present.length };
}

/** A round axis top above `value`: 1, 2 or 5 times a power of ten. */
export function niceCeil(value: number): number {
  if (value <= 0) return 1;
  const power = 10 ** Math.floor(Math.log10(value));
  const step = [1, 2, 5, 10].find((factor) => factor * power >= value) ?? 10;
  return step * power;
}
