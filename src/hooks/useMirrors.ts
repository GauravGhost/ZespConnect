import { useMutation, useQuery, useQueryClient, type QueryClient } from "@tanstack/react-query";
import { useEffect, useMemo } from "react";
import { toast } from "sonner";

import { events } from "@/lib/events";
import { errorMessage, ipc } from "@/lib/ipc";
import { DEFAULT_MIRROR_OPTIONS } from "@/lib/mirror";
import { supportsInAppMirror } from "@/lib/webcodecs";
import type { MirrorKind, MirrorOptions } from "@/types";

export const mirrorsKey = ["mirrors"] as const;
export const sessionsKey = ["mirror-sessions"] as const;

function invalidateWindows(queryClient: QueryClient) {
  queryClient.invalidateQueries({ queryKey: mirrorsKey });
  queryClient.invalidateQueries({ queryKey: sessionsKey });
}

/** One open mirror, whether an in-app session or an external scrcpy window. */
export interface OpenWindow {
  key: string;
  id: number;
  session: boolean;
  serial: string;
  kind: MirrorKind;
  package: string | null;
  label: string | null;
  started: number;
}

/** Every open mirror, kept in sync with the backend's exit events. */
export function useOpenWindows() {
  const queryClient = useQueryClient();

  useEffect(() => {
    const unlisten: (() => void)[] = [];
    let disposed = false;

    const subscribe = (pending: Promise<() => void>) => {
      pending
        .then((fn) => {
          if (disposed) fn();
          else unlisten.push(fn);
        })
        .catch(() => {
          /* backend not ready yet */
        });
    };

    subscribe(events.mirrorExited(() => invalidateWindows(queryClient)));
    subscribe(events.mirrorSessionExited(() => invalidateWindows(queryClient)));

    return () => {
      disposed = true;
      unlisten.forEach((fn) => fn());
    };
  }, [queryClient]);

  const mirrors = useQuery({ queryKey: mirrorsKey, queryFn: () => ipc.mirror.list() });
  const sessions = useQuery({ queryKey: sessionsKey, queryFn: () => ipc.mirror.listSessions() });

  const windows = useMemo<OpenWindow[]>(() => {
    const open: OpenWindow[] = (sessions.data ?? []).map((session) => ({
      key: `session-${session.id}`,
      id: session.id,
      session: true,
      serial: session.serial,
      kind: session.kind,
      package: session.package,
      label: session.label,
      started: session.started,
    }));

    for (const mirror of mirrors.data ?? []) {
      open.push({
        key: `pid-${mirror.pid}`,
        id: mirror.pid,
        session: false,
        serial: mirror.serial,
        kind: mirror.kind,
        package: mirror.package,
        label: mirror.label,
        started: mirror.started,
      });
    }

    return open;
  }, [mirrors.data, sessions.data]);

  return { windows, isLoading: mirrors.isLoading || sessions.isLoading };
}

/** Render in-app when this webview can decode H.264, else open an external window. */
function openMirror(
  serial: string,
  kind: MirrorKind,
  pkg: string | null,
  label: string,
  options: MirrorOptions,
  fallback: () => Promise<unknown>,
): Promise<unknown> {
  if (supportsInAppMirror()) {
    return ipc.mirror.openWindow({ serial, kind, package: pkg, label, options });
  }
  return fallback();
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
    mutationFn: ({ serial, pkg, label }: OpenAppVars) =>
      openMirror(serial, "app", pkg, label, DEFAULT_MIRROR_OPTIONS, () =>
        ipc.mirror.startAppWindow(serial, pkg, label),
      ),
    onSuccess: () => invalidateWindows(queryClient),
    onError: (error) => toast.error(errorMessage(error)),
  });
}

/** Mirror the main display with the app in the foreground. */
export function useOpenAppOnScreen() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ serial, pkg, label }: OpenAppVars) =>
      openMirror(serial, "screen-app", pkg, label, DEFAULT_MIRROR_OPTIONS, () =>
        ipc.mirror.startAppOnScreen(serial, pkg, label),
      ),
    onSuccess: () => invalidateWindows(queryClient),
    onError: (error) => toast.error(errorMessage(error)),
  });
}

/** Mirror the full device screen. */
export function useStartMirror() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ serial, options }: { serial: string; options: MirrorOptions }) =>
      openMirror(serial, "mirror", null, "", options, () => ipc.mirror.start(serial, options)),
    onSuccess: () => invalidateWindows(queryClient),
    onError: (error) => toast.error(errorMessage(error)),
  });
}

/** Stream the device camera. */
export function useStartCamera() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ serial, options }: { serial: string; options: MirrorOptions }) =>
      openMirror(serial, "camera", null, "", options, () =>
        ipc.mirror.startCamera(serial, options),
      ),
    onSuccess: () => invalidateWindows(queryClient),
    onError: (error) => toast.error(errorMessage(error)),
  });
}

/** Close one open mirror. */
export function useStopOpenWindow() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (target: OpenWindow) =>
      target.session ? ipc.mirror.stopSession(target.id) : ipc.mirror.stop(target.id),
    onSuccess: () => invalidateWindows(queryClient),
    onError: (error) => toast.error(errorMessage(error)),
  });
}

/** Close every open mirror. */
export function useStopAllMirrors() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: () => ipc.mirror.stopAll(),
    onSuccess: () => invalidateWindows(queryClient),
    onError: (error) => toast.error(errorMessage(error)),
  });
}
