import { listen, type EventCallback, type UnlistenFn } from "@tauri-apps/api/event";

import type { Device, MirrorProcess } from "@/types";

/** Typed map of the backend events emitted via `AppHandle::emit`. */
export interface BackendEvents {
  "devices:changed": Device[];
  "mirror:exited": MirrorProcess;
  "install:progress": { serial: string; package: string; progress: number; message: string };
}

export function onBackendEvent<K extends keyof BackendEvents>(
  event: K,
  handler: EventCallback<BackendEvents[K]>,
): Promise<UnlistenFn> {
  return listen<BackendEvents[K]>(event, handler);
}

export const events = {
  devicesChanged: (handler: EventCallback<Device[]>) => onBackendEvent("devices:changed", handler),
  mirrorExited: (handler: EventCallback<MirrorProcess>) => onBackendEvent("mirror:exited", handler),
  installProgress: (handler: EventCallback<BackendEvents["install:progress"]>) =>
    onBackendEvent("install:progress", handler),
};
