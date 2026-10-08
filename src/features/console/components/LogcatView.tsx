import { Loader2, Play, Square, Trash2 } from "lucide-react";
import { useEffect, useRef, useState } from "react";

import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { cn } from "@/lib/utils";
import type { LogLine } from "@/types";
import { useLogcat } from "../hooks";

const LEVEL_COLORS: Record<string, string> = {
  V: "text-neutral-400",
  D: "text-sky-400",
  I: "text-emerald-400",
  W: "text-amber-400",
  E: "text-rose-400",
  F: "text-rose-500",
};

/** The single-letter severity in a `threadtime` logcat line, if present. */
function levelOf(line: string): string | null {
  const parts = line.split(/\s+/);
  const candidate = parts[4];
  if (parts.length >= 5 && candidate.length === 1 && "VDIWEF".includes(candidate)) {
    return candidate;
  }
  return null;
}

/** One logcat line, tinted by severity (stderr always reads as an error). */
function LogRow({ line }: { line: LogLine }) {
  const level = line.stream === "stderr" ? null : levelOf(line.text);
  const tone =
    line.stream === "stderr" ? "text-rose-400" : level ? LEVEL_COLORS[level] : "text-neutral-200";
  return <div className={cn("whitespace-pre-wrap", tone)}>{line.text}</div>;
}

export function LogcatView({ serial }: { serial: string }) {
  const { lines, streaming, error, start, stop, clear } = useLogcat(serial);
  const [autoscroll, setAutoscroll] = useState(true);
  const endRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (autoscroll) endRef.current?.scrollIntoView({ block: "end" });
  }, [lines, autoscroll]);

  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="flex shrink-0 items-center justify-between gap-2 border-b px-4 py-2">
        <div className="flex items-center gap-2">
          {streaming ? (
            <Button variant="outline" size="sm" onClick={stop}>
              <Square className="size-3.5" />
              Stop
            </Button>
          ) : (
            <Button variant="outline" size="sm" onClick={start}>
              <Play className="size-3.5" />
              Start
            </Button>
          )}
          <Button variant="ghost" size="sm" onClick={clear} disabled={lines.length === 0}>
            <Trash2 className="size-3.5" />
            Clear
          </Button>
          {streaming ? (
            <span className="text-success inline-flex items-center gap-1.5 text-xs">
              <Loader2 className="size-3.5 animate-spin" />
              Streaming
            </span>
          ) : null}
        </div>
        <Label className="flex items-center gap-2 text-xs font-normal">
          <Switch checked={autoscroll} onCheckedChange={setAutoscroll} />
          Auto-scroll
        </Label>
      </div>

      <div className="min-h-0 flex-1 overflow-auto bg-neutral-950 p-3 font-mono text-[11px] leading-relaxed text-neutral-200">
        {error ? (
          <p className="p-2 text-rose-400">{error}</p>
        ) : lines.length === 0 ? (
          <p className="p-2 text-neutral-400">
            {streaming ? "Waiting for log output…" : "Press Start to stream the device log."}
          </p>
        ) : (
          <>
            {lines.map((line, index) => (
              <LogRow key={index} line={line} />
            ))}
            <div ref={endRef} />
          </>
        )}
      </div>
    </div>
  );
}
