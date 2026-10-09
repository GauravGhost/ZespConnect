import { Channel, invoke } from "@tauri-apps/api/core";

import type {
  AppListResult,
  Avd,
  Device,
  DeviceInfo,
  FileEntry,
  KeyAction,
  LogLine,
  MdnsDevice,
  MirrorKind,
  MirrorOptions,
  MirrorProcess,
  MirrorSession,
  QrPayload,
  QuickState,
  QuickToggleKey,
  RebootMode,
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
    clearPath: (tool: ToolKind) => invoke<ToolStatus>("set_tool_path", { tool, path: null }),
    install: (tool: ToolKind) => invoke<ToolStatus>("install_tool", { tool }),
    checkUpdates: () => invoke<ToolStatus>("check_for_updates"),
    scrcpyVersion: () => invoke<string | null>("get_scrcpy_version"),
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
  },

  files: {
    listDir: (serial: string, path: string) => invoke<FileEntry[]>("list_dir", { serial, path }),
    pull: (serial: string, remote: string, local: string) =>
      invoke<void>("pull", { serial, remote, local }),
    push: (serial: string, local: string, remote: string) =>
      invoke<void>("push", { serial, local, remote }),
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

  clipboard: {
    send: (serial: string, text: string) => invoke<void>("clipboard_send", { serial, text }),
    read: (serial: string) => invoke<string>("clipboard_read", { serial }),
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
