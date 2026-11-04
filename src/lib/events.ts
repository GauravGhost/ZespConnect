import { listen, type EventCallback, type UnlistenFn } from "@tauri-apps/api/event";

import type {
  AppInfo,
  Device,
  InstallDone,
  InstallProgress,
  MirrorProcess,
  MirrorSession,
  NowPlaying,
  ReconnectEvent,
  RecordingOutcome,
} from "@/types";

/** Typed map of the backend events emitted via `AppHandle::emit`. */
export interface BackendEvents {
  "devices:changed": Device[];
  "mirror:started": MirrorProcess;
  "mirror:exited": MirrorProcess;
  "mirror:session:started": MirrorSession;
  "mirror:session:exited": MirrorSession;
  /** A desktop session's phone was locked or unlocked (the desktop only shows unlocked). */
  "mirror:session:locked": { id: number; locked: boolean; phoneScreen: boolean };
  "phone-audio:changed": { serial: string; onComputer: boolean };
  "clipboard-sync:changed": { serial: string; on: boolean };
  "now-playing:changed": { serial: string; nowPlaying: NowPlaying | null };
  /** The launcher popup was shown again and should reset. */
  "launcher:shown": null;
  /** A phone's app list was re-read while the launcher showed the cached one. */
  "launcher:apps-changed": { serial: string; apps: AppInfo[] };
  "install:progress": { serial: string; package: string; progress: number; message: string };
  "tools:install:progress": InstallProgress;
  "tools:install:done": InstallDone;
  "tray:navigate": { tab: string };
  "wireless:reconnected": ReconnectEvent;
  /** A saved phone answered but no longer trusts this computer. */
  "wireless:unpaired": ReconnectEvent;
  "tray:select-device": { serial: string };
  "recording:changed": { id: number; recording: boolean; startedAt: number | null };
  /** A recording ended on its own (window closed, phone lost); sent to the window that shows it. */
  "recording:finished": {
    id: number;
    outcome: RecordingOutcome | null;
    error: string | null;
    /** The recording's window is still open and shows the result itself. */
    inWindow: boolean;
  };
}

function onBackendEvent<K extends keyof BackendEvents>(
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
  phoneAudioChanged: (handler: EventCallback<BackendEvents["phone-audio:changed"]>) =>
    onBackendEvent("phone-audio:changed", handler),
  clipboardSyncChanged: (handler: EventCallback<BackendEvents["clipboard-sync:changed"]>) =>
    onBackendEvent("clipboard-sync:changed", handler),
  nowPlayingChanged: (handler: EventCallback<BackendEvents["now-playing:changed"]>) =>
    onBackendEvent("now-playing:changed", handler),
  launcherShown: (handler: EventCallback<null>) => onBackendEvent("launcher:shown", handler),
  launcherAppsChanged: (handler: EventCallback<BackendEvents["launcher:apps-changed"]>) =>
    onBackendEvent("launcher:apps-changed", handler),
  installProgress: (handler: EventCallback<BackendEvents["install:progress"]>) =>
    onBackendEvent("install:progress", handler),
  toolsInstallProgress: (handler: EventCallback<InstallProgress>) =>
    onBackendEvent("tools:install:progress", handler),
  toolsInstallDone: (handler: EventCallback<InstallDone>) =>
    onBackendEvent("tools:install:done", handler),
  trayNavigate: (handler: EventCallback<BackendEvents["tray:navigate"]>) =>
    onBackendEvent("tray:navigate", handler),
  wirelessReconnected: (handler: EventCallback<ReconnectEvent>) =>
    onBackendEvent("wireless:reconnected", handler),
  wirelessUnpaired: (handler: EventCallback<ReconnectEvent>) =>
    onBackendEvent("wireless:unpaired", handler),
  traySelectDevice: (handler: EventCallback<BackendEvents["tray:select-device"]>) =>
    onBackendEvent("tray:select-device", handler),
  recordingChanged: (handler: EventCallback<BackendEvents["recording:changed"]>) =>
    onBackendEvent("recording:changed", handler),
  recordingFinished: (handler: EventCallback<BackendEvents["recording:finished"]>) =>
    onBackendEvent("recording:finished", handler),
};
