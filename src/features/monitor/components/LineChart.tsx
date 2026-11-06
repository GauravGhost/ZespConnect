import { useId, useState, type PointerEvent } from "react";

import { useElementWidth } from "@/hooks/useElementWidth";
import { cn } from "@/lib/utils";
import { formatAge } from "../format";
import { HISTORY_LENGTH } from "../history";

/** Room above the top gridline for its label. */
const TOP_PAD = 14;
const BOTTOM_PAD = 2;
const TOOLTIP_FLIP_PX = 160;

interface Point {
  x: number;
  y: number;
}

/** Runs of consecutive readings; a missing reading breaks the line instead of faking a value. */
function segments(points: (Point | null)[]): Point[][] {
  const runs: Point[][] = [];
  let run: Point[] = [];
  for (const point of points) {
    if (point) {
      run.push(point);
    } else if (run.length > 0) {
      runs.push(run);
      run = [];
    }
  }
  if (run.length > 0) runs.push(run);
  return runs;
}

const linePath = (run: Point[]) =>
  run.map((p, i) => `${i === 0 ? "M" : "L"}${p.x.toFixed(1)},${p.y.toFixed(1)}`).join("");

export function LineChart({
  values,
  times,
  domain,
  format,
  label,
  height,
  detailed = false,
  className,
}: {
  values: (number | null)[];
  times: number[];
  domain: [number, number];
  format: (value: number) => string;
  label: string;
  height: number;
  /** Gridlines, axis labels and the hover readout; off for tile sparklines. */
  detailed?: boolean;
  className?: string;
}) {
  const [element, setElement] = useState<HTMLDivElement | null>(null);
  const width = useElementWidth(element);
  // useId output holds characters that url(#id) does not accept.
  const gradientId = `chart-${useId().replace(/[^\w-]/g, "")}`;
  const [hover, setHover] = useState<number | null>(null);

  const [low, high] = domain;
  const top = detailed ? TOP_PAD : 2;
  const plotHeight = height - top - BOTTOM_PAD;
  const baseline = top + plotHeight;
  const slot = width / (HISTORY_LENGTH - 1);
  const xAt = (index: number) => width - (values.length - 1 - index) * slot;
  const yAt = (value: number) => {
    const share = high > low ? (value - low) / (high - low) : 0;
    return top + (1 - Math.min(1, Math.max(0, share))) * plotHeight;
  };

  const runs = segments(values.map((v, i) => (v === null ? null : { x: xAt(i), y: yAt(v) })));
  const latestTime = times[times.length - 1] ?? 0;

  const onPointerMove = (event: PointerEvent<HTMLDivElement>) => {
    if (!detailed || values.length === 0 || width === 0) return;
    const fromRight = width - (event.clientX - event.currentTarget.getBoundingClientRect().left);
    const index = values.length - 1 - Math.round(fromRight / slot);
    setHover(index >= 0 ? Math.min(index, values.length - 1) : null);
  };

  const hoverValue = hover === null ? null : values[hover];
  const hoverX = hover === null ? 0 : xAt(hover);
  const gridlines = [high, (low + high) / 2, low];

  return (
    <div
      ref={setElement}
      className={cn("relative w-full", className)}
      style={{ height }}
      onPointerMove={onPointerMove}
      onPointerLeave={() => setHover(null)}
    >
      {width > 0 ? (
        <svg width={width} height={height} role="img" aria-label={label} className="block">
          <defs>
            <linearGradient id={gradientId} x1="0" x2="0" y1="0" y2="1">
              <stop offset="0%" stopColor="var(--primary)" stopOpacity={detailed ? 0.22 : 0.18} />
              <stop offset="100%" stopColor="var(--primary)" stopOpacity={0} />
            </linearGradient>
          </defs>

          {detailed
            ? gridlines.map((value) => (
                <g key={value} className="text-muted-foreground">
                  <line
                    x1={0}
                    x2={width}
                    y1={yAt(value)}
                    y2={yAt(value)}
                    stroke="currentColor"
                    strokeOpacity={0.18}
                    strokeDasharray={value === low ? undefined : "3 4"}
                  />
                  <text x={0} y={yAt(value) - 4} fontSize={10} fill="currentColor">
                    {format(value)}
                  </text>
                </g>
              ))
            : null}

          {runs.map((run) => (
            <g key={`${run[0].x}`}>
              <path
                d={`${linePath(run)}L${run[run.length - 1].x.toFixed(1)},${baseline}L${run[0].x.toFixed(1)},${baseline}Z`}
                fill={`url(#${gradientId})`}
              />
              <path
                d={linePath(run)}
                fill="none"
                stroke="var(--primary)"
                strokeWidth={2}
                strokeLinejoin="round"
                strokeLinecap="round"
              />
            </g>
          ))}

          {hover !== null ? (
            <g>
              <line
                x1={hoverX}
                x2={hoverX}
                y1={top}
                y2={baseline}
                stroke="var(--foreground)"
                strokeOpacity={0.35}
              />
              {hoverValue !== null ? (
                <circle
                  cx={hoverX}
                  cy={yAt(hoverValue)}
                  r={4.5}
                  fill="var(--primary)"
                  stroke="var(--card)"
                  strokeWidth={2}
                />
              ) : null}
            </g>
          ) : null}
        </svg>
      ) : null}

      {hover !== null ? (
        <div
          className="bg-popover text-popover-foreground pointer-events-none absolute top-0 rounded-xl px-2.5 py-1.5 text-xs shadow-md ring-1 ring-border"
          style={
            width - hoverX < TOOLTIP_FLIP_PX
              ? { right: width - hoverX + 10 }
              : { left: hoverX + 10 }
          }
        >
          <p className="font-semibold tabular-nums">
            {hoverValue === null ? "No reading" : format(hoverValue)}
          </p>
          <p className="text-muted-foreground">{formatAge(latestTime - times[hover])}</p>
        </div>
      ) : null}
    </div>
  );
}
