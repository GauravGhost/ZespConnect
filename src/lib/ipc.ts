import { Channel, invoke } from "@tauri-apps/api/core";

import type {
  AppInfo,
  AppListResult,
  Avd,
  CaptureDirs,
  RecordingOutcome,
  SavedCapture,
  Device,
  DeviceInfo,
  FileEntry,
  KeyAction,
  LauncherSettings,
  LauncherStatus,
  LauncherTarget,
  LogLine,
  MediaKey,
  PerfSample,
  ProcessInfo,
  MirrorAction,
  MdnsDevice,
  ReconnectTarget,
  MirrorKind,
  MirrorOptions,
  MirrorProcess,
  MirrorSession,
  QrPayload,
  QuickState,
  QuickToggleKey,
  RebootMode,
  SendOutcome,
  SerializedAppError,
  ShellResult,
  ToolKind,
  ToolStatus,
  TouchAction,
} from "@/types";

/** The only place that talks to the Rust backend; features import `ipc` and never call `invoke`. */
export const ipc = {
  tools: {
    status: () => invoke<ToolStatus>("get_tools_status"),
    setPath: (tool: ToolKind, path: string | null) =>
      invoke<ToolStatus>("set_tool_path", { tool, path }),
    install: (tool: ToolKind) => invoke<ToolStatus>("install_tool", { tool }),
    checkUpdates: () => invoke<ToolStatus>("check_for_updates"),
  },

  devices: {
    list: () => invoke<Device[]>("list_devices"),
    info: (serial: string) => invoke<DeviceInfo>("get_device_info", { serial }),
    setSelected: (serial: string | null) => invoke<void>("set_selected_device", { serial }),
    selected: () => invoke<string | null>("get_selected_device"),
  },

  apps: {
    list: (serial: string, includeSystem = false, refresh = false) =>
      invoke<AppListResult>("list_apps", { serial, includeSystem, refresh }),
    launch: (serial: string, pkg: string) => invoke<void>("launch_app", { serial, package: pkg }),
    uninstall: (serial: string, pkg: string) =>
      invoke<void>("uninstall_app", { serial, package: pkg }),
    clearData: (serial: string, pkg: string) =>
      invoke<void>("clear_app_data", { serial, package: pkg }),
    installApk: (serial: string, path: string) => invoke<void>("install_apk", { serial, path }),
    /** Stop every process of an app. */
    forceStop: (serial: string, pkg: string) =>
      invoke<void>("force_stop_app", { serial, package: pkg }),
  },

  mirror: {
    start: (serial: string, options: MirrorOptions) =>
      invoke<MirrorProcess>("start_mirror", { serial, options }),
    startCamera: (serial: string, options: MirrorOptions) =>
      invoke<MirrorProcess>("start_camera", { serial, options }),
    startAppWindow: (serial: string, pkg: string, label: string) =>
      invoke<MirrorProcess>("start_app_window", { serial, package: pkg, label }),
    startAppOnScreen: (serial: string, pkg: string, label: string) =>
      invoke<MirrorProcess>("start_app_on_screen", { serial, package: pkg, label }),
    /** Android desktop mode in an external window (no in-app rendering). */
    startDesktop: (serial: string) => invoke<MirrorProcess>("start_desktop", { serial }),
    stop: (pid: number) => invoke<void>("stop_mirror", { pid }),
    stopAll: () => invoke<void>("stop_all_mirrors"),
    list: () => invoke<MirrorProcess[]>("list_mirrors"),

    /** Open a mirror inside a Tauri window the app owns; returns the session id. */
    openWindow: (args: {
      serial: string;
      kind: MirrorKind;
      package: string | null;
      label: string;
      options: MirrorOptions;
    }) => invoke<number>("open_mirror_window", args),
    /** Stream a session's video into the calling window over an IPC channel. */
    startStream: (id: number, channel: Channel<ArrayBuffer>) =>
      invoke<MirrorSession>("start_mirror_stream", { id, channel }),
    /** Fit the session's window to the video's aspect ratio once its first frame arrives. */
    fitWindow: (id: number, width: number, height: number) =>
      invoke<void>("fit_mirror_window", { id, width, height }),
    stopSession: (id: number) => invoke<void>("stop_mirror_session", { id }),
    /** Forward a pointer action; coordinates are in the video's pixel space. */
    touch: (
      id: number,
      action: TouchAction,
      x: number,
      y: number,
      width: number,
      height: number,
      /** Desktop mode: the Android `BUTTON_*` that changed, and the ones held. */
      mouse?: { button: number; buttons: number },
    ) =>
      invoke<void>("mirror_touch", {
        id,
        action,
        x,
        y,
        width,
        height,
        button: mouse?.button ?? null,
        buttons: mouse?.buttons ?? null,
      }),
    /** Scroll at a point; amounts are in wheel notches (positive = up / left). */
    scroll: (
      id: number,
      x: number,
      y: number,
      width: number,
      height: number,
      hscroll: number,
      vscroll: number,
    ) => invoke<void>("mirror_scroll", { id, x, y, width, height, hscroll, vscroll }),
    /** Press or release an Android `KEYCODE_*` with a `META_*` mask. */
    key: (id: number, action: KeyAction, keycode: number, repeat: number, metaState: number) =>
      invoke<void>("mirror_key", { id, action, keycode, repeat, metaState }),
    /** Ask the device for a fresh key frame so a decoder that dropped frames can resync. */
    resetVideo: (id: number) => invoke<void>("mirror_reset_video", { id }),
    /** Desktop mode: turn the phone's own screen on (to use it too) or off. */
    phoneScreen: (id: number, on: boolean) => invoke<void>("mirror_phone_screen", { id, on }),
    /** Type text (at most 300 UTF-8 bytes per call). */
    text: (id: number, text: string) => invoke<void>("mirror_text", { id, text }),
    /** Run a one-shot control (Back, panels, rotate, camera torch/zoom). */
    action: (id: number, action: MirrorAction) => invoke<void>("mirror_action", { id, action }),
    listSessions: () => invoke<MirrorSession[]>("list_mirror_sessions"),
    /** Reopen a session as an external window when in-app rendering is unavailable. */
    fallback: (id: number) => invoke<void>("mirror_session_fallback", { id }),
  },

  controls: {
    quickState: (serial: string) => invoke<QuickState>("get_quick_state", { serial }),
    setToggle: (serial: string, key: QuickToggleKey, on: boolean) =>
      invoke<void>("set_toggle", { serial, key, on }),
    setBrightness: (serial: string, percent: number) =>
      invoke<void>("set_brightness", { serial, percent }),
    setVolume: (serial: string, percent: number) => invoke<void>("set_volume", { serial, percent }),
    reboot: (serial: string, mode: RebootMode) => invoke<void>("reboot", { serial, mode }),
  },

  wireless: {
    qrPayload: () => invoke<QrPayload>("get_qr_payload"),
    qrPair: (payload: string, timeout?: number) =>
      invoke<string>("pair_wireless_qr", { payload, timeout }),
    cancelQr: () => invoke<void>("cancel_wireless_qr"),
    pairCode: (addr: string, code: string) => invoke<string>("pair_wireless_code", { addr, code }),
    connect: (addr: string) => invoke<string>("connect_wireless", { addr }),
    disconnect: (addr: string) => invoke<void>("disconnect_wireless", { addr }),
    enableTcpip: (serial: string, port = 5555) =>
      invoke<string>("enable_wireless_tcpip", { serial, port }),
    mdnsServices: () => invoke<MdnsDevice[]>("list_mdns_services"),
    setReconnectTargets: (targets: ReconnectTarget[]) =>
      invoke<void>("set_reconnect_targets", { targets }),
    reconnect: (target: ReconnectTarget) => invoke<string>("reconnect_wireless", { target }),
  },

  files: {
    listDir: (serial: string, path: string) => invoke<FileEntry[]>("list_dir", { serial, path }),
    pull: (serial: string, remote: string, local: string) =>
      invoke<void>("pull", { serial, remote, local }),
    /** Copy a device file to the computer and open it in the default app for its type. */
    openOnComputer: (serial: string, remote: string) =>
      invoke<void>("open_on_computer", { serial, remote }),
    push: (serial: string, local: string, remote: string) =>
      invoke<void>("push", { serial, local, remote }),
    sendToPhone: (serial: string, path: string) =>
      invoke<SendOutcome>("send_to_phone", { serial, path }),
    mkdir: (serial: string, path: string) => invoke<void>("mkdir", { serial, path }),
    delete: (serial: string, path: string) => invoke<void>("delete", { serial, path }),
  },

  console: {
    exec: (serial: string, command: string) =>
      invoke<ShellResult>("shell_exec", { serial, command }),
    /** Stream `adb logcat` into the given channel until `closeLogcat`. */
    openLogcat: (serial: string, channel: Channel<LogLine>) =>
      invoke<void>("open_logcat", { serial, channel }),
    closeLogcat: (serial: string) => invoke<void>("close_logcat", { serial }),
  },

  monitor: {
    /** Stream the phone's vitals once a second until `closePerformance`. */
    openPerformance: (serial: string, channel: Channel<PerfSample>) =>
      invoke<void>("open_performance", { serial, channel }),
    closePerformance: (serial: string) => invoke<void>("close_performance", { serial }),
    /** Stream the phone's process table every few seconds until `closeProcesses`. */
    openProcesses: (serial: string, channel: Channel<ProcessInfo[]>) =>
      invoke<void>("open_processes", { serial, channel }),
    closeProcesses: (serial: string) => invoke<void>("close_processes", { serial }),
  },

  clipboard: {
    /** Devices whose clipboard syncs with the computer's both ways. */
    syncing: () => invoke<string[]>("list_clipboard_sync"),
    setSync: (serial: string, on: boolean) => invoke<void>("set_clipboard_sync", { serial, on }),
  },

  audio: {
    /** Devices whose sound plays on the computer (the phone's own speaker is silent meanwhile). */
    onComputer: () => invoke<string[]>("list_phone_audio"),
    setOnComputer: (serial: string, onComputer: boolean) =>
      invoke<void>("set_phone_audio", { serial, onComputer }),
  },

  media: {
    /** Report what `serial` plays through `now-playing:changed`; `null` stops watching. */
    watch: (serial: string | null) => invoke<void>("watch_now_playing", { serial }),
    sendKey: (serial: string, key: MediaKey) => invoke<void>("send_media_key", { serial, key }),
  },

  launcher: {
    status: () => invoke<LauncherStatus>("get_launcher_status"),
    /** Fails, keeping the previous hotkey, when the OS refuses the new one. */
    save: (settings: LauncherSettings) =>
      invoke<LauncherStatus>("set_launcher_settings", { settings }),
    target: () => invoke<LauncherTarget>("launcher_target"),
    apps: (serial: string) => invoke<AppInfo[]>("launcher_apps", { serial }),
    hide: () => invoke<void>("hide_launcher"),
  },

  capture: {
    defaultDirs: () => invoke<CaptureDirs>("capture_default_dirs"),
    /** Opens a capture folder, creating it first so a fresh install can open it too. */
    openFolder: (dir: string) => invoke<void>("capture_open_folder", { dir }),
    /** The PNG goes as the raw body; IPC headers are ASCII, so the folder is URI-encoded. */
    saveScreenshot: (
      serial: string,
      png: Uint8Array,
      target: "folder" | "phone",
      dir: string | null,
      stamp: string,
    ) =>
      invoke<SavedCapture>("save_screenshot", png, {
        headers: {
          "x-serial": serial,
          "x-target": target,
          "x-stamp": stamp,
          "x-dir": dir ? encodeURIComponent(dir) : "",
        },
      }),
    /** Resolves with when recording started, in Unix milliseconds. */
    startRecording: (id: number, target: "folder" | "phone", dir: string | null, stamp: string) =>
      invoke<number>("start_recording", { id, target, dir, stamp }),
    /** Resolves once the file is saved (and copied to the phone when that is the target). */
    stopRecording: (id: number) => invoke<RecordingOutcome>("stop_recording", { id }),
    recordingState: (id: number) => invoke<number | null>("recording_state", { id }),
  },

  avd: {
    list: () => invoke<Avd[]>("list_avds"),
    start: (name: string) => invoke<void>("start_avd", { name }),
  },
};

/** Normalise whatever `invoke` rejected with into a readable `{ code, message }`. */
export function toAppError(error: unknown): SerializedAppError {
  if (
    typeof error === "object" &&
    error !== null &&
    "code" in error &&
    "message" in error &&
    typeof (error as SerializedAppError).message === "string"
  ) {
    return error as SerializedAppError;
  }
  return {
    code: "unknown",
    message: typeof error === "string" ? error : "Unexpected error",
  };
}

export function errorMessage(error: unknown): string {
  return toAppError(error).message;
}
