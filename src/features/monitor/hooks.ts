import { Channel } from "@tauri-apps/api/core";
import { useMutation } from "@tanstack/react-query";
import { useCallback, useEffect, useRef, useState } from "react";
import { toast } from "sonner";

import { errorMessage, ipc } from "@/lib/ipc";
import type { PerfSample, ProcessInfo } from "@/types";
import { appendSample, type TimedSample } from "./history";

interface StreamApi<T> {
  open: (serial: string, channel: Channel<T>) => Promise<void>;
  close: (serial: string) => Promise<void>;
}

/**
 * Keep a phone stream open while `serial` is set, `paused` is false and the window is visible,
 * so a page left open in the tray costs the phone nothing.
 */
function useDeviceStream<T>(
  api: StreamApi<T>,
  serial: string | null,
  paused: boolean,
  onValue: (value: T) => void,
) {
  const [error, setError] = useState<string | null>(null);
  const [visible, setVisible] = useState(() => document.visibilityState === "visible");
  const onValueRef = useRef(onValue);
  // Open and close must reach the backend in order, or a late close could end the new stream.
  const queue = useRef<Promise<unknown>>(Promise.resolve());

  useEffect(() => {
    onValueRef.current = onValue;
  }, [onValue]);

  useEffect(() => {
    const update = () => setVisible(document.visibilityState === "visible");
    document.addEventListener("visibilitychange", update);
    return () => document.removeEventListener("visibilitychange", update);
  }, []);

  useEffect(() => {
    setError(null);
    if (!serial || paused || !visible) return;
    let active = true;
    const channel = new Channel<T>();
    channel.onmessage = (value) => {
      if (active) onValueRef.current(value);
    };
    queue.current = queue.current
      .then(() => api.open(serial, channel))
      .catch((failure: unknown) => {
        if (active) setError(errorMessage(failure));
      });
    return () => {
      active = false;
      // Closing a stream that already ended is not worth reporting.
      queue.current = queue.current.then(() => api.close(serial)).catch(() => undefined);
    };
  }, [api, serial, paused, visible]);

  return { error, live: Boolean(serial) && !paused && visible && error === null };
}

const PERFORMANCE_API: StreamApi<PerfSample> = {
  open: ipc.monitor.openPerformance,
  close: ipc.monitor.closePerformance,
};

const PROCESSES_API: StreamApi<ProcessInfo[]> = {
  open: ipc.monitor.openProcesses,
  close: ipc.monitor.closeProcesses,
};

export function usePerformance(serial: string | null) {
  const [history, setHistory] = useState<TimedSample[]>([]);
  const [paused, setPaused] = useState(false);

  useEffect(() => {
    setHistory([]);
  }, [serial]);

  const onSample = useCallback((sample: PerfSample) => {
    setHistory((previous) => appendSample(previous, { ...sample, at: Date.now() }));
  }, []);

  const stream = useDeviceStream(PERFORMANCE_API, serial, paused, onSample);
  return { history, paused, setPaused, ...stream };
}

export function useProcesses(serial: string | null) {
  const [processes, setProcesses] = useState<ProcessInfo[] | null>(null);
  const [paused, setPaused] = useState(false);

  useEffect(() => {
    setProcesses(null);
  }, [serial]);

  const stream = useDeviceStream(PROCESSES_API, serial, paused, setProcesses);
  return { processes, paused, setPaused, ...stream };
}

export function useForceStop(serial: string | null) {
  return useMutation({
    mutationFn: (pkg: string) => ipc.apps.forceStop(serial as string, pkg),
    onError: (error) => toast.error(errorMessage(error)),
  });
}
