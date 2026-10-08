import type { ReactNode } from "react";

import { formatClock, useClock } from "@/hooks/useClock";
import { cn } from "@/lib/utils";

/**
 * A stylised Android handset: bezel, punch-hole camera, status bar and gesture
 * pill. The screen is whatever `children` renders; `status` fills the right
 * side of the status bar and `statusLeft` replaces the clock on the left.
 */
export function PhoneFrame({
  children,
  status,
  statusLeft,
  className,
}: {
  children: ReactNode;
  status?: ReactNode;
  statusLeft?: ReactNode;
  className?: string;
}) {
  const now = useClock();

  return (
    <div className={cn("relative w-[320px] shrink-0 select-none", className)}>
      {/* Hardware keys: power + volume rocker. */}
      <span
        aria-hidden="true"
        className="bg-border absolute top-28 -right-[3px] h-12 w-[3px] rounded-r-full"
      />
      <span
        aria-hidden="true"
        className="bg-border absolute top-44 -right-[3px] h-20 w-[3px] rounded-r-full"
      />

      <div className="bg-muted rounded-[2.75rem] p-2 shadow-xl ring-1 ring-border">
        <div className="bg-background relative flex h-[584px] flex-col overflow-hidden rounded-[2.25rem]">
          {/* Wallpaper glow behind the shade, tinted by the active theme. */}
          <div
            aria-hidden="true"
            className="from-primary/20 pointer-events-none absolute inset-x-0 top-0 h-56 bg-gradient-to-b to-transparent"
          />

          <div className="relative flex h-8 shrink-0 items-center justify-between px-6 pt-1 text-xs font-semibold">
            <span className="min-w-0 truncate tabular-nums">{statusLeft ?? formatClock(now)}</span>
            <span
              aria-hidden="true"
              className="bg-foreground/85 dark:bg-background absolute top-2.5 left-1/2 size-3 -translate-x-1/2 rounded-full dark:ring-1 dark:ring-border"
            />
            <span className="flex shrink-0 items-center gap-1 [&_svg]:size-3.5">{status}</span>
          </div>

          <div className="relative flex min-h-0 flex-1 flex-col">{children}</div>

          <div
            aria-hidden="true"
            className="relative flex h-5 shrink-0 items-center justify-center"
          >
            <span className="bg-foreground/40 h-1 w-24 rounded-full" />
          </div>
        </div>
      </div>
    </div>
  );
}
