import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect } from "react";
import { toast } from "sonner";

import { events } from "@/lib/events";
import { errorMessage, ipc } from "@/lib/ipc";
import type { MirrorOptions } from "@/types";

export const mirrorsKey = ["mirrors"] as const;

/** Open scrcpy windows, kept in sync with the backend's `mirror:exited` event. */
export function useMirrors() {
  const queryClient = useQueryClient();

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let disposed = false;

    events
      .mirrorExited(() => {
        queryClient.invalidateQueries({ queryKey: mirrorsKey });
      })
      .then((fn) => {
        if (disposed) fn();
        else unlisten = fn;
      })
      .catch(() => {
        /* backend not ready yet */
      });

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [queryClient]);

  return useQuery({
    queryKey: mirrorsKey,
    queryFn: () => ipc.mirror.list(),
  });
}

interface OpenAppVars {
  serial: string;
  pkg: string;
  label: string;
}

/** Open an app in its own window on the laptop. */
export function useOpenAppWindow() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (vars: OpenAppVars) => ipc.mirror.startAppWindow(vars.serial, vars.pkg, vars.label),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: mirrorsKey }),
    onError: (error) => toast.error(errorMessage(error)),
  });
}

/** Mirror the main display with the app in the foreground. */
export function useOpenAppOnScreen() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (vars: OpenAppVars) =>
      ipc.mirror.startAppOnScreen(vars.serial, vars.pkg, vars.label),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: mirrorsKey }),
    onError: (error) => toast.error(errorMessage(error)),
  });
}

export function useStopMirror() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (pid: number) => ipc.mirror.stop(pid),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: mirrorsKey }),
    onError: (error) => toast.error(errorMessage(error)),
  });
}

/** Mirror the full device screen. */
export function useStartMirror() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ serial, options }: { serial: string; options: MirrorOptions }) =>
      ipc.mirror.start(serial, options),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: mirrorsKey }),
    onError: (error) => toast.error(errorMessage(error)),
  });
}

/** Stream the device camera. */
export function useStartCamera() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ serial, options }: { serial: string; options: MirrorOptions }) =>
      ipc.mirror.startCamera(serial, options),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: mirrorsKey }),
    onError: (error) => toast.error(errorMessage(error)),
  });
}

/** Close every open scrcpy window. */
export function useStopAllMirrors() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: () => ipc.mirror.stopAll(),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: mirrorsKey }),
    onError: (error) => toast.error(errorMessage(error)),
  });
}
