import { invoke } from "@tauri-apps/api/core";

import type {
  AppListResult,
  Avd,
  Device,
  DeviceInfo,
  FileEntry,
  MirrorOptions,
  MirrorProcess,
  QrPayload,
  QuickState,
  QuickToggleKey,
  RebootMode,
  SerializedAppError,
  ShellResult,
  ToolStatus,
} from "@/types";

/**
 * The ONLY place that talks to the Rust backend. Features import `ipc` and
 * never call `invoke` directly, so the command surface stays in one file.
 */
export const ipc = {
  tools: {
    status: () => invoke<ToolStatus>("get_tools_status"),
    setPath: (tool: "adb" | "scrcpy", path: string | null) =>
      invoke<ToolStatus>("set_tool_path", { tool, path }),
    scrcpyVersion: () => invoke<string | null>("get_scrcpy_version"),
  },

  devices: {
    list: () => invoke<Device[]>("list_devices"),
    info: (serial: string) => invoke<DeviceInfo>("get_device_info", { serial }),
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
    stop: (pid: number) => invoke<void>("stop_mirror", { pid }),
    stopAll: () => invoke<void>("stop_all_mirrors"),
    list: () => invoke<MirrorProcess[]>("list_mirrors"),
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
    pair: (addr: string, code: string) => invoke<void>("pair", { addr, code }),
    connect: (addr: string) => invoke<void>("connect", { addr }),
    disconnect: (addr: string) => invoke<void>("disconnect", { addr }),
    enableTcpip: (serial: string, port = 5555) => invoke<void>("enable_tcpip", { serial, port }),
    qrPayload: () => invoke<QrPayload>("get_qr_payload"),
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
    openLogcat: (serial: string) => invoke<void>("open_logcat", { serial }),
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
