import type { ComboboxOption } from "@/components/ui/combobox-input";
import type { MirrorKind, MirrorOptions } from "@/types";

/** Human labels for each mirror kind, shown in window lists. */
export const KIND_LABEL: Record<MirrorKind, string> = {
  mirror: "Screen",
  camera: "Camera",
  app: "App window",
  "screen-app": "App on screen",
  desktop: "Desktop mode",
};

/** The frame rate and bitrate shown when a mirror leaves them unset. */
export const DEFAULT_FPS = 60;
export const DEFAULT_BITRATE_MBPS = 8;

/** Baseline mirror settings used when a screen has no options form of its own. */
export const DEFAULT_MIRROR_OPTIONS: MirrorOptions = {
  resolution: null,
  fps: null,
  bitrate: DEFAULT_BITRATE_MBPS,
  audio: true,
  borderless: false,
  stay_awake: true,
  always_on_top: false,
  aspect_lock: true,
  background: null,
  fit: null,
  camera_facing: "back",
  camera_torch: false,
  camera_zoom: null,
};

export const presetOptions = (values: number[]): ComboboxOption[] =>
  values.map((value) => ({ value: String(value), label: String(value) }));

export const RESOLUTION_PRESETS: ComboboxOption[] = [
  { value: "", label: "Native" },
  ...presetOptions([720, 1080, 1440, 2160]),
];
export const FPS_PRESETS = presetOptions([30, 60, 90, 120]);
export const BITRATE_PRESETS = presetOptions([2, 4, 8, 16, 24]);
