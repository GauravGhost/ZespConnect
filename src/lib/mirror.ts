import type { MirrorKind, MirrorOptions } from "@/types";

/** Human labels for each mirror kind, shown in window lists. */
export const KIND_LABEL: Record<MirrorKind, string> = {
  mirror: "Screen",
  camera: "Camera",
  app: "App window",
  "screen-app": "App on screen",
  desktop: "Desktop mode",
};

/** Baseline mirror settings used when a screen has no options form of its own. */
export const DEFAULT_MIRROR_OPTIONS: MirrorOptions = {
  resolution: null,
  fps: null,
  bitrate: 8,
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
