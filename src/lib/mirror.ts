import type { MirrorOptions } from "@/types";

/** Baseline mirror settings used when a screen has no options form of its own. */
export const DEFAULT_MIRROR_OPTIONS: MirrorOptions = {
  resolution: null,
  fps: null,
  bitrate: 8,
  audio: true,
  borderless: false,
  stay_awake: true,
  virtual_display: false,
  always_on_top: false,
  aspect_lock: true,
  background: null,
  fit: null,
  camera_facing: "back",
  camera_torch: false,
  camera_zoom: null,
};
