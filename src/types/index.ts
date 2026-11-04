/** TS mirrors of the Rust models in `src-tauri/src/models/`; keep in sync with the snake_case serde structs. */

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
  /** Stable wireless debugging name announced over mDNS; survives port changes. */
  mdns_name: string | null;
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

export type LabelSource = "scrcpy" | "cache" | "derived";

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

export type MirrorKind = "mirror" | "camera" | "app" | "screen-app" | "desktop";

/** A pointer action forwarded to a mirrored device ("hover": desktop mode only). */
export type TouchAction = "down" | "move" | "up" | "hover";

/** A key press or release forwarded to a mirrored device. */
export type KeyAction = "down" | "up";

/** A one-shot device control from a mirror window's menu (camera ones need scrcpy 4.0+). */
export type MirrorAction =
  | "back"
  | "notifications"
  | "quick-settings"
  | "collapse-panels"
  | "rotate"
  | "torch-on"
  | "torch-off"
  | "zoom-in"
  | "zoom-out";

export interface MirrorOptions {
  resolution: number | null;
  fps: number | null;
  bitrate: number | null;
  audio: boolean;
  borderless: boolean;
  stay_awake: boolean;
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
  /** Desktop mode: whether the phone's own screen is on (the title-bar toggle). */
  phoneScreen: boolean;
  /** The virtual display follows the window's size instead of locking its aspect ratio. */
  flexDisplay: boolean;
  /** A camera session that takes torch and zoom controls. */
  cameraControls: boolean;
  /** Camera: whether the torch started on. */
  torch: boolean;
}

export type QuickToggleKey =
  "wifi" | "bluetooth" | "darkmode" | "airplane" | "data" | "location" | "dnd" | "rotate";

export type RebootMode = "normal" | "bootloader" | "recovery";

export interface FileEntry {
  name: string;
  path: string;
  is_directory: boolean;
  size: number;
  formatted_size: string;
  date: string;
  permissions: string;
}

/** What happened to a file dropped on a window: APKs install, anything else is copied. */
export type SendOutcome = "installed" | "copied";

export interface ShellResult {
  stdout: string;
  stderr: string;
  code: number;
}

export type LogStream = "stdout" | "stderr";

/** One line streamed from `adb logcat` into the in-app viewer. */
export interface LogLine {
  stream: LogStream;
  text: string;
}

export interface Avd {
  name: string;
  running: boolean;
}

/** A service instance discovered by `adb mdns services`. */
export interface MdnsDevice {
  name: string;
  service_type: string;
  host: string;
  port: number;
  address: string;
}

/** A saved phone the backend reconnects when it reappears on the network. */
export interface ReconnectTarget {
  serial: string;
  name: string | null;
  address: string | null;
}

export interface ReconnectEvent {
  serial: string;
  address: string | null;
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
