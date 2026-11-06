import { Channel } from "@tauri-apps/api/core";
import { useMutation } from "@tanstack/react-query";
import { useCallback, useEffect, useRef, useState } from "react";
import { toast } from "sonner";

import { errorMessage, ipc } from "@/lib/ipc";
import type { LogLine, LogcatFilter } from "@/types";
import { parseLogLine, type ParsedLine } from "./logcat";

const MAX_LINES = 2000;
/** Coalesce logcat lines so a chatty device does not re-render per line. */
const FLUSH_MS = 300;

export function useShellExec(serial: string | null) {
  return useMutation({
    mutationFn: (command: string) => ipc.console.exec(serial as string, command),
    onError: (error) => toast.error(errorMessage(error)),
  });
}

export interface LogcatController {
  lines: ParsedLine[];
  streaming: boolean;
  paused: boolean;
  error: string | null;
  filter: LogcatFilter;
  start: () => void;
  stop: () => void;
  clear: () => void;
  setPaused: (paused: boolean) => void;
  /** Restarts a running stream so the device applies the new filter. */
  setFilter: (filter: LogcatFilter) => void;
}

const NO_FILTER: LogcatFilter = { package: null, minLevel: null };

function capped(lines: ParsedLine[]): ParsedLine[] {
  return lines.length > MAX_LINES ? lines.slice(lines.length - MAX_LINES) : lines;
}

export function useLogcat(serial: string | null): LogcatController {
  const [lines, setLines] = useState<ParsedLine[]>([]);
  const [streaming, setStreaming] = useState(false);
  const [paused, setPaused] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [filter, setFilterState] = useState<LogcatFilter>(NO_FILTER);
  const buffer = useRef<ParsedLine[]>([]);
  // Lines from a stream that was replaced or stopped are dropped, not shown.
  const generation = useRef(0);

  useEffect(() => {
    if (paused) return;
    const id = window.setInterval(() => {
      if (buffer.current.length === 0) return;
      const incoming = buffer.current;
      buffer.current = [];
      setLines((prev) => capped(prev.concat(incoming)));
    }, FLUSH_MS);
    return () => window.clearInterval(id);
  }, [paused]);

  // Reset and stop the stream whenever the target device changes or goes away.
  useEffect(() => {
    generation.current += 1;
    setStreaming(false);
    setPaused(false);
    setError(null);
    setLines([]);
    setFilterState(NO_FILTER);
    buffer.current = [];
    return () => {
      // Closing a stream that already ended (or never opened) is not worth reporting.
      if (serial) void ipc.console.closeLogcat(serial).catch(() => undefined);
    };
  }, [serial]);

  const open = useCallback(
    (target: LogcatFilter) => {
      if (!serial) return;
      const current = ++generation.current;
      setError(null);
      setLines([]);
      buffer.current = [];
      const channel = new Channel<LogLine>();
      channel.onmessage = (line) => {
        if (generation.current !== current) return;
        buffer.current.push(parseLogLine(line));
        // A paused view keeps only the newest lines it could show.
        if (buffer.current.length > MAX_LINES * 2) buffer.current = capped(buffer.current);
      };
      setStreaming(true);
      ipc.console.openLogcat(serial, target, channel).catch((failure) => {
        if (generation.current !== current) return;
        setStreaming(false);
        setError(errorMessage(failure));
      });
    },
    [serial],
  );

  const start = useCallback(() => open(filter), [open, filter]);

  const stop = useCallback(() => {
    generation.current += 1;
    if (serial) void ipc.console.closeLogcat(serial).catch(() => undefined);
    setStreaming(false);
  }, [serial]);

  const setFilter = useCallback(
    (next: LogcatFilter) => {
      setFilterState(next);
      if (streaming) open(next);
    },
    [open, streaming],
  );

  const clear = useCallback(() => {
    buffer.current = [];
    setLines([]);
  }, []);

  return {
    lines,
    streaming,
    paused,
    error,
    filter,
    start,
    stop,
    clear,
    setPaused,
    setFilter,
  };
}
