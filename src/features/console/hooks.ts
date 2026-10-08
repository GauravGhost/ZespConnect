import { Channel } from "@tauri-apps/api/core";
import { useMutation } from "@tanstack/react-query";
import { useCallback, useEffect, useRef, useState } from "react";
import { toast } from "sonner";

import { errorMessage, ipc } from "@/lib/ipc";
import type { LogLine } from "@/types";

const MAX_LINES = 2000;
/** Coalesce logcat lines so a chatty device does not re-render per line. */
const FLUSH_MS = 300;

/** Run one `adb shell` command on the active device. */
export function useShellExec(serial: string | null) {
  return useMutation({
    mutationFn: (command: string) => ipc.console.exec(serial as string, command),
    onError: (error) => toast.error(errorMessage(error)),
  });
}

export function useClipboardSend(serial: string | null) {
  return useMutation({
    mutationFn: (text: string) => ipc.clipboard.send(serial as string, text),
    onSuccess: () => toast.success("Sent to the device clipboard"),
    onError: (error) => toast.error(errorMessage(error)),
  });
}

export function useClipboardRead(serial: string | null) {
  return useMutation({
    mutationFn: () => ipc.clipboard.read(serial as string),
    onSuccess: () => toast.success("Device clipboard copied to your computer"),
    onError: (error) => toast.error(errorMessage(error)),
  });
}

export interface LogcatController {
  lines: LogLine[];
  streaming: boolean;
  error: string | null;
  start: () => void;
  stop: () => void;
  clear: () => void;
}

/** Stream `adb logcat` into a bounded, buffered line list. */
export function useLogcat(serial: string | null): LogcatController {
  const [lines, setLines] = useState<LogLine[]>([]);
  const [streaming, setStreaming] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const buffer = useRef<LogLine[]>([]);

  useEffect(() => {
    const id = window.setInterval(() => {
      if (buffer.current.length === 0) return;
      const incoming = buffer.current;
      buffer.current = [];
      setLines((prev) => {
        const next = prev.concat(incoming);
        return next.length > MAX_LINES ? next.slice(next.length - MAX_LINES) : next;
      });
    }, FLUSH_MS);
    return () => window.clearInterval(id);
  }, []);

  // Reset and stop the stream whenever the target device changes or goes away.
  useEffect(() => {
    setStreaming(false);
    setError(null);
    setLines([]);
    buffer.current = [];
    return () => {
      if (serial) void ipc.console.closeLogcat(serial).catch(() => undefined);
    };
  }, [serial]);

  const stop = useCallback(() => {
    if (serial) void ipc.console.closeLogcat(serial).catch(() => undefined);
    setStreaming(false);
  }, [serial]);

  const start = useCallback(() => {
    if (!serial) return;
    setError(null);
    const channel = new Channel<LogLine>();
    channel.onmessage = (line) => buffer.current.push(line);
    setStreaming(true);
    ipc.console.openLogcat(serial, channel).catch((failure) => {
      setStreaming(false);
      setError(errorMessage(failure));
    });
  }, [serial]);

  const clear = useCallback(() => {
    buffer.current = [];
    setLines([]);
  }, []);

  return { lines, streaming, error, start, stop, clear };
}
