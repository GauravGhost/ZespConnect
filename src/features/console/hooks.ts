import { Channel } from "@tauri-apps/api/core";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useCallback, useEffect, useRef, useState } from "react";
import { toast } from "sonner";

import { errorMessage, ipc } from "@/lib/ipc";
import type { LogLine, LogcatFilter, PortDirection, PortRule, ShellEvent } from "@/types";
import { parseLogLine, type ParsedLine } from "./logcat";
import { capOutput, cleanTerminalText } from "./shell";

const MAX_LINES = 2000;
/** Coalesce logcat lines so a chatty device does not re-render per line. */
const FLUSH_MS = 300;
/** Shorter than logcat so typed commands echo back without a visible lag. */
const SHELL_FLUSH_MS = 40;

export interface ShellTab {
  key: number;
  title: string;
  output: string;
  status: "starting" | "running" | "ended";
  /** Exit code once ended; null when the shell was killed or never started. */
  exitCode: number | null;
  error: string | null;
}

export interface ShellSessions {
  tabs: ShellTab[];
  activeKey: number | null;
  setActiveKey: (key: number) => void;
  openTab: () => void;
  closeTab: (key: number) => void;
  /** Start a fresh shell in an ended tab, keeping its transcript. */
  restart: (key: number) => void;
  /** Raw input for the tab's shell: a command line ends with "\n", Ctrl+C is "\x03". */
  send: (key: number, input: string) => void;
  clear: (key: number) => void;
}

/** One long-lived `adb shell` per tab, so cd, exports and su carry over between commands. All
 * shells close when the device changes or the view unmounts. */
export function useShellSessions(serial: string): ShellSessions {
  const [tabs, setTabs] = useState<ShellTab[]>([]);
  const [activeKey, setActiveKey] = useState<number | null>(null);
  // Tab key -> its session id (null when the shell failed to start).
  const sessions = useRef(new Map<number, Promise<number | null>>());
  const nextKey = useRef(1);
  const titleCount = useRef(0);
  const pending = useRef(new Map<number, string>());
  const flushTimer = useRef<number | null>(null);

  const updateTab = useCallback((key: number, change: Partial<ShellTab>) => {
    setTabs((prev) => prev.map((tab) => (tab.key === key ? { ...tab, ...change } : tab)));
  }, []);

  const flush = useCallback(() => {
    if (flushTimer.current !== null) window.clearTimeout(flushTimer.current);
    flushTimer.current = null;
    if (pending.current.size === 0) return;
    const chunks = pending.current;
    pending.current = new Map();
    setTabs((prev) =>
      prev.map((tab) => {
        const text = chunks.get(tab.key);
        return text ? { ...tab, output: capOutput(tab.output + text) } : tab;
      }),
    );
  }, []);

  const startSession = useCallback(
    (key: number) => {
      const session = new Promise<number | null>((resolve) => {
        const channel = new Channel<ShellEvent>();
        channel.onmessage = (event) => {
          if (sessions.current.get(key) !== session) return;
          if (event.kind === "output") {
            const text = cleanTerminalText(event.text);
            pending.current.set(key, (pending.current.get(key) ?? "") + text);
            flushTimer.current ??= window.setTimeout(flush, SHELL_FLUSH_MS);
          } else {
            flush();
            updateTab(key, { status: "ended", exitCode: event.code });
          }
        };
        ipc.console.openShell(serial, channel).then(
          (id) => {
            if (sessions.current.get(key) === session) updateTab(key, { status: "running" });
            resolve(id);
          },
          (failure) => {
            updateTab(key, { status: "ended", exitCode: null, error: errorMessage(failure) });
            resolve(null);
          },
        );
      });
      sessions.current.set(key, session);
      updateTab(key, { status: "starting", exitCode: null, error: null });
    },
    [serial, flush, updateTab],
  );

  const stopSession = useCallback((key: number) => {
    const session = sessions.current.get(key);
    sessions.current.delete(key);
    pending.current.delete(key);
    void session?.then((id) => {
      if (id !== null) void ipc.console.closeShell(id).catch(() => undefined);
    });
  }, []);

  const openTab = useCallback(() => {
    const key = nextKey.current++;
    titleCount.current += 1;
    const tab: ShellTab = {
      key,
      title: `Shell ${titleCount.current}`,
      output: "",
      status: "starting",
      exitCode: null,
      error: null,
    };
    setTabs((prev) => [...prev, tab]);
    setActiveKey(key);
    startSession(key);
  }, [startSession]);

  useEffect(() => {
    const open = sessions.current;
    openTab();
    return () => {
      for (const key of [...open.keys()]) stopSession(key);
      titleCount.current = 0;
      setTabs([]);
      setActiveKey(null);
    };
  }, [openTab, stopSession]);

  const closeTab = useCallback(
    (key: number) => {
      stopSession(key);
      const index = tabs.findIndex((tab) => tab.key === key);
      const rest = tabs.filter((tab) => tab.key !== key);
      setTabs(rest);
      if (activeKey === key) setActiveKey(rest[Math.min(index, rest.length - 1)]?.key ?? null);
    },
    [tabs, activeKey, stopSession],
  );

  const restart = useCallback(
    (key: number) => {
      stopSession(key);
      startSession(key);
    },
    [startSession, stopSession],
  );

  const send = useCallback((key: number, input: string) => {
    void sessions.current.get(key)?.then((id) => {
      if (id === null) return;
      ipc.console.writeShell(id, input).catch((failure) => toast.error(errorMessage(failure)));
    });
  }, []);

  const clear = useCallback(
    (key: number) => {
      pending.current.delete(key);
      updateTab(key, { output: "" });
    },
    [updateTab],
  );

  return { tabs, activeKey, setActiveKey, openTab, closeTab, restart, send, clear };
}

const portRulesKey = (serial: string) => ["port-rules", serial] as const;

/** Read on open and after each change; rules made outside the app show up on the next open. */
export function usePortRules(serial: string) {
  return useQuery({
    queryKey: portRulesKey(serial),
    queryFn: () => ipc.ports.list(serial),
    staleTime: 0,
  });
}

export type PortChange =
  | { kind: "add"; direction: PortDirection; hostPort: number; devicePort: number }
  | { kind: "remove"; rule: PortRule }
  | { kind: "clear" };

export function useChangePortRules(serial: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (change: PortChange) => {
      switch (change.kind) {
        case "add":
          return ipc.ports.add(serial, change.direction, change.hostPort, change.devicePort);
        case "remove":
          return ipc.ports.remove(serial, change.rule);
        case "clear":
          return ipc.ports.clear(serial);
      }
    },
    onSuccess: (rules) => queryClient.setQueryData(portRulesKey(serial), rules),
    onError: (error) => {
      toast.error(errorMessage(error));
      void queryClient.invalidateQueries({ queryKey: portRulesKey(serial) });
    },
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
