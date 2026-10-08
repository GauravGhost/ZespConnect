/**
 * TS mirrors of the Rust models in `src-tauri/src/models/`.
 * Keep these in sync with the serde structs; the Rust side uses snake_case
 * fields which serde serialises exactly as written here.
 */

export type ToolSource = "override" | "path" | "known" | "not_found";

export type ToolKind = "adb" | "scrcpy";

export interface ToolInfo {
  found: boolean;
  path: string | null;
  version: string | null;
  source: ToolSource;
}

export interface ToolStatus {
  adb: ToolInfo;
  scrcpy: ToolInfo;
  adb_update: UpdateInfo;
  scrcpy_update: UpdateInfo;
}

/** Result of comparing an installed tool against the latest upstream release. */
export interface UpdateInfo {
  latest_version: string | null;
  outdated: boolean;
  error: string | null;
}

export interface InstallProgress {
  tool: ToolKind;
  line: string;
}

export interface InstallDone {
  tool: ToolKind;
  ok: boolean;
  message: string;
}

export type DeviceState = "device" | "unauthorized" | "offline" | "connecting" | "unknown";
export type DeviceTransport = "usb" | "wifi";

export interface Device {
  serial: string;
  display_name: string;
  model: string;
  android_version: string | null;
  sdk: string | null;
  transport: DeviceTransport;
  state: DeviceState;
  connected: boolean;
  battery: number | null;
  charging: boolean | null;
  address: string | null;
}

export interface QuickState {
  wifi: boolean;
  bluetooth: boolean;
  dark_mode: boolean;
  airplane_mode: boolean;
  mobile_data: boolean;
  location: boolean;
  dnd: boolean;
  auto_rotate: boolean;
  brightness: number;
  brightness_max: number;
  volume: number;
}

export interface DeviceInfo {
  model: string;
  android_version: string;
  sdk: string;
  battery: number | null;
  charging: boolean;
  quick: QuickState;
}

export type LabelSource = "scrcpy" | "cache" | "pm" | "derived";

export interface AppInfo {
  package: string;
  label: string;
  is_system: boolean;
  version_name: string | null;
  label_source: LabelSource;
  icon_path: string | null;
}

export interface AppListResult {
  apps: AppInfo[];
  source: "scrcpy" | "cache" | "pm";
  stale: boolean;
}

export type MirrorKind = "mirror" | "camera" | "app" | "screen-app";

/** A pointer action forwarded to a mirrored device. */
export type TouchAction = "down" | "move" | "up";

export interface MirrorOptions {
  resolution: number | null;
  fps: number | null;
  bitrate: number | null;
  audio: boolean;
  borderless: boolean;
  stay_awake: boolean;
  virtual_display: boolean;
  always_on_top: boolean;
  aspect_lock: boolean;
  background: string | null;
  fit: string | null;
  camera_facing: string | null;
  camera_torch: boolean;
  camera_zoom: number | null;
}

export interface MirrorProcess {
  pid: number;
  serial: string;
  kind: MirrorKind;
  package: string | null;
  label: string | null;
  started: number;
}

/** A mirror rendered inside a Tauri window the app owns (no external scrcpy window). */
export interface MirrorSession {
  id: number;
  serial: string;
  kind: MirrorKind;
  package: string | null;
  label: string | null;
  started: number;
}

export type QuickToggleKey =
  "wifi" | "bluetooth" | "darkmode" | "airplane" | "data" | "location" | "dnd" | "rotate";

export type RebootMode = "normal" | "bootloader" | "recovery";
export type RingerMode = "normal" | "vibrate" | "silent";

export interface FileEntry {
  name: string;
  path: string;
  is_directory: boolean;
  size: number;
  formatted_size: string;
  date: string;
  permissions: string;
}

export interface ShellResult {
  stdout: string;
  stderr: string;
  code: number;
}

export interface Avd {
  name: string;
  running: boolean;
}

export interface PreviousDevice {
  id: string;
  name: string;
  wireless: boolean;
  last_seen: number;
}

export interface QrPayload {
  payload: string;
  password: string;
  created: number;
}

/** Serialised form of the Rust `AppError`. */
export interface SerializedAppError {
  code: string;
  message: string;
}
