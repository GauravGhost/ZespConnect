import { listen, type EventCallback, type UnlistenFn } from "@tauri-apps/api/event";

import type { Device, InstallDone, InstallProgress, MirrorProcess, MirrorSession } from "@/types";

/** Typed map of the backend events emitted via `AppHandle::emit`. */
export interface BackendEvents {
  "devices:changed": Device[];
  "mirror:started": MirrorProcess;
  "mirror:exited": MirrorProcess;
  "mirror:session:started": MirrorSession;
  "mirror:session:exited": MirrorSession;
  /** A desktop session's phone was locked or unlocked (the desktop only shows unlocked). */
  "mirror:session:locked": { id: number; locked: boolean };
  "install:progress": { serial: string; package: string; progress: number; message: string };
  "tools:install:progress": InstallProgress;
  "tools:install:done": InstallDone;
  "tray:navigate": { tab: string };
  "tray:select-device": { serial: string };
}

export function onBackendEvent<K extends keyof BackendEvents>(
  event: K,
  handler: EventCallback<BackendEvents[K]>,
): Promise<UnlistenFn> {
  return listen<BackendEvents[K]>(event, handler);
}

export const events = {
  devicesChanged: (handler: EventCallback<Device[]>) => onBackendEvent("devices:changed", handler),
  mirrorStarted: (handler: EventCallback<MirrorProcess>) =>
    onBackendEvent("mirror:started", handler),
  mirrorExited: (handler: EventCallback<MirrorProcess>) => onBackendEvent("mirror:exited", handler),
  mirrorSessionStarted: (handler: EventCallback<MirrorSession>) =>
    onBackendEvent("mirror:session:started", handler),
  mirrorSessionExited: (handler: EventCallback<MirrorSession>) =>
    onBackendEvent("mirror:session:exited", handler),
  mirrorSessionLocked: (handler: EventCallback<BackendEvents["mirror:session:locked"]>) =>
    onBackendEvent("mirror:session:locked", handler),
  installProgress: (handler: EventCallback<BackendEvents["install:progress"]>) =>
    onBackendEvent("install:progress", handler),
  toolsInstallProgress: (handler: EventCallback<InstallProgress>) =>
    onBackendEvent("tools:install:progress", handler),
  toolsInstallDone: (handler: EventCallback<InstallDone>) =>
    onBackendEvent("tools:install:done", handler),
  trayNavigate: (handler: EventCallback<BackendEvents["tray:navigate"]>) =>
    onBackendEvent("tray:navigate", handler),
  traySelectDevice: (handler: EventCallback<BackendEvents["tray:select-device"]>) =>
    onBackendEvent("tray:select-device", handler),
};
