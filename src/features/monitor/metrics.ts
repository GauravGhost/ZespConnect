import type { LucideIcon } from "lucide-react";
import { ArrowDownUp, Cpu, Gauge, MemoryStick, Thermometer, Zap } from "lucide-react";

import { formatBytes, formatRate } from "./format";
import { niceCeil, type TimedSample } from "./history";

export type MetricId = "cpu" | "memory" | "fps" | "temperature" | "power" | "network";

export type MetricLevel = "normal" | "warning" | "critical";

export interface Metric {
  id: MetricId;
  label: string;
  icon: LucideIcon;
  value: (sample: TimedSample) => number | null;
  format: (value: number) => string;
  /** Secondary line under the value, from the newest sample. */
  detail: (sample: TimedSample) => string | null;
  /** The chart's y range for the visible history. */
  domain: (values: number[], latest: TimedSample | undefined) => [number, number];
  level?: (value: number) => MetricLevel;
}

const percentDomain = (): [number, number] => [0, 100];
const autoDomain = (values: number[]): [number, number] => [0, niceCeil(Math.max(0, ...values))];

const KB = 1024;
/** Battery temperatures where Android itself starts to throttle and then to warn. */
const WARM_C = 40;
const HOT_C = 45;

export const METRICS: Metric[] = [
  {
    id: "cpu",
    label: "CPU",
    icon: Cpu,
    value: (s) => s.cpu,
    format: (v) => `${Math.round(v)}%`,
    detail: (s) => (s.cores.length > 0 ? `${s.cores.length} cores` : null),
    domain: percentDomain,
  },
  {
    id: "memory",
    label: "Memory",
    icon: MemoryStick,
    value: (s) =>
      s.memUsedKb !== null && s.memTotalKb ? (s.memUsedKb / s.memTotalKb) * 100 : null,
    format: (v) => `${Math.round(v)}%`,
    detail: (s) =>
      s.memUsedKb !== null && s.memTotalKb !== null
        ? `${formatBytes(s.memUsedKb * KB)} of ${formatBytes(s.memTotalKb * KB)}`
        : null,
    domain: percentDomain,
  },
  {
    id: "fps",
    label: "Frame rate",
    icon: Gauge,
    value: (s) => s.fps,
    format: (v) => `${Math.round(v)} fps`,
    detail: (s) => {
      const app = s.foreground?.label ?? "No app in front";
      return s.jank !== null ? `${app} · ${Math.round(s.jank)}% janky` : app;
    },
    domain: (values, latest) => [
      0,
      Math.max(latest?.refreshHz ? Math.round(latest.refreshHz) : 60, ...values),
    ],
  },
  {
    id: "temperature",
    label: "Battery temp",
    icon: Thermometer,
    value: (s) => s.batteryTempC,
    format: (v) => `${v.toFixed(1)} °C`,
    detail: (s) => (s.batteryLevel !== null ? `Battery ${s.batteryLevel}%` : null),
    domain: (values) => {
      if (values.length === 0) return [20, HOT_C];
      const low = Math.floor(Math.min(...values) / 5) * 5 - 5;
      const high = Math.ceil(Math.max(...values) / 5) * 5 + 5;
      return [Math.max(0, low), Math.max(high, HOT_C)];
    },
    level: (v) => (v >= HOT_C ? "critical" : v >= WARM_C ? "warning" : "normal"),
  },
  {
    id: "power",
    label: "Power",
    icon: Zap,
    value: (s) => s.batteryWatts,
    format: (v) => `${v.toFixed(v < 10 ? 2 : 1)} W`,
    detail: (s) => {
      if (s.batteryMa === null) return null;
      const flow = s.charging ? "charging" : "draw";
      return `${Math.round(s.batteryMa)} mA ${flow}`;
    },
    domain: autoDomain,
  },
  {
    id: "network",
    label: "Network",
    icon: ArrowDownUp,
    value: (s) => (s.netRxBps !== null && s.netTxBps !== null ? s.netRxBps + s.netTxBps : null),
    format: formatRate,
    detail: (s) =>
      s.netRxBps !== null && s.netTxBps !== null
        ? `↓ ${formatRate(s.netRxBps)} · ↑ ${formatRate(s.netTxBps)}`
        : null,
    domain: autoDomain,
  },
];

export function metricById(id: MetricId): Metric {
  return METRICS.find((metric) => metric.id === id) ?? METRICS[0];
}
