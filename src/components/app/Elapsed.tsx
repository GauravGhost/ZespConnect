import { useNow } from "@/hooks/useNow";
import { formatElapsed } from "@/lib/format";
import { cn } from "@/lib/utils";

const TICK_MS = 1000;

/** A running "m:ss" since `since`, ticking once a second. */
export function Elapsed({ since, className }: Readonly<{ since: number; className?: string }>) {
  const now = useNow(TICK_MS);
  return (
    <span className={cn("tabular-nums", className)}>{formatElapsed(Math.max(0, now - since))}</span>
  );
}
