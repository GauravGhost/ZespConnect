import { useCallback, useEffect, useState } from "react";

import { events } from "@/lib/events";
import type { ToolKind } from "@/types";

export interface InstallLogState {
  running: boolean;
  tool: ToolKind | null;
  lines: string[];
  message: string | null;
  ok: boolean | null;
}

const EMPTY: InstallLogState = {
  running: false,
  tool: null,
  lines: [],
  message: null,
  ok: null,
};

const MAX_LINES = 300;

/**
 * Live view of the tool installer: streams `tools:install:progress` lines and
 * settles on `tools:install:done`.
 */
export function useInstallLog() {
  const [state, setState] = useState<InstallLogState>(EMPTY);

  useEffect(() => {
    let unlistenProgress: (() => void) | undefined;
    let unlistenDone: (() => void) | undefined;
    let disposed = false;

    events
      .toolsInstallProgress((event) => {
        setState((prev) => ({
          ...prev,
          running: true,
          tool: event.payload.tool,
          lines: [...prev.lines.slice(-MAX_LINES), event.payload.line],
          message: null,
          ok: null,
        }));
      })
      .then((fn) => {
        if (disposed) fn();
        else unlistenProgress = fn;
      })
      .catch(() => {});

    events
      .toolsInstallDone((event) => {
        setState((prev) => ({
          ...prev,
          running: false,
          tool: event.payload.tool,
          message: event.payload.message,
          ok: event.payload.ok,
        }));
      })
      .then((fn) => {
        if (disposed) fn();
        else unlistenDone = fn;
      })
      .catch(() => {});

    return () => {
      disposed = true;
      unlistenProgress?.();
      unlistenDone?.();
    };
  }, []);

  const reset = useCallback(() => setState(EMPTY), []);

  return { ...state, reset };
}
