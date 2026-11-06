import { AlertTriangle, Flame } from "lucide-react";

import { cn } from "@/lib/utils";
import type { TimedSample } from "../history";
import type { Metric, MetricLevel } from "../metrics";
import { LineChart } from "./LineChart";

const SPARK_HEIGHT = 36;

const LEVEL_BADGE: Record<Exclude<MetricLevel, "normal">, { label: string; className: string }> = {
  warning: { label: "Warm", className: "bg-warning/15 text-foreground" },
  critical: { label: "Hot", className: "bg-destructive/15 text-foreground" },
};

export function LevelBadge({ level }: { level: MetricLevel }) {
  if (level === "normal") return null;
  const badge = LEVEL_BADGE[level];
  const Icon = level === "critical" ? Flame : AlertTriangle;
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-[11px] font-medium",
        badge.className,
      )}
    >
      <Icon className={cn("size-3", level === "critical" ? "text-destructive" : "text-warning")} />
      {badge.label}
    </span>
  );
}

/** A live reading with its recent trend; selecting it puts the metric in the large chart. */
export function VitalTile({
  metric,
  history,
  selected,
  onSelect,
}: {
  metric: Metric;
  history: TimedSample[];
  selected: boolean;
  onSelect: () => void;
}) {
  const latest = history[history.length - 1];
  const value = latest ? metric.value(latest) : null;
  const values = history.map(metric.value);
  const present = values.filter((v): v is number => v !== null);
  const level = value !== null && metric.level ? metric.level(value) : "normal";
  const Icon = metric.icon;

  return (
    <button
      type="button"
      onClick={onSelect}
      aria-pressed={selected}
      className={cn(
        "bg-card flex flex-col gap-2 overflow-hidden rounded-[1.5rem] border border-border/60 p-3.5 pb-0 text-left shadow-sm transition-[box-shadow,background-color] duration-150 ease-emphasized outline-none",
        "hover:bg-muted/40 focus-visible:ring-ring/50 focus-visible:ring-[3px]",
        selected && "ring-primary ring-2",
      )}
    >
      <div className="flex items-center gap-2">
        <span className="bg-muted/70 text-primary flex size-7 items-center justify-center rounded-full">
          <Icon className="size-3.5" />
        </span>
        <span className="text-muted-foreground flex-1 text-xs font-semibold tracking-wide uppercase">
          {metric.label}
        </span>
        <LevelBadge level={level} />
      </div>
      <div className="min-w-0">
        <p className="text-2xl font-semibold tabular-nums">
          {value === null ? "-" : metric.format(value)}
        </p>
        <p className="text-muted-foreground truncate text-xs">
          {(latest && metric.detail(latest)) ?? (latest ? "Not reported by this phone" : " ")}
        </p>
      </div>
      <LineChart
        values={values}
        times={history.map((s) => s.at)}
        domain={metric.domain(present, latest)}
        format={metric.format}
        label={`${metric.label} trend`}
        height={SPARK_HEIGHT}
        className="-mx-3.5 w-[calc(100%+1.75rem)]"
      />
    </button>
  );
}
