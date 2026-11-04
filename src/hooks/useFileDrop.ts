import { useQueryClient } from "@tanstack/react-query";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { useCallback, useEffect, useRef, useState } from "react";

import { dropSummary, fileName, isApk, type DropResult } from "@/lib/fileDrop";
import { errorMessage, ipc } from "@/lib/ipc";

import { useBackendEvents } from "./useBackendEvents";
import { appsKey } from "./useInstallApk";

const DONE_VISIBLE_MS = 4000;
const FAILED_VISIBLE_MS = 8000;

export type DropStatus =
  | { kind: "sending"; name: string; installing: boolean; index: number; total: number }
  | { kind: "done"; text: string; failed: boolean };

interface QueuedFile {
  serial: string;
  path: string;
}

/**
 * Files dropped on this window go to `serial`: APKs install, anything else is copied to the
 * phone's drop folder. Drops made while a batch is running join that batch.
 */
export function useFileDrop(serial: string | null) {
  const queryClient = useQueryClient();
  const [hovering, setHovering] = useState(false);
  const [status, setStatus] = useState<DropStatus | null>(null);
  const serialRef = useRef(serial);
  const queueRef = useRef<QueuedFile[]>([]);
  const busyRef = useRef(false);

  useEffect(() => {
    serialRef.current = serial;
  }, [serial]);

  const drain = useCallback(async () => {
    busyRef.current = true;
    const results: DropResult[] = [];
    const installedOn = new Set<string>();

    for (let next = queueRef.current.shift(); next; next = queueRef.current.shift()) {
      const name = fileName(next.path);
      const index = results.length + 1;
      setStatus({
        kind: "sending",
        name,
        installing: isApk(next.path),
        index,
        total: index + queueRef.current.length,
      });
      try {
        const outcome = await ipc.files.sendToPhone(next.serial, next.path);
        if (outcome === "installed") installedOn.add(next.serial);
        results.push({ name, outcome });
      } catch (error) {
        results.push({ name, error: errorMessage(error) });
      }
    }

    busyRef.current = false;
    installedOn.forEach((target) => queryClient.invalidateQueries({ queryKey: appsKey(target) }));
    setStatus({ kind: "done", ...dropSummary(results) });
  }, [queryClient]);

  useBackendEvents(
    (track) => {
      track(
        getCurrentWebview().onDragDropEvent(({ payload }) => {
          if (payload.type === "enter") setHovering(true);
          if (payload.type === "leave") setHovering(false);
          if (payload.type !== "drop") return;
          setHovering(false);
          const target = serialRef.current;
          if (!target || payload.paths.length === 0) return;
          queueRef.current.push(...payload.paths.map((path) => ({ serial: target, path })));
          if (!busyRef.current) void drain();
        }),
      );
    },
    [drain],
  );

  useEffect(() => {
    if (status?.kind !== "done") return;
    const timer = setTimeout(
      () => setStatus(null),
      status.failed ? FAILED_VISIBLE_MS : DONE_VISIBLE_MS,
    );
    return () => clearTimeout(timer);
  }, [status]);

  return { hovering, status };
}
