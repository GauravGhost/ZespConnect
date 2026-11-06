import { useMutation, useMutationState, useQuery, useQueryClient } from "@tanstack/react-query";
import { useCallback, useEffect, useRef, useState } from "react";
import { toast } from "sonner";

import { errorMessage, ipc } from "@/lib/ipc";
import type { DeveloperState, DeveloperToggleKey } from "@/types";
import { parseLayout, type LayoutNode } from "./layout";

const developerStateKey = (serial: string) => ["developer-state", serial] as const;

/** Read when the tab opens and after each change; never polled. */
export function useDeveloperState(serial: string) {
  return useQuery({
    queryKey: developerStateKey(serial),
    queryFn: () => ipc.controls.developerState(serial),
    staleTime: 0,
  });
}

function useDeveloperMutation<T>(
  serial: string,
  run: (vars: T) => Promise<void>,
  patch: (vars: T) => Partial<DeveloperState>,
) {
  const queryClient = useQueryClient();
  const key = developerStateKey(serial);
  return useMutation({
    mutationFn: run,
    onMutate: async (vars: T) => {
      await queryClient.cancelQueries({ queryKey: key });
      const previous = queryClient.getQueryData<DeveloperState>(key);
      if (previous) queryClient.setQueryData<DeveloperState>(key, { ...previous, ...patch(vars) });
      return { previous };
    },
    onError: (error, _vars, context) => {
      if (context?.previous) queryClient.setQueryData(key, context.previous);
      toast.error(errorMessage(error));
    },
    onSettled: () => queryClient.invalidateQueries({ queryKey: key }),
  });
}

export function useSetDeveloperToggle(serial: string) {
  return useDeveloperMutation(
    serial,
    ({ key, on }: { key: DeveloperToggleKey; on: boolean }) =>
      ipc.controls.setDeveloperToggle(serial, key, on),
    ({ key, on }) => ({ [key]: on }),
  );
}

export function useSetAnimationScale(serial: string) {
  return useDeveloperMutation(
    serial,
    (scale: number) => ipc.controls.setAnimationScale(serial, scale),
    (scale) => ({ animation_scale: scale }),
  );
}

const crashesKey = (serial: string) => ["crashes", serial] as const;
const bugReportKey = (serial: string) => ["bug-report", serial] as const;

/** Read when the tab opens and on Refresh; never polled. */
export function useCrashReports(serial: string) {
  return useQuery({
    queryKey: crashesKey(serial),
    queryFn: () => ipc.crashes.list(serial),
    staleTime: 0,
  });
}

/** Toasts are set here, not per call, so they still show if the tab closes during the capture. */
export function useSaveBugReport(serial: string) {
  return useMutation({
    mutationKey: bugReportKey(serial),
    mutationFn: (path: string) => ipc.crashes.saveBugReport(serial, path),
    onSuccess: () => toast.success("Bug report saved"),
    onError: (error) => toast.error(errorMessage(error)),
  });
}

/** When this device's running bug report started, or null. Read from the mutation cache so a
 * capture started before the tab was reopened still shows. */
export function useBugReportStart(serial: string): number | null {
  const started = useMutationState({
    filters: { mutationKey: bugReportKey(serial), status: "pending" },
    select: (mutation) => mutation.state.submittedAt,
  });
  return started[0] ?? null;
}

export interface LayoutSnapshot {
  xml: string;
  nodes: LayoutNode[];
  /** Object URL of the screenshot, revoked when the snapshot is replaced. */
  imageUrl: string;
  /** Screenshot size in device pixels, the space node bounds are in. */
  width: number;
  height: number;
}

export interface LayoutCapture {
  snapshot: LayoutSnapshot | null;
  capturing: boolean;
  error: string | null;
  capture: () => void;
}

/** Captured on demand only: a dump takes seconds and briefly loads the phone. */
export function useLayoutCapture(serial: string): LayoutCapture {
  const [snapshot, setSnapshot] = useState<LayoutSnapshot | null>(null);
  const [capturing, setCapturing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  // A capture that finishes after the device changed or a newer one started is dropped.
  const generation = useRef(0);

  useEffect(() => {
    generation.current += 1;
    setSnapshot(null);
    setCapturing(false);
    setError(null);
  }, [serial]);

  useEffect(() => {
    const url = snapshot?.imageUrl;
    return () => {
      if (url) URL.revokeObjectURL(url);
    };
  }, [snapshot]);

  const capture = useCallback(() => {
    const current = ++generation.current;
    setCapturing(true);
    setError(null);
    Promise.all([ipc.layout.dump(serial), ipc.layout.screenshot(serial)])
      .then(async ([xml, png]) => {
        const nodes = parseLayout(xml);
        const image = new Blob([png], { type: "image/png" });
        const bitmap = await createImageBitmap(image);
        const { width, height } = bitmap;
        bitmap.close();
        if (generation.current !== current) return;
        setSnapshot({ xml, nodes, imageUrl: URL.createObjectURL(image), width, height });
      })
      .catch((failure) => {
        if (generation.current === current) setError(errorMessage(failure));
      })
      .finally(() => {
        if (generation.current === current) setCapturing(false);
      });
  }, [serial]);

  return { snapshot, capturing, error, capture };
}
