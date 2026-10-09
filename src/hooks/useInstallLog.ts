import { useCallback, useState } from "react";

import { events } from "@/lib/events";
import type { ToolKind } from "@/types";

import { useBackendEvents } from "./useBackendEvents";

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

/** Live tool-installer log: streams `tools:install:progress` and settles on `tools:install:done`. */
export function useInstallLog() {
  const [state, setState] = useState<InstallLogState>(EMPTY);

  useBackendEvents((track) => {
    track(
      events.toolsInstallProgress((event) => {
        setState((prev) => ({
          ...prev,
          running: true,
          tool: event.payload.tool,
          lines: [...prev.lines.slice(-MAX_LINES), event.payload.line],
          message: null,
          ok: null,
        }));
      }),
    );
    track(
      events.toolsInstallDone((event) => {
        setState((prev) => ({
          ...prev,
          running: false,
          tool: event.payload.tool,
          message: event.payload.message,
          ok: event.payload.ok,
        }));
      }),
    );
  }, []);

  const reset = useCallback(() => setState(EMPTY), []);

  return { ...state, reset };
}
